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

use std::collections::BTreeMap;

use qsl_semantics::check::{
    Arithmetic, ArithmeticOp, CheckedFunctionRef, CheckedGraph, CheckedNode, CompareOp, Connective,
    HybridTerm, LogicOp, NodeKind,
};
use quire_exact::{ArithmeticOperator, Integer, OrderingOperator, Rational, Value, ValueType};
use quire_semantic_value::declaration::EqualityOperator;

/// The expression nodes an expansion may build before it stops, so a body
/// whose `let` substitutions multiply stays bounded.
const EXPANSION_NODES: usize = 1 << 20;

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
        match self {
            Self::Recursive(_) => true,
            Self::Variable(_)
            | Self::Activation(_)
            | Self::Capture(_)
            | Self::Parameter(_)
            | Self::Number(_)
            | Self::Boolean(_) => false,
            Self::Negate(a) | Self::Not(a) | Self::Ranged { value: a, .. } => a.calls_recursive(),
            Self::Arithmetic(_, a, b, _) | Self::Compare(_, a, b) | Self::Logic(_, a, b) => {
                a.calls_recursive() || b.calls_recursive()
            }
            Self::Conditional(a, b, c) => {
                a.calls_recursive() || b.calls_recursive() || c.calls_recursive()
            }
        }
    }

    /// Whether `self` reads a capture or an activation variable.
    pub(crate) fn reads_activation(&self) -> bool {
        match self {
            Self::Activation(_) | Self::Capture(_) => true,
            Self::Recursive(call) => call.arguments.iter().any(Self::reads_activation),
            Self::Variable(_) | Self::Parameter(_) | Self::Number(_) | Self::Boolean(_) => false,
            Self::Negate(a) | Self::Not(a) | Self::Ranged { value: a, .. } => a.reads_activation(),
            Self::Arithmetic(_, a, b, _) | Self::Compare(_, a, b) | Self::Logic(_, a, b) => {
                a.reads_activation() || b.reads_activation()
            }
            Self::Conditional(a, b, c) => {
                a.reads_activation() || b.reads_activation() || c.reads_activation()
            }
        }
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
    /// The expansion passed its node bound.
    #[error("the expansion of a term passed {EXPANSION_NODES} nodes")]
    TooLarge,
}

/// Expands terms against one checked package, remembering which functions
/// are recursive.
pub(crate) struct Expander<'g> {
    graph: &'g CheckedGraph,
    recursive: BTreeMap<usize, bool>,
    nodes: usize,
}

impl<'g> Expander<'g> {
    pub(crate) fn new(graph: &'g CheckedGraph) -> Self {
        Self {
            graph,
            recursive: BTreeMap::new(),
            nodes: 0,
        }
    }

    fn charge(&mut self) -> Result<(), ExpansionGap> {
        self.nodes += 1;
        if self.nodes > EXPANSION_NODES {
            return Err(ExpansionGap::TooLarge);
        }
        Ok(())
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

    /// `term` expanded, its loci rooted at `locus`.
    pub(crate) fn term(&mut self, term: &HybridTerm) -> Result<Exp, ExpansionGap> {
        self.term_at(term, &TermLocus::default())
    }

    fn term_at(&mut self, term: &HybridTerm, locus: &TermLocus) -> Result<Exp, ExpansionGap> {
        self.charge()?;
        let sub = |this: &mut Self, position: usize, child: &HybridTerm| {
            this.term_at(child, &locus.child(position)).map(Box::new)
        };
        Ok(match term {
            HybridTerm::Variable(index) => Exp::Variable(index.get()),
            HybridTerm::ActivationVariable(index) => Exp::Activation(index.get()),
            HybridTerm::Capture(index) => Exp::Capture(index.get()),
            HybridTerm::Constant(value) => Exp::Number(value.clone()),
            HybridTerm::Boolean(value) => Exp::Boolean(*value),
            HybridTerm::Negate(a) => Exp::Negate(sub(self, 0, a)?),
            HybridTerm::Not(a) => Exp::Not(sub(self, 0, a)?),
            HybridTerm::Arithmetic(op, a, b) => {
                Exp::Arithmetic(*op, sub(self, 0, a)?, sub(self, 1, b)?, locus.clone())
            }
            HybridTerm::Compare(op, a, b) => Exp::Compare(*op, sub(self, 0, a)?, sub(self, 1, b)?),
            HybridTerm::Logic(op, a, b) => Exp::Logic(*op, sub(self, 0, a)?, sub(self, 1, b)?),
            HybridTerm::Conditional(c, a, b) => {
                Exp::Conditional(sub(self, 0, c)?, sub(self, 1, a)?, sub(self, 2, b)?)
            }
            HybridTerm::Call(function, arguments) => {
                let arguments = arguments
                    .iter()
                    .enumerate()
                    .map(|(position, argument)| self.term_at(argument, &locus.child(position)))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call(*function, arguments, locus)?
            }
        })
    }

    /// A call of `function` on `arguments` at `locus`.
    fn call(
        &mut self,
        function: CheckedFunctionRef,
        arguments: Vec<Exp>,
        locus: &TermLocus,
    ) -> Result<Exp, ExpansionGap> {
        let graph = self.graph;
        let unreachable = ExpansionGap::UnreachableFunction(function);
        let state = graph
            .function_state(function.get())
            .ok_or_else(|| unreachable.clone())?;
        let signature = graph
            .callable(state.name)
            .ok_or_else(|| unreachable.clone())?;
        if signature.parameters.len() != arguments.len() {
            return Err(unreachable);
        }
        let arguments = arguments
            .into_iter()
            .zip(signature.parameters)
            .enumerate()
            .map(|(parameter, (argument, (_, ty)))| match result_kind(ty) {
                Some(ResultKind::Number(range)) if !range.is_unbounded() => Exp::Ranged {
                    value: Box::new(argument),
                    range,
                    role: RangeRole::Parameter {
                        function,
                        parameter,
                    },
                    locus: locus.clone(),
                },
                _ => argument,
            })
            .collect::<Vec<_>>();
        let result = result_kind(signature.result).ok_or(ExpansionGap::UnsupportedBodyForm {
            function,
            node: state.body.id().index(),
        })?;
        if self.is_recursive(function.get()) {
            let measured = signature
                .parameters
                .iter()
                .map(|(_, ty)| matches!(ty, ValueType::Integer | ValueType::Int(_)))
                .collect();
            return Ok(Exp::Recursive(RecursiveCall {
                function,
                arguments,
                measured,
                result,
                locus: locus.clone(),
            }));
        }
        self.body(function, arguments, locus)
            .map(|body| match result {
                ResultKind::Number(range) if !range.is_unbounded() => Exp::Ranged {
                    value: Box::new(body),
                    range,
                    role: RangeRole::Result { function },
                    locus: locus.clone(),
                },
                ResultKind::Number(_) | ResultKind::Boolean => body,
            })
    }

    /// `function`'s checked body over `arguments`, its parameters, with
    /// every non-recursive call in it expanded.
    pub(crate) fn body(
        &mut self,
        function: CheckedFunctionRef,
        arguments: Vec<Exp>,
        locus: &TermLocus,
    ) -> Result<Exp, ExpansionGap> {
        let state = self
            .graph
            .function_state(function.get())
            .ok_or(ExpansionGap::UnreachableFunction(function))?;
        let mut slots: Vec<Option<Exp>> = vec![None; state.slots.max(arguments.len())];
        for (slot, argument) in slots.iter_mut().zip(arguments) {
            *slot = Some(argument);
        }
        let inner = locus.in_body(function, state.body.id().index());
        self.node(function, state.body, &mut slots, &inner)
    }

    fn node(
        &mut self,
        function: CheckedFunctionRef,
        node: CheckedNode<'_>,
        slots: &mut Vec<Option<Exp>>,
        locus: &TermLocus,
    ) -> Result<Exp, ExpansionGap> {
        self.charge()?;
        let locus = locus.at_node(node.id().index());
        let unsupported = ExpansionGap::UnsupportedBodyForm {
            function,
            node: node.id().index(),
        };
        let child = |this: &mut Self, id, slots: &mut Vec<Option<Exp>>| {
            this.node(function, node.at(id), slots, &locus)
                .map(Box::new)
        };
        let arithmetic = |op: ArithmeticOperator| match op {
            ArithmeticOperator::Add => ArithmeticOp::Add,
            ArithmeticOperator::Subtract => ArithmeticOp::Sub,
            ArithmeticOperator::Multiply => ArithmeticOp::Mul,
            ArithmeticOperator::Divide => ArithmeticOp::Div,
        };
        Ok(match node.kind() {
            NodeKind::Literal(literal) => match &literal.0 {
                Value::Boolean(value) => Exp::Boolean(*value),
                Value::Integer(value) => Exp::Number(Rational::from_integer(value.clone())),
                Value::Rational(value) => Exp::Number(value.clone()),
                Value::Decimal(value) => Exp::Number(value.normalized().to_rational()),
                _ => return Err(unsupported),
            },
            NodeKind::Local(slot) => slots.get(*slot).cloned().flatten().ok_or(unsupported)?,
            NodeKind::Let { slot, value, body } => {
                let bound = self.node(function, node.at(*value), slots, &locus)?;
                let Some(entry) = slots.get_mut(*slot) else {
                    return Err(unsupported);
                };
                let saved = entry.replace(bound);
                let body = self.node(function, node.at(*body), slots, &locus);
                if let Some(entry) = slots.get_mut(*slot) {
                    *entry = saved;
                }
                body?
            }
            NodeKind::If {
                condition,
                then,
                otherwise,
            } => Exp::Conditional(
                child(self, *condition, slots)?,
                child(self, *then, slots)?,
                child(self, *otherwise, slots)?,
            ),
            NodeKind::Arithmetic(op, left, right) => {
                let op = match op {
                    Arithmetic::Add => ArithmeticOp::Add,
                    Arithmetic::Subtract => ArithmeticOp::Sub,
                    Arithmetic::Multiply => ArithmeticOp::Mul,
                };
                Exp::Arithmetic(
                    op,
                    child(self, *left, slots)?,
                    child(self, *right, slots)?,
                    locus.clone(),
                )
            }
            NodeKind::Rational {
                operator,
                left,
                right,
                ..
            }
            | NodeKind::Decimal {
                operator,
                left,
                right,
                ..
            } => Exp::Arithmetic(
                arithmetic(*operator),
                child(self, *left, slots)?,
                child(self, *right, slots)?,
                locus.clone(),
            ),
            NodeKind::Divide { left, right, .. } => Exp::Arithmetic(
                ArithmeticOp::Div,
                child(self, *left, slots)?,
                child(self, *right, slots)?,
                locus.clone(),
            ),
            NodeKind::Negate(operand)
            | NodeKind::RationalNegate(operand, _)
            | NodeKind::DecimalNegate(operand, _) => Exp::Negate(child(self, *operand, slots)?),
            // A real argument is passed as the real it is: a coercion's
            // range and a conversion's precision do not apply (QSpec FR-193
            // "Continuous values and expressions").
            NodeKind::Coerce(operand, _)
            | NodeKind::ConvertScalar(_, operand)
            | NodeKind::ConvertDecimal(operand, _) => {
                self.node(function, node.at(*operand), slots, &locus)?
            }
            NodeKind::Order(op, _, left, right) => Exp::Compare(
                match op {
                    OrderingOperator::Less => CompareOp::Lt,
                    OrderingOperator::LessOrEqual => CompareOp::Le,
                    OrderingOperator::Greater => CompareOp::Gt,
                    OrderingOperator::GreaterOrEqual => CompareOp::Ge,
                },
                child(self, *left, slots)?,
                child(self, *right, slots)?,
            ),
            NodeKind::Equality(op, _, left, right) => Exp::Compare(
                match op {
                    EqualityOperator::Equal => CompareOp::Eq,
                    EqualityOperator::NotEqual => CompareOp::Ne,
                },
                child(self, *left, slots)?,
                child(self, *right, slots)?,
            ),
            NodeKind::Connective(op, left, right) => Exp::Logic(
                match op {
                    Connective::And => LogicOp::And,
                    Connective::Or => LogicOp::Or,
                    Connective::Implies => LogicOp::Implies,
                },
                child(self, *left, slots)?,
                child(self, *right, slots)?,
            ),
            NodeKind::Not(operand) => Exp::Not(child(self, *operand, slots)?),
            NodeKind::Call {
                function: callee,
                arguments,
            } => {
                let arguments = arguments
                    .iter()
                    .map(|id| self.node(function, node.at(*id), slots, &locus))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call(CheckedFunctionRef::new(*callee), arguments, &locus)?
            }
            _ => return Err(unsupported),
        })
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
