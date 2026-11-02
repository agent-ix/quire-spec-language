// SPDX-License-Identifier: AGPL-3.0-or-later
//! The expansion of a `HybridTerm` into the expression both replays
//! evaluate: every `Call` of a function that is not recursive is replaced by
//! its checked body over the argument expressions (FR-450 "Exact replay":
//! "after expanding every `Call` whose function is not recursive"), each
//! argument wrapped by its parameter's value range and the body by the
//! result type's (QSpec FR-193 "Continuous values and expressions": an
//! argument or result outside its range has no value). A call of a
//! recursive function stays a call, which static replay evaluates over
//! intervals and exact replay refuses as `NotExact`.

use quire_checked::node::{Arithmetic, Connective, NodeKind};
use std::collections::BTreeMap;

use qsl_semantics::check::{
    ArithmeticOp, CheckedFunctionRef, CheckedGraph, CheckedNode, CompareOp, HybridTerm, LogicOp,
};
use quire_exact::{ArithmeticOperator, Integer, OrderingOperator, Rational, Value, ValueType};
use quire_semantic_value::declaration::EqualityOperator;

use super::{HybridReplayMeter, HybridReplayRefusal};

/// Where, inside one expression of the model or claim, an evaluation is:
/// the path of child positions from the expression's root `HybridTerm`
/// (`Negate`, `Not`: 0; `Arithmetic`, `Compare`, `Logic`: 0 and 1;
/// `Conditional`: 0 to 2; `Call`: the argument's position), and, inside a
/// called function's checked body, each called function and the node's
/// index in its body, outermost first.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TermLocus {
    /// The path from the expression's root to the outermost `Call`, or to
    /// the node itself when it is outside every call.
    pub term: Vec<usize>,
    /// Each called function entered and the node's index in its checked
    /// body, outermost first.
    pub body: Vec<(CheckedFunctionRef, usize)>,
}

impl TermLocus {
    fn child(&self, position: usize) -> Self {
        let mut child = self.clone();
        if child.body.is_empty() {
            child.term.push(position);
        }
        child
    }

    fn in_body(&self, function: CheckedFunctionRef, node: usize) -> Self {
        let mut inner = self.clone();
        inner.body.push((function, node));
        inner
    }

    fn at_node(&self, node: usize) -> Self {
        let mut at = self.clone();
        if let Some(last) = at.body.last_mut() {
            last.1 = node;
        }
        at
    }
}

/// The value range of a numeric type, either end absent when unbounded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ValueRange {
    pub(crate) lower: Option<Rational>,
    pub(crate) upper: Option<Rational>,
}

impl ValueRange {
    fn unbounded() -> Self {
        Self {
            lower: None,
            upper: None,
        }
    }

    fn is_unbounded(&self) -> bool {
        self.lower.is_none() && self.upper.is_none()
    }
}

/// Whose range a [`Exp::Ranged`] checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RangeRole {
    /// A call's argument against its parameter's type.
    Parameter {
        function: CheckedFunctionRef,
        parameter: usize,
    },
    /// A call's result against its result type.
    Result { function: CheckedFunctionRef },
}

/// What a value of a type ranges over: a Boolean, or a number in a range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ResultKind {
    Boolean,
    Number(ValueRange),
}

/// A call of a recursive function, kept unexpanded.
#[derive(Clone, Debug)]
pub(crate) struct RecursiveCall {
    pub(crate) function: CheckedFunctionRef,
    pub(crate) arguments: Vec<Exp>,
    /// Whether each parameter is `Integer` or `Int`: the parameters a
    /// `decreases` measure (FR-146) can name, since a hybrid term passes no
    /// collection or composite value.
    pub(crate) measured: Vec<bool>,
    pub(crate) result: ResultKind,
    pub(crate) locus: TermLocus,
}

/// An expanded expression.
#[derive(Clone, Debug)]
pub(crate) enum Exp {
    Variable(usize),
    Activation(usize),
    Capture(usize),
    /// A parameter of a recursive function's body, bound by its evaluator.
    Parameter(usize),
    Number(Rational),
    Boolean(bool),
    Negate(Box<Exp>),
    Arithmetic(ArithmeticOp, Box<Exp>, Box<Exp>, TermLocus),
    Compare(CompareOp, Box<Exp>, Box<Exp>),
    Not(Box<Exp>),
    Logic(LogicOp, Box<Exp>, Box<Exp>),
    Conditional(Box<Exp>, Box<Exp>, Box<Exp>),
    Ranged {
        value: Box<Exp>,
        range: ValueRange,
        role: RangeRole,
        locus: TermLocus,
    },
    Recursive(RecursiveCall),
}

impl Exp {
    /// Whether a recursive call occurs anywhere in `self`.
    pub(crate) fn calls_recursive(&self) -> bool {
        self.any_node(|node| matches!(node, Self::Recursive(_)))
    }

    /// Whether `self` reads a capture or an activation variable.
    pub(crate) fn reads_activation(&self) -> bool {
        self.any_node(|node| matches!(node, Self::Activation(_) | Self::Capture(_)))
    }

    fn any_node(&self, predicate: impl Fn(&Self) -> bool) -> bool {
        let mut pending = vec![self];
        while let Some(node) = pending.pop() {
            if predicate(node) {
                return true;
            }
            match node {
                Self::Negate(a) | Self::Not(a) | Self::Ranged { value: a, .. } => pending.push(a),
                Self::Arithmetic(_, a, b, _) | Self::Compare(_, a, b) | Self::Logic(_, a, b) => {
                    pending.extend([a.as_ref(), b.as_ref()])
                }
                Self::Conditional(a, b, c) => {
                    pending.extend([a.as_ref(), b.as_ref(), c.as_ref()]);
                }
                Self::Recursive(call) => pending.extend(call.arguments.iter()),
                Self::Variable(_)
                | Self::Activation(_)
                | Self::Capture(_)
                | Self::Parameter(_)
                | Self::Number(_)
                | Self::Boolean(_) => {}
            }
        }
        false
    }

    /// `self` with every activation variable read as the current variable.
    pub(crate) fn activation_as_current(&self) -> Self {
        let map = |e: &Self| Box::new(e.activation_as_current());
        match self {
            Self::Activation(i) => Self::Variable(*i),
            Self::Variable(_)
            | Self::Capture(_)
            | Self::Parameter(_)
            | Self::Number(_)
            | Self::Boolean(_) => self.clone(),
            Self::Negate(a) => Self::Negate(map(a)),
            Self::Not(a) => Self::Not(map(a)),
            Self::Ranged {
                value,
                range,
                role,
                locus,
            } => Self::Ranged {
                value: map(value),
                range: range.clone(),
                role: *role,
                locus: locus.clone(),
            },
            Self::Arithmetic(op, a, b, locus) => {
                Self::Arithmetic(*op, map(a), map(b), locus.clone())
            }
            Self::Compare(op, a, b) => Self::Compare(*op, map(a), map(b)),
            Self::Logic(op, a, b) => Self::Logic(*op, map(a), map(b)),
            Self::Conditional(a, b, c) => Self::Conditional(map(a), map(b), map(c)),
            Self::Recursive(call) => Self::Recursive(RecursiveCall {
                arguments: call
                    .arguments
                    .iter()
                    .map(Self::activation_as_current)
                    .collect(),
                ..call.clone()
            }),
        }
    }
}

/// Why a term cannot be expanded from the checked package.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ExpansionGap {
    /// The package holds no callable function at this index.
    #[error("the package holds no callable function {0:?}")]
    UnreachableFunction(CheckedFunctionRef),
    /// A called function's checked body holds a node outside the hybrid
    /// expression forms.
    #[error(
        "function {function:?} holds a form outside the hybrid expression forms at node {node}"
    )]
    UnsupportedBodyForm {
        /// The function.
        function: CheckedFunctionRef,
        /// The node's index in its checked body.
        node: usize,
    },
}

struct Frame {
    slots: Vec<Option<Exp>>,
}

enum Build {
    Negate,
    Not,
    Arithmetic(ArithmeticOp, TermLocus),
    Compare(CompareOp),
    Logic(LogicOp),
    Conditional,
}

/// The explicit work stack for expanding source and checked-body terms.
enum Task<'t, 'g> {
    Term {
        term: &'t HybridTerm,
        locus: TermLocus,
    },
    Node {
        function: CheckedFunctionRef,
        node: CheckedNode<'g>,
        frame: usize,
        locus: TermLocus,
    },
    Build(Build),
    Call {
        function: CheckedFunctionRef,
        count: usize,
        locus: TermLocus,
    },
    WrapRange {
        range: ValueRange,
        function: CheckedFunctionRef,
        locus: TermLocus,
    },
    BindLet {
        function: CheckedFunctionRef,
        body: CheckedNode<'g>,
        frame: usize,
        slot: usize,
        locus: TermLocus,
    },
    RestoreLet {
        frame: usize,
        slot: usize,
        saved: Box<Option<Exp>>,
    },
    FinishBody,
}

/// Expands terms against one checked package, remembering which functions
/// are recursive.
pub(crate) struct Expander<'g> {
    graph: &'g CheckedGraph,
    recursive: BTreeMap<usize, bool>,
}

impl<'g> Expander<'g> {
    pub(crate) fn new(graph: &'g CheckedGraph) -> Self {
        Self {
            graph,
            recursive: BTreeMap::new(),
        }
    }

    fn charge(meter: &mut HybridReplayMeter) -> Result<(), HybridReplayRefusal> {
        meter.charge()
    }

    fn expand<'t>(
        &mut self,
        initial: Task<'t, 'g>,
        meter: &mut HybridReplayMeter,
        mut frames: Vec<Frame>,
    ) -> Result<Exp, HybridReplayRefusal> {
        let mut pending = vec![initial];
        let mut values = Vec::new();
        while let Some(task) = pending.pop() {
            match task {
                Task::Term { term, locus } => {
                    Self::charge(meter)?;
                    match term {
                        HybridTerm::Variable(index) => values.push(Exp::Variable(index.get())),
                        HybridTerm::ActivationVariable(index) => {
                            values.push(Exp::Activation(index.get()));
                        }
                        HybridTerm::Capture(index) => values.push(Exp::Capture(index.get())),
                        HybridTerm::Constant(value) => values.push(Exp::Number(value.clone())),
                        HybridTerm::Boolean(value) => values.push(Exp::Boolean(*value)),
                        HybridTerm::Negate(child) | HybridTerm::Not(child) => {
                            pending.push(Task::Build(if matches!(term, HybridTerm::Negate(_)) {
                                Build::Negate
                            } else {
                                Build::Not
                            }));
                            pending.push(Task::Term {
                                term: child,
                                locus: locus.child(0),
                            });
                        }
                        HybridTerm::Arithmetic(op, left, right) => {
                            pending.push(Task::Build(Build::Arithmetic(*op, locus.clone())));
                            for (position, child) in
                                [(0, left.as_ref()), (1, right.as_ref())].into_iter().rev()
                            {
                                pending.push(Task::Term {
                                    term: child,
                                    locus: locus.child(position),
                                });
                            }
                        }
                        HybridTerm::Compare(op, left, right) => {
                            pending.push(Task::Build(Build::Compare(*op)));
                            for (position, child) in
                                [(0, left.as_ref()), (1, right.as_ref())].into_iter().rev()
                            {
                                pending.push(Task::Term {
                                    term: child,
                                    locus: locus.child(position),
                                });
                            }
                        }
                        HybridTerm::Logic(op, left, right) => {
                            pending.push(Task::Build(Build::Logic(*op)));
                            for (position, child) in
                                [(0, left.as_ref()), (1, right.as_ref())].into_iter().rev()
                            {
                                pending.push(Task::Term {
                                    term: child,
                                    locus: locus.child(position),
                                });
                            }
                        }
                        HybridTerm::Conditional(condition, yes, no) => {
                            pending.push(Task::Build(Build::Conditional));
                            for (position, child) in
                                [(0, condition.as_ref()), (1, yes.as_ref()), (2, no.as_ref())]
                                    .into_iter()
                                    .rev()
                            {
                                pending.push(Task::Term {
                                    term: child,
                                    locus: locus.child(position),
                                });
                            }
                        }
                        HybridTerm::Call(function, arguments) => {
                            pending.push(Task::Call {
                                function: *function,
                                count: arguments.len(),
                                locus: locus.clone(),
                            });
                            for (position, argument) in arguments.iter().enumerate().rev() {
                                pending.push(Task::Term {
                                    term: argument,
                                    locus: locus.child(position),
                                });
                            }
                        }
                    }
                }
                Task::Node {
                    function,
                    node,
                    frame,
                    locus,
                } => {
                    Self::charge(meter)?;
                    let locus = locus.at_node(node.id().index());
                    let unsupported = ExpansionGap::UnsupportedBodyForm {
                        function,
                        node: node.id().index(),
                    };
                    match node.kind() {
                        NodeKind::Literal(literal) => values.push(match &literal.0 {
                            Value::Boolean(value) => Exp::Boolean(*value),
                            Value::Integer(value) => {
                                Exp::Number(Rational::from_integer(value.clone()))
                            }
                            Value::Rational(value) => Exp::Number(value.clone()),
                            Value::Decimal(value) => Exp::Number(value.normalized().to_rational()),
                            _ => return Err(unsupported.into()),
                        }),
                        NodeKind::Local(slot) => {
                            let value = frames
                                .get(frame)
                                .and_then(|frame| frame.slots.get(*slot))
                                .and_then(Option::as_ref)
                                .ok_or(unsupported)?;
                            values.push(value.clone());
                        }
                        NodeKind::Let { slot, value, body } => {
                            pending.push(Task::BindLet {
                                function,
                                body: node.at(*body),
                                frame,
                                slot: *slot,
                                locus: locus.clone(),
                            });
                            pending.push(Task::Node {
                                function,
                                node: node.at(*value),
                                frame,
                                locus,
                            });
                        }
                        kind => {
                            let build = match kind {
                                NodeKind::If { .. } => Some(Build::Conditional),
                                NodeKind::Arithmetic(op, _, _) => Some(Build::Arithmetic(
                                    match op {
                                        Arithmetic::Add => ArithmeticOp::Add,
                                        Arithmetic::Subtract => ArithmeticOp::Sub,
                                        Arithmetic::Multiply => ArithmeticOp::Mul,
                                    },
                                    locus.clone(),
                                )),
                                NodeKind::Rational { operator, .. }
                                | NodeKind::Decimal { operator, .. } => Some(Build::Arithmetic(
                                    match operator {
                                        ArithmeticOperator::Add => ArithmeticOp::Add,
                                        ArithmeticOperator::Subtract => ArithmeticOp::Sub,
                                        ArithmeticOperator::Multiply => ArithmeticOp::Mul,
                                        ArithmeticOperator::Divide => ArithmeticOp::Div,
                                    },
                                    locus.clone(),
                                )),
                                NodeKind::Divide { .. } => {
                                    Some(Build::Arithmetic(ArithmeticOp::Div, locus.clone()))
                                }
                                NodeKind::Negate(_)
                                | NodeKind::RationalNegate(_, _)
                                | NodeKind::DecimalNegate(_, _) => Some(Build::Negate),
                                NodeKind::Order(op, _, _, _) => Some(Build::Compare(match op {
                                    OrderingOperator::Less => CompareOp::Lt,
                                    OrderingOperator::LessOrEqual => CompareOp::Le,
                                    OrderingOperator::Greater => CompareOp::Gt,
                                    OrderingOperator::GreaterOrEqual => CompareOp::Ge,
                                })),
                                NodeKind::Equality(op, _, _, _) => Some(Build::Compare(match op {
                                    EqualityOperator::Equal => CompareOp::Eq,
                                    EqualityOperator::NotEqual => CompareOp::Ne,
                                })),
                                NodeKind::Connective(op, _, _) => Some(Build::Logic(match op {
                                    Connective::And => LogicOp::And,
                                    Connective::Or => LogicOp::Or,
                                    Connective::Implies => LogicOp::Implies,
                                })),
                                NodeKind::Not(_) => Some(Build::Not),
                                NodeKind::Coerce(_, _)
                                | NodeKind::ConvertScalar(_, _)
                                | NodeKind::ConvertDecimal(_, _) => None,
                                NodeKind::Call {
                                    function: callee,
                                    arguments,
                                } => {
                                    pending.push(Task::Call {
                                        function: CheckedFunctionRef::new(*callee),
                                        count: arguments.len(),
                                        locus: locus.clone(),
                                    });
                                    None
                                }
                                _ => return Err(unsupported.into()),
                            };
                            if let Some(build) = build {
                                pending.push(Task::Build(build));
                            }
                            for child in node.children().into_iter().rev() {
                                pending.push(Task::Node {
                                    function,
                                    node: child,
                                    frame,
                                    locus: locus.clone(),
                                });
                            }
                        }
                    }
                }
                Task::Build(build) => {
                    let value = match build {
                        Build::Negate => Exp::Negate(Box::new(values.pop().expect("operand"))),
                        Build::Not => Exp::Not(Box::new(values.pop().expect("operand"))),
                        Build::Arithmetic(op, locus) => {
                            let right = values.pop().expect("right operand");
                            let left = values.pop().expect("left operand");
                            Exp::Arithmetic(op, Box::new(left), Box::new(right), locus)
                        }
                        Build::Compare(op) => {
                            let right = values.pop().expect("right operand");
                            let left = values.pop().expect("left operand");
                            Exp::Compare(op, Box::new(left), Box::new(right))
                        }
                        Build::Logic(op) => {
                            let right = values.pop().expect("right operand");
                            let left = values.pop().expect("left operand");
                            Exp::Logic(op, Box::new(left), Box::new(right))
                        }
                        Build::Conditional => {
                            let no = values.pop().expect("otherwise branch");
                            let yes = values.pop().expect("then branch");
                            let condition = values.pop().expect("condition");
                            Exp::Conditional(Box::new(condition), Box::new(yes), Box::new(no))
                        }
                    };
                    values.push(value);
                }
                Task::Call {
                    function,
                    count,
                    locus,
                } => {
                    let start = values.len().checked_sub(count).expect("call arguments");
                    let arguments = values.split_off(start);
                    let unreachable = ExpansionGap::UnreachableFunction(function);
                    let state = self
                        .graph
                        .function_state(function.get())
                        .ok_or_else(|| unreachable.clone())?;
                    let signature = self
                        .graph
                        .callable(state.name)
                        .ok_or_else(|| unreachable.clone())?;
                    if signature.parameters.len() != arguments.len() {
                        return Err(unreachable.into());
                    }
                    let arguments = arguments
                        .into_iter()
                        .zip(signature.parameters)
                        .enumerate()
                        .map(|(parameter, (argument, (_, ty)))| match result_kind(ty) {
                            Some(ResultKind::Number(range)) if !range.is_unbounded() => {
                                Exp::Ranged {
                                    value: Box::new(argument),
                                    range,
                                    role: RangeRole::Parameter {
                                        function,
                                        parameter,
                                    },
                                    locus: locus.clone(),
                                }
                            }
                            _ => argument,
                        })
                        .collect::<Vec<_>>();
                    let result =
                        result_kind(signature.result).ok_or(ExpansionGap::UnsupportedBodyForm {
                            function,
                            node: state.body.id().index(),
                        })?;
                    if self.is_recursive(function.get()) {
                        let measured = signature
                            .parameters
                            .iter()
                            .map(|(_, ty)| matches!(ty, ValueType::Integer | ValueType::Int(_)))
                            .collect();
                        values.push(Exp::Recursive(RecursiveCall {
                            function,
                            arguments,
                            measured,
                            result,
                            locus,
                        }));
                        continue;
                    }
                    let slots = state.slots.max(arguments.len());
                    let mut frame_slots = vec![None; slots];
                    for (slot, argument) in frame_slots.iter_mut().zip(arguments) {
                        *slot = Some(argument);
                    }
                    let frame = frames.len();
                    frames.push(Frame { slots: frame_slots });
                    if let ResultKind::Number(range) = result {
                        if !range.is_unbounded() {
                            pending.push(Task::WrapRange {
                                range,
                                function,
                                locus: locus.clone(),
                            });
                        }
                    }
                    pending.push(Task::FinishBody);
                    pending.push(Task::Node {
                        function,
                        node: state.body,
                        frame,
                        locus: locus.in_body(function, state.body.id().index()),
                    });
                }
                Task::WrapRange {
                    range,
                    function,
                    locus,
                } => {
                    let value = values.pop().expect("function body");
                    values.push(Exp::Ranged {
                        value: Box::new(value),
                        range,
                        role: RangeRole::Result { function },
                        locus,
                    });
                }
                Task::BindLet {
                    function,
                    body,
                    frame,
                    slot,
                    locus,
                } => {
                    let bound = values.pop().expect("let value");
                    let entry = frames
                        .get_mut(frame)
                        .and_then(|frame| frame.slots.get_mut(slot))
                        .ok_or(ExpansionGap::UnsupportedBodyForm {
                            function,
                            node: body.id().index(),
                        })?;
                    let saved = entry.replace(bound);
                    pending.push(Task::RestoreLet {
                        frame,
                        slot,
                        saved: Box::new(saved),
                    });
                    pending.push(Task::Node {
                        function,
                        node: body,
                        frame,
                        locus,
                    });
                }
                Task::RestoreLet { frame, slot, saved } => {
                    let entry = frames
                        .get_mut(frame)
                        .and_then(|frame| frame.slots.get_mut(slot))
                        .expect("let slot exists until body finishes");
                    *entry = *saved;
                }
                Task::FinishBody => {
                    frames
                        .pop()
                        .expect("function frame exists until body finishes");
                }
            }
        }
        Ok(values.pop().expect("expansion produced a value"))
    }

    /// The functions `function`'s body calls.
    fn callees(&self, function: usize) -> Vec<usize> {
        let Some(state) = self.graph.function_state(function) else {
            return Vec::new();
        };
        let mut callees = Vec::new();
        let mut pending = vec![state.body];
        while let Some(node) = pending.pop() {
            if let NodeKind::Call { function, .. } = node.kind() {
                callees.push(*function);
            }
            pending.extend(node.children());
        }
        callees
    }

    /// Whether `function` reaches itself through the calls of the bodies.
    pub(crate) fn is_recursive(&mut self, function: usize) -> bool {
        if let Some(known) = self.recursive.get(&function) {
            return *known;
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut pending = self.callees(function);
        let mut recursive = false;
        while let Some(next) = pending.pop() {
            if next == function {
                recursive = true;
                break;
            }
            if seen.insert(next) {
                pending.extend(self.callees(next));
            }
        }
        self.recursive.insert(function, recursive);
        recursive
    }

    /// Expand a source term using caller-supplied work fuel.
    pub(crate) fn term(
        &mut self,
        term: &HybridTerm,
        meter: &mut HybridReplayMeter,
    ) -> Result<Exp, HybridReplayRefusal> {
        self.expand(
            Task::Term {
                term,
                locus: TermLocus::default(),
            },
            meter,
            Vec::new(),
        )
    }

    /// Expand a checked function body over its arguments.
    pub(crate) fn body(
        &mut self,
        function: CheckedFunctionRef,
        arguments: Vec<Exp>,
        locus: &TermLocus,
        meter: &mut HybridReplayMeter,
    ) -> Result<Exp, HybridReplayRefusal> {
        let state = self
            .graph
            .function_state(function.get())
            .ok_or(ExpansionGap::UnreachableFunction(function))?;
        let mut slots = vec![None; state.slots.max(arguments.len())];
        for (slot, argument) in slots.iter_mut().zip(arguments) {
            *slot = Some(argument);
        }
        self.expand(
            Task::Node {
                function,
                node: state.body,
                frame: 0,
                locus: locus.in_body(function, state.body.id().index()),
            },
            meter,
            vec![Frame { slots }],
        )
    }
}

/// What a value of `ty` ranges over; `None` for a type that is neither
/// Boolean nor numeric. A numeric type's value range is the interval between
/// its least and greatest members; its precision components do not apply
/// (QSpec FR-193 "Continuous values and expressions").
pub(crate) fn result_kind(ty: &ValueType) -> Option<ResultKind> {
    let ratio = |numerator: &Integer, denominator: &Integer| {
        Rational::new(numerator.clone(), denominator.clone()).ok()
    };
    let range = match ty {
        ValueType::Boolean => return Some(ResultKind::Boolean),
        ValueType::Integer | ValueType::Float(_) => ValueRange::unbounded(),
        ValueType::Int(interval) => ValueRange {
            lower: Some(Rational::from_integer(interval.lower().clone())),
            upper: Some(Rational::from_integer(interval.upper().clone())),
        },
        ValueType::Rational(domain) => {
            let (numerator, denominator) = (domain.numerator(), domain.denominator());
            let lower = if numerator.lower().is_negative() {
                denominator.lower()
            } else {
                denominator.upper()
            };
            let upper = if numerator.upper().is_negative() || numerator.upper().is_zero() {
                denominator.upper()
            } else {
                denominator.lower()
            };
            ValueRange {
                lower: ratio(numerator.lower(), lower),
                upper: ratio(numerator.upper(), upper),
            }
        }
        ValueType::Decimal(decimal) => {
            let scaled = |coefficient: &Integer, scale: u32| {
                Rational::from_integer(coefficient.clone())
                    .divided_by_power_of_ten(u64::from(scale))
            };
            let lower_scale = if decimal.lower().is_negative() {
                decimal.min_scale()
            } else {
                decimal.max_scale()
            };
            let upper_scale = if decimal.upper().is_negative() || decimal.upper().is_zero() {
                decimal.max_scale()
            } else {
                decimal.min_scale()
            };
            ValueRange {
                lower: Some(scaled(decimal.lower(), lower_scale)),
                upper: Some(scaled(decimal.upper(), upper_scale)),
            }
        }
        _ => return None,
    };
    Some(ResultKind::Number(range))
}
