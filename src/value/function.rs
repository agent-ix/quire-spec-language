// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-146 total pure functions: typed first-order declarations, purity,
//! path-sensitive definedness, well-founded termination over the call graph,
//! and fuel-metered evaluation.
//!
//! [`check_functions`] admits a closed set of [`FunctionDeclaration`]s or
//! returns the first located [`FunctionRefusal`] in declaration-key order.
//! Every branch is typed and checked for effects and declarations whether or
//! not it is reachable. A checked function publishes its callable identity,
//! its termination argument and its discharged partial-operation
//! preconditions as typed data. Evaluation never recurses on the host stack;
//! each call charges `function.call` after its arguments are evaluated and
//! before they are bound, and denied fuel is `incomplete` without changing the
//! checked verdict.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

use super::accounting::Meter;
use super::collection_query::{AlgebraicProperties, FoldFunction, ValueFunction};
use super::composite::{CompositeShape, FieldValue, Presence, TypeEnvironment, Value, ValueType};
use super::definition::AdmittedIntegerDivision;
use super::division::divide;
use super::integer::{Integer, IntegerDomain, IntegerInterval};
use super::node::NodeKey;
use super::outcome::{Outcome, Refusal};
use crate::diagnostic::Code;

// SPEC-GAP(119-19): the Complete-V1 function source form and its declaration
// node preimage are not available to this value layer. A function is
// identified by an opaque declaration `NodeKey` (as composites are,
// SPEC-GAP(119-5)); its body is the typed `Expression` tree below, which
// covers parameters, `let`, literals, calls, `if`, integer `+ - *`, integer
// comparison, the FR-147 quotient, record field projection, `present` and
// `value`. Integer `+ - *` charge nothing because `quire.value.accounting/v1`
// names no point for them. An FR-044 `Int[lo, hi]` parameter is modelled by an
// `IntegerInterval` on an `Integer` parameter.

/// A prohibited effect: a model or host operation with no Complete-V1 source
/// form.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Effect {
    /// Input or output.
    Io,
    /// Mutation of any state.
    Mutation,
    /// Reflection over declarations or values.
    Reflection,
    /// Lookup of ambient context.
    AmbientLookup,
}

/// An exact integer arithmetic operator.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ArithmeticOperator {
    /// `+`.
    Add,
    /// `-`.
    Subtract,
    /// `*`.
    Multiply,
}

/// An integer comparison operator.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IntegerComparison {
    /// `<`.
    Less,
    /// `<=`.
    LessOrEqual,
    /// `>`.
    Greater,
    /// `>=`.
    GreaterOrEqual,
    /// `==`.
    Equal,
    /// `!=`.
    NotEqual,
}

impl IntegerComparison {
    /// The operator with its operands swapped.
    fn flipped(self) -> Self {
        match self {
            Self::Less => Self::Greater,
            Self::LessOrEqual => Self::GreaterOrEqual,
            Self::Greater => Self::Less,
            Self::GreaterOrEqual => Self::LessOrEqual,
            Self::Equal => Self::Equal,
            Self::NotEqual => Self::NotEqual,
        }
    }
}

/// A typed function body expression.
#[derive(Clone, Debug)]
pub enum Expression {
    /// The zero-based parameter.
    Parameter(usize),
    /// The zero-based `let` name in scope, outermost first.
    Local(usize),
    /// A literal of a declared type.
    Literal {
        /// The value.
        value: Value,
        /// Its declared type.
        value_type: ValueType,
    },
    /// `let` binding `value` as the next local within `body`.
    Let {
        /// The bound value.
        value: Box<Expression>,
        /// The scope of the binding.
        body: Box<Expression>,
    },
    /// A call binding exact declaration identity and ordered arguments.
    Call {
        /// The callee declaration.
        callee: NodeKey,
        /// Arguments in parameter order.
        arguments: Vec<Expression>,
    },
    /// `if condition then then else otherwise`.
    If {
        /// A Boolean condition.
        condition: Box<Expression>,
        /// Taken when the condition is `true`.
        then: Box<Expression>,
        /// Taken when the condition is `false`.
        otherwise: Box<Expression>,
    },
    /// Exact integer arithmetic.
    Arithmetic {
        /// The operator.
        operator: ArithmeticOperator,
        /// Left operand.
        left: Box<Expression>,
        /// Right operand.
        right: Box<Expression>,
    },
    /// Integer comparison.
    Compare {
        /// The operator.
        operator: IntegerComparison,
        /// Left operand.
        left: Box<Expression>,
        /// Right operand.
        right: Box<Expression>,
    },
    /// The FR-147 quotient under an admitted law; its divisor must be proved
    /// nonzero.
    Quotient {
        /// The admitted `div` law.
        law: AdmittedIntegerDivision,
        /// Dividend.
        dividend: Box<Expression>,
        /// Divisor.
        divisor: Box<Expression>,
    },
    /// Projection of a required record field.
    Field {
        /// A record value.
        record: Box<Expression>,
        /// The field identity.
        field: NodeKey,
    },
    /// `present(e)` for an option.
    Present(Box<Expression>),
    /// `value(e)` for an option; `present(e)` must be proved.
    Value(Box<Expression>),
    /// A call to a model or host operation. It refuses the declaration even
    /// when unreachable.
    Effect(Effect),
}

/// A parameter declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterDeclaration {
    /// The value type.
    pub value_type: ValueType,
    /// For an `Integer` parameter, the declared `Int[lo, hi]` interval.
    pub interval: Option<IntegerInterval>,
}

impl ParameterDeclaration {
    /// An unrefined parameter of `value_type`.
    pub fn new(value_type: ValueType) -> Self {
        Self {
            value_type,
            interval: None,
        }
    }

    /// An `Int[lower, upper]` parameter.
    pub fn bounded(interval: IntegerInterval) -> Self {
        Self {
            value_type: ValueType::Integer,
            interval: Some(interval),
        }
    }

    fn admits(&self, value: &Value) -> bool {
        match (&self.interval, value) {
            (None, _) => self.value_type.admits(value),
            (Some(interval), Value::Integer(integer)) => interval.contains(integer),
            (Some(_), _) => false,
        }
    }
}

/// One element of a lexicographic `decreases` measure, over a parameter.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MeasureElement {
    /// A nonnegative `Int[lo, hi]` parameter, ordered by `<`.
    Integer(usize),
    /// `size(p)` of a finite collection parameter.
    Cardinality(usize),
    /// A record parameter, ordered by strict containment.
    Containment(usize),
}

impl MeasureElement {
    /// The measured parameter.
    pub fn parameter(self) -> usize {
        match self {
            Self::Integer(parameter)
            | Self::Cardinality(parameter)
            | Self::Containment(parameter) => parameter,
        }
    }

    fn same_kind(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Integer(_), Self::Integer(_))
                | (Self::Cardinality(_), Self::Cardinality(_))
                | (Self::Containment(_), Self::Containment(_))
        )
    }
}

/// A function declaration.
#[derive(Clone, Debug)]
pub struct FunctionDeclaration {
    /// Declaration identity.
    pub key: NodeKey,
    /// Parameters in order.
    pub parameters: Vec<ParameterDeclaration>,
    /// Result type.
    pub result: ValueType,
    /// Declared effects; a pure function declares none.
    pub effects: BTreeSet<Effect>,
    /// The `decreases` tuple, required in a recursive component.
    pub measure: Option<Vec<MeasureElement>>,
    /// The body.
    pub body: Expression,
}

/// One step from an expression to a subexpression.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PathStep {
    /// A call argument.
    Argument(usize),
    /// An `if` condition.
    Condition,
    /// An `if` `then` branch.
    Then,
    /// An `if` `else` branch.
    Otherwise,
    /// A binary left operand or dividend.
    Left,
    /// A binary right operand or divisor.
    Right,
    /// A `let` value.
    Bound,
    /// A `let` body.
    Body,
    /// The operand of a projection, `present` or `value`.
    Operand,
}

/// A located expression inside a declaration; the body is the empty path.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Location {
    /// The declaration.
    pub function: NodeKey,
    /// Steps from its body.
    pub path: Vec<PathStep>,
}

/// A partial-operation precondition.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Precondition {
    /// A quotient divisor is nonzero.
    NonzeroDivisor,
    /// `present(e)` holds for `value(e)`.
    Present,
    /// A call argument lies in the callee parameter's `Int[lo, hi]`.
    ArgumentInterval(usize),
}

/// How a precondition was proved.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PreconditionEvidence {
    /// The operand is a nonzero literal.
    NonzeroLiteral,
    /// A dominating `!= 0` guard excludes zero.
    ExcludedZero,
    /// Declared intervals and dominating guards bound the operand.
    Bounds {
        /// Proved lower bound.
        lower: Option<Integer>,
        /// Proved upper bound.
        upper: Option<Integer>,
    },
    /// A dominating `present` guard on the same access path.
    PresenceGuard,
}

/// A discharged definedness obligation.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DischargedPrecondition {
    /// The partial operation.
    pub location: Location,
    /// The obligation.
    pub precondition: Precondition,
    /// Its proof.
    pub evidence: PreconditionEvidence,
}

/// The relation of one substituted callee measure element to the caller's.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ElementRelation {
    /// Proved equal.
    Equal,
    /// Proved strictly smaller.
    Less,
    /// Neither proved.
    Unproved,
}

/// The decrease obligation of one recursive call edge.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DecreaseObligation {
    /// The call site.
    pub call: Location,
    /// The callee.
    pub callee: NodeKey,
    /// Per-position relations of the substituted callee tuple to the caller's.
    pub relations: Vec<ElementRelation>,
}

impl DecreaseObligation {
    /// The position that proves the lexicographic decrease, if any.
    pub fn decreasing_position(&self) -> Option<usize> {
        let position = self
            .relations
            .iter()
            .position(|relation| *relation != ElementRelation::Equal)?;
        (self.relations.get(position) == Some(&ElementRelation::Less)).then_some(position)
    }
}

/// The termination argument of a checked function.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Termination {
    /// Not in any call cycle; no measure is needed.
    Acyclic,
    /// In a recursive strongly connected component.
    Recursive {
        /// The component members in key order.
        component: Vec<NodeKey>,
        /// This function's measure.
        measure: Vec<MeasureElement>,
        /// The proved obligations of its calls into the component.
        edges: Vec<DecreaseObligation>,
    },
}

/// Why a declaration set is not admitted.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum FunctionCause {
    /// Two declarations share this identity.
    DuplicateFunction,
    /// A body nests deeper than the checking limit.
    ExpressionTooDeep,
    /// A declaration declares an effect.
    DeclaredEffect(Effect),
    /// A body calls a model or host operation, reachable or not.
    ProhibitedEffect(Effect),
    /// A parameter index is out of range.
    UnknownParameter(usize),
    /// A local index is not in scope.
    UnknownLocal(usize),
    /// An interval is declared on a non-`Integer` parameter.
    IntervalOnNonInteger(usize),
    /// A literal is not a member of its declared type.
    LiteralType,
    /// A call names no declaration.
    UnknownCallee(NodeKey),
    /// A call has the wrong number of arguments.
    Arity {
        /// Declared parameters.
        declared: usize,
        /// Supplied arguments.
        supplied: usize,
    },
    /// An argument is not of the parameter type.
    ArgumentType(usize),
    /// A result path does not produce the declared result type.
    ResultType,
    /// A condition is not Boolean.
    ConditionNotBoolean,
    /// An arithmetic, comparison or quotient operand is not an integer.
    OperandNotInteger,
    /// A projection operand is not a record.
    ProjectionNotRecord,
    /// The record declares no such field.
    UnknownField(NodeKey),
    /// A projected field is not always present; its meaning is not supported.
    FieldNotRequired(NodeKey),
    /// A `present` or `value` operand is not an option.
    OperandNotOption,
    /// A partial operation's precondition is not proved.
    UnprovedPrecondition(Precondition),
    /// A recursive component member declares no measure.
    MissingMeasure {
        /// A cycle through the member, first and last equal.
        cycle: Vec<NodeKey>,
    },
    /// A measure element names an absent parameter or one of the wrong kind.
    MeasureParameter(usize),
    /// An integer measure element is not proved nonnegative on entry.
    MeasureNotNonnegative(usize),
    /// Component members' measures differ in arity or element kinds.
    MeasureShape {
        /// A cycle through the component, first and last equal.
        cycle: Vec<NodeKey>,
    },
    /// A recursive call does not decrease the shared measure.
    NonDecreasing {
        /// A cycle through the edge, first and last equal.
        cycle: Vec<NodeKey>,
        /// The failed obligation.
        obligation: Box<DecreaseObligation>,
    },
}

impl FunctionCause {
    /// The FR-146 refusal code.
    pub fn code(&self) -> Code {
        match self {
            Self::DuplicateFunction => Code::AmbiguousDeclaration,
            Self::ExpressionTooDeep => Code::ResourceExhausted,
            Self::UnknownCallee(_) => Code::MissingDeclaration,
            Self::FieldNotRequired(_) => Code::UnsupportedConstruct,
            Self::DeclaredEffect(_)
            | Self::ProhibitedEffect(_)
            | Self::UnknownParameter(_)
            | Self::UnknownLocal(_)
            | Self::IntervalOnNonInteger(_)
            | Self::LiteralType
            | Self::Arity { .. }
            | Self::ArgumentType(_)
            | Self::ResultType
            | Self::ConditionNotBoolean
            | Self::OperandNotInteger
            | Self::ProjectionNotRecord
            | Self::UnknownField(_)
            | Self::OperandNotOption => Code::IllTyped,
            Self::UnprovedPrecondition(_)
            | Self::MissingMeasure { .. }
            | Self::MeasureParameter(_)
            | Self::MeasureNotNonnegative(_)
            | Self::MeasureShape { .. }
            | Self::NonDecreasing { .. } => Code::UndefinedExpression,
        }
    }
}

/// A located function refusal.
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("function refused at {location:?}: {cause:?}")]
pub struct FunctionRefusal {
    /// Where the refusal originates.
    pub location: Location,
    /// The typed cause.
    pub cause: FunctionCause,
}

/// Checking limits.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FunctionLimits {
    /// Maximum body nesting depth; the body root has depth one.
    pub expression_depth: u64,
}

/// One admitted function and its published facts.
#[derive(Clone, Debug)]
pub struct CheckedFunction {
    declaration: FunctionDeclaration,
    termination: Termination,
    definedness: Vec<DischargedPrecondition>,
}

impl CheckedFunction {
    /// Declaration identity.
    pub fn key(&self) -> NodeKey {
        self.declaration.key
    }

    /// Parameters.
    pub fn parameters(&self) -> &[ParameterDeclaration] {
        &self.declaration.parameters
    }

    /// Result type.
    pub fn result(&self) -> &ValueType {
        &self.declaration.result
    }

    /// The termination argument.
    pub fn termination(&self) -> &Termination {
        &self.termination
    }

    /// Every discharged partial-operation precondition, in body order.
    pub fn definedness(&self) -> &[DischargedPrecondition] {
        &self.definedness
    }
}

/// A closed set of admitted total pure functions.
#[derive(Clone, Debug)]
pub struct CheckedFunctions {
    functions: BTreeMap<NodeKey, CheckedFunction>,
    /// Slot position of each projected `(record declaration, field)`.
    fields: BTreeMap<(NodeKey, NodeKey), usize>,
}

/// Why a call into [`CheckedFunctions`] is ill-formed.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
pub enum CallRefusal {
    /// No checked function has this identity.
    #[error("unknown function")]
    UnknownFunction,
    /// Wrong argument count.
    #[error("wrong arity")]
    Arity,
    /// This argument is not a member of its parameter type.
    #[error("argument {0} has the wrong type")]
    ArgumentType(usize),
}

// ---- checking ----------------------------------------------------------------

/// Admit `declarations` over `environment`: typing, purity and definedness
/// per body, then a termination argument per call-graph component.
pub fn check_functions(
    environment: &TypeEnvironment,
    declarations: Vec<FunctionDeclaration>,
    limits: FunctionLimits,
) -> Result<CheckedFunctions, FunctionRefusal> {
    let mut by_key = BTreeMap::new();
    for declaration in declarations {
        let key = declaration.key;
        if by_key.insert(key, declaration).is_some() {
            return Err(refuse(key, Vec::new(), FunctionCause::DuplicateFunction));
        }
    }
    let mut bodies = BTreeMap::new();
    let mut fields = BTreeMap::new();
    for (key, declaration) in &by_key {
        if let Some(effect) = declaration.effects.first() {
            return Err(refuse(
                *key,
                Vec::new(),
                FunctionCause::DeclaredEffect(*effect),
            ));
        }
        if let Some(index) = declaration.parameters.iter().position(|parameter| {
            parameter.interval.is_some() && parameter.value_type != ValueType::Integer
        }) {
            return Err(refuse(
                *key,
                Vec::new(),
                FunctionCause::IntervalOnNonInteger(index),
            ));
        }
        if depth_exceeds(&declaration.body, limits.expression_depth) {
            return Err(refuse(*key, Vec::new(), FunctionCause::ExpressionTooDeep));
        }
        let mut checker = BodyChecker {
            environment,
            declarations: &by_key,
            declaration,
            path: Vec::new(),
            locals: Vec::new(),
            facts: Vec::new(),
            calls: Vec::new(),
            definedness: Vec::new(),
            fields: &mut fields,
        };
        checker.check(
            &declaration.body,
            &declaration.result,
            FunctionCause::ResultType,
        )?;
        let (calls, definedness) = (checker.calls, checker.definedness);
        bodies.insert(*key, (calls, definedness));
    }
    let mut terminations = termination(&by_key, &bodies)?;
    let functions = by_key
        .into_iter()
        .map(|(key, declaration)| {
            let definedness = bodies
                .remove(&key)
                .map(|(_, facts)| facts)
                .unwrap_or_default();
            let termination = terminations.remove(&key).unwrap_or(Termination::Acyclic);
            (
                key,
                CheckedFunction {
                    declaration,
                    termination,
                    definedness,
                },
            )
        })
        .collect();
    Ok(CheckedFunctions { functions, fields })
}

fn refuse(function: NodeKey, path: Vec<PathStep>, cause: FunctionCause) -> FunctionRefusal {
    FunctionRefusal {
        location: Location { function, path },
        cause,
    }
}

/// Whether `body` nests deeper than `limit`, without host recursion.
fn depth_exceeds(body: &Expression, limit: u64) -> bool {
    let mut pending = vec![(body, 1_u64)];
    while let Some((expression, depth)) = pending.pop() {
        if depth > limit {
            return true;
        }
        let next = depth.saturating_add(1);
        pending.extend(children(expression).into_iter().map(|child| (child, next)));
    }
    false
}

fn children(expression: &Expression) -> Vec<&Expression> {
    match expression {
        Expression::Parameter(_)
        | Expression::Local(_)
        | Expression::Literal { .. }
        | Expression::Effect(_) => Vec::new(),
        Expression::Call { arguments, .. } => arguments.iter().collect(),
        Expression::Let { value, body } => vec![value, body],
        Expression::If {
            condition,
            then,
            otherwise,
        } => vec![condition, then, otherwise],
        Expression::Arithmetic { left, right, .. } | Expression::Compare { left, right, .. } => {
            vec![left, right]
        }
        Expression::Quotient {
            dividend, divisor, ..
        } => vec![dividend, divisor],
        Expression::Field { record, .. } => vec![record],
        Expression::Present(operand) | Expression::Value(operand) => vec![operand],
    }
}

/// The root of an access path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Root {
    Parameter(usize),
    Local(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Projection {
    Field(NodeKey),
    Value,
}

/// A parameter or local followed by projections; guards and measures speak
/// about these.
#[derive(Clone, Debug, Eq, PartialEq)]
struct AccessPath {
    root: Root,
    projections: Vec<Projection>,
}

impl AccessPath {
    fn is_parameter(&self, index: usize) -> bool {
        self.root == Root::Parameter(index) && self.projections.is_empty()
    }
}

/// A fact that a guard proves on one branch.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Fact {
    AtLeast(AccessPath, Integer),
    AtMost(AccessPath, Integer),
    NotEqual(AccessPath, Integer),
    Present(AccessPath),
}

/// What a call argument is, relative to the caller's access paths.
#[derive(Clone, Debug, Eq, PartialEq)]
enum ArgumentShape {
    Path(AccessPath),
    /// `path - amount` for a positive literal `amount`.
    Decrement(AccessPath),
    Other,
}

struct CallSite {
    path: Vec<PathStep>,
    callee: NodeKey,
    arguments: Vec<ArgumentShape>,
}

struct Local {
    value_type: ValueType,
    alias: Option<AccessPath>,
}

#[derive(Clone, Debug, Default)]
struct Bounds {
    lower: Option<Integer>,
    upper: Option<Integer>,
}

struct BodyChecker<'a> {
    environment: &'a TypeEnvironment,
    declarations: &'a BTreeMap<NodeKey, FunctionDeclaration>,
    declaration: &'a FunctionDeclaration,
    path: Vec<PathStep>,
    locals: Vec<Local>,
    facts: Vec<Fact>,
    calls: Vec<CallSite>,
    definedness: Vec<DischargedPrecondition>,
    fields: &'a mut BTreeMap<(NodeKey, NodeKey), usize>,
}

impl BodyChecker<'_> {
    fn refuse<T>(&self, cause: FunctionCause) -> Result<T, FunctionRefusal> {
        Err(refuse(self.declaration.key, self.path.clone(), cause))
    }

    fn within<T>(
        &mut self,
        step: PathStep,
        run: impl FnOnce(&mut Self) -> Result<T, FunctionRefusal>,
    ) -> Result<T, FunctionRefusal> {
        self.path.push(step);
        let result = run(self);
        self.path.pop();
        result
    }

    fn discharge(&mut self, precondition: Precondition, evidence: PreconditionEvidence) {
        self.definedness.push(DischargedPrecondition {
            location: Location {
                function: self.declaration.key,
                path: self.path.clone(),
            },
            precondition,
            evidence,
        });
    }

    /// Check `expression` against `expected`; a mismatch refuses with
    /// `mismatch`. `if` and `let` check each result path.
    fn check(
        &mut self,
        expression: &Expression,
        expected: &ValueType,
        mismatch: FunctionCause,
    ) -> Result<(), FunctionRefusal> {
        match expression {
            Expression::If {
                condition,
                then,
                otherwise,
            } => {
                let facts = self.condition(condition)?;
                self.branches(facts, |checker, step| {
                    let branch = if step == PathStep::Then {
                        then
                    } else {
                        otherwise
                    };
                    checker.check(branch, expected, mismatch.clone())
                })
            }
            Expression::Let { value, body } => {
                let local = self.bound(value)?;
                self.bind(local, |checker| {
                    checker.within(PathStep::Body, |checker| {
                        checker.check(body, expected, mismatch)
                    })
                })
            }
            Expression::Parameter(_)
            | Expression::Local(_)
            | Expression::Literal { .. }
            | Expression::Call { .. }
            | Expression::Arithmetic { .. }
            | Expression::Compare { .. }
            | Expression::Quotient { .. }
            | Expression::Field { .. }
            | Expression::Present(_)
            | Expression::Value(_)
            | Expression::Effect(_) => {
                if &self.synthesize(expression)? == expected {
                    Ok(())
                } else {
                    self.refuse(mismatch)
                }
            }
        }
    }

    /// The type of `expression`.
    fn synthesize(&mut self, expression: &Expression) -> Result<ValueType, FunctionRefusal> {
        match expression {
            Expression::Parameter(index) => match self.declaration.parameters.get(*index) {
                Some(parameter) => Ok(parameter.value_type.clone()),
                None => self.refuse(FunctionCause::UnknownParameter(*index)),
            },
            Expression::Local(index) => match self.locals.get(*index) {
                Some(local) => Ok(local.value_type.clone()),
                None => self.refuse(FunctionCause::UnknownLocal(*index)),
            },
            Expression::Literal { value, value_type } => {
                if value_type.admits(value) {
                    Ok(value_type.clone())
                } else {
                    self.refuse(FunctionCause::LiteralType)
                }
            }
            Expression::Effect(effect) => self.refuse(FunctionCause::ProhibitedEffect(*effect)),
            Expression::Call { callee, arguments } => self.call(*callee, arguments),
            Expression::Let { value, body } => {
                let local = self.bound(value)?;
                self.bind(local, |checker| {
                    checker.within(PathStep::Body, |checker| checker.synthesize(body))
                })
            }
            Expression::Arithmetic { left, right, .. } => {
                self.integer_operands(left, right)?;
                Ok(ValueType::Integer)
            }
            Expression::Compare { left, right, .. } => {
                self.integer_operands(left, right)?;
                Ok(ValueType::Boolean)
            }
            Expression::Quotient {
                dividend, divisor, ..
            } => {
                self.integer_operands(dividend, divisor)?;
                let evidence = self.within(PathStep::Right, |checker| checker.nonzero(divisor))?;
                self.discharge(Precondition::NonzeroDivisor, evidence);
                Ok(ValueType::Integer)
            }
            Expression::Field { record, field } => self.field(record, *field),
            Expression::Present(operand) => {
                self.option_operand(operand)?;
                Ok(ValueType::Boolean)
            }
            Expression::Value(operand) => {
                let payload = self.option_operand(operand)?;
                let proved = self
                    .access_path(operand)
                    .is_some_and(|path| self.facts.contains(&Fact::Present(path)));
                if !proved {
                    return self.refuse(FunctionCause::UnprovedPrecondition(Precondition::Present));
                }
                self.discharge(Precondition::Present, PreconditionEvidence::PresenceGuard);
                Ok(payload)
            }
            Expression::If {
                condition,
                then,
                otherwise,
            } => {
                let facts = self.condition(condition)?;
                let mut synthesized = None;
                // `branches` runs `then` first, which fixes the type that
                // `else` is checked against.
                self.branches(facts, |checker, step| match (&synthesized, step) {
                    (None, _) => {
                        synthesized = Some(checker.synthesize(then)?);
                        Ok(())
                    }
                    (Some(expected), _) => {
                        let expected = expected.clone();
                        checker.check(otherwise, &expected, FunctionCause::ResultType)
                    }
                })?;
                match synthesized {
                    Some(value_type) => Ok(value_type),
                    None => self.refuse(FunctionCause::ResultType),
                }
            }
        }
    }

    fn integer_operands(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> Result<(), FunctionRefusal> {
        self.within(PathStep::Left, |checker| {
            checker.check(left, &ValueType::Integer, FunctionCause::OperandNotInteger)
        })?;
        self.within(PathStep::Right, |checker| {
            checker.check(right, &ValueType::Integer, FunctionCause::OperandNotInteger)
        })
    }

    fn option_operand(&mut self, operand: &Expression) -> Result<ValueType, FunctionRefusal> {
        self.within(PathStep::Operand, |checker| {
            match checker.synthesize(operand)? {
                ValueType::Option(payload) => Ok(*payload),
                _ => checker.refuse(FunctionCause::OperandNotOption),
            }
        })
    }

    fn field(&mut self, record: &Expression, field: NodeKey) -> Result<ValueType, FunctionRefusal> {
        self.within(PathStep::Operand, |checker| {
            let ValueType::Composite(declaration) = checker.synthesize(record)? else {
                return checker.refuse(FunctionCause::ProjectionNotRecord);
            };
            let Some(CompositeShape::Record(declared)) = checker
                .environment
                .declaration(declaration)
                .map(|declaration| declaration.shape())
            else {
                return checker.refuse(FunctionCause::ProjectionNotRecord);
            };
            let Some((position, found)) = declared
                .iter()
                .enumerate()
                .find(|(_, candidate)| candidate.key() == field)
            else {
                return checker.refuse(FunctionCause::UnknownField(field));
            };
            if found.presence() != Presence::Required {
                return checker.refuse(FunctionCause::FieldNotRequired(field));
            }
            let value_type = found.value_type().clone();
            checker.fields.insert((declaration, field), position);
            Ok(value_type)
        })
    }

    fn bound(&mut self, value: &Expression) -> Result<Local, FunctionRefusal> {
        let value_type = self.within(PathStep::Bound, |checker| checker.synthesize(value))?;
        Ok(Local {
            value_type,
            alias: self.access_path(value),
        })
    }

    fn bind<T>(
        &mut self,
        local: Local,
        run: impl FnOnce(&mut Self) -> Result<T, FunctionRefusal>,
    ) -> Result<T, FunctionRefusal> {
        self.locals.push(local);
        let result = run(self);
        self.locals.pop();
        result
    }

    fn call(
        &mut self,
        callee: NodeKey,
        arguments: &[Expression],
    ) -> Result<ValueType, FunctionRefusal> {
        let Some(declaration) = self.declarations.get(&callee) else {
            return self.refuse(FunctionCause::UnknownCallee(callee));
        };
        if declaration.parameters.len() != arguments.len() {
            return self.refuse(FunctionCause::Arity {
                declared: declaration.parameters.len(),
                supplied: arguments.len(),
            });
        }
        let mut shapes = Vec::with_capacity(arguments.len());
        for (index, (argument, parameter)) in
            arguments.iter().zip(&declaration.parameters).enumerate()
        {
            self.within(PathStep::Argument(index), |checker| {
                checker.check(
                    argument,
                    &parameter.value_type,
                    FunctionCause::ArgumentType(index),
                )?;
                if let Some(interval) = &parameter.interval {
                    let bounds = checker.bounds_of(argument);
                    let within = bounds
                        .lower
                        .as_ref()
                        .is_some_and(|lower| lower >= interval.lower())
                        && bounds
                            .upper
                            .as_ref()
                            .is_some_and(|upper| upper <= interval.upper());
                    if !within {
                        return checker.refuse(FunctionCause::UnprovedPrecondition(
                            Precondition::ArgumentInterval(index),
                        ));
                    }
                    checker.discharge(
                        Precondition::ArgumentInterval(index),
                        PreconditionEvidence::Bounds {
                            lower: bounds.lower,
                            upper: bounds.upper,
                        },
                    );
                }
                Ok(())
            })?;
            shapes.push(self.argument_shape(argument));
        }
        self.calls.push(CallSite {
            path: self.path.clone(),
            callee,
            arguments: shapes,
        });
        Ok(declaration.result.clone())
    }

    /// Check a condition and return the facts it proves on its `then` and
    /// `else` branches.
    fn condition(&mut self, condition: &Expression) -> Result<[Vec<Fact>; 2], FunctionRefusal> {
        self.within(PathStep::Condition, |checker| {
            checker.check(
                condition,
                &ValueType::Boolean,
                FunctionCause::ConditionNotBoolean,
            )
        })?;
        Ok(self.branch_facts(condition))
    }

    /// Run `run` for `Then` then `Otherwise` under each branch's facts.
    fn branches(
        &mut self,
        facts: [Vec<Fact>; 2],
        mut run: impl FnMut(&mut Self, PathStep) -> Result<(), FunctionRefusal>,
    ) -> Result<(), FunctionRefusal> {
        for (step, branch) in [PathStep::Then, PathStep::Otherwise].into_iter().zip(facts) {
            let mark = self.facts.len();
            self.facts.extend(branch);
            let result = self.within(step, |checker| run(checker, step));
            self.facts.truncate(mark);
            result?;
        }
        Ok(())
    }

    fn access_path(&self, expression: &Expression) -> Option<AccessPath> {
        match expression {
            Expression::Parameter(index) => Some(AccessPath {
                root: Root::Parameter(*index),
                projections: Vec::new(),
            }),
            Expression::Local(index) => {
                let local = self.locals.get(*index)?;
                Some(local.alias.clone().unwrap_or(AccessPath {
                    root: Root::Local(*index),
                    projections: Vec::new(),
                }))
            }
            Expression::Field { record, field } => {
                let mut path = self.access_path(record)?;
                path.projections.push(Projection::Field(*field));
                Some(path)
            }
            Expression::Value(operand) => {
                let mut path = self.access_path(operand)?;
                path.projections.push(Projection::Value);
                Some(path)
            }
            _ => None,
        }
    }

    fn path_bounds(&self, path: &AccessPath) -> Bounds {
        let declared = match path.root {
            Root::Parameter(index) if path.projections.is_empty() => self
                .declaration
                .parameters
                .get(index)
                .and_then(|parameter| parameter.interval.as_ref()),
            Root::Parameter(_) | Root::Local(_) => None,
        };
        let mut bounds = Bounds {
            lower: declared.map(|interval| interval.lower().clone()),
            upper: declared.map(|interval| interval.upper().clone()),
        };
        for fact in &self.facts {
            match fact {
                Fact::AtLeast(fact_path, value) if fact_path == path => {
                    if bounds.lower.as_ref().is_none_or(|lower| value > lower) {
                        bounds.lower = Some(value.clone());
                    }
                }
                Fact::AtMost(fact_path, value) if fact_path == path => {
                    if bounds.upper.as_ref().is_none_or(|upper| value < upper) {
                        bounds.upper = Some(value.clone());
                    }
                }
                Fact::AtLeast(..) | Fact::AtMost(..) | Fact::NotEqual(..) | Fact::Present(_) => {}
            }
        }
        // Each pass can move an end past one excluded value.
        for _ in 0..self.facts.len() {
            let mut moved = false;
            for fact in &self.facts {
                let Fact::NotEqual(fact_path, excluded) = fact else {
                    continue;
                };
                if fact_path != path {
                    continue;
                }
                if bounds.lower.as_ref() == Some(excluded) {
                    bounds.lower = Some(excluded.add(&Integer::one()));
                    moved = true;
                }
                if bounds.upper.as_ref() == Some(excluded) {
                    bounds.upper = Some(excluded.sub(&Integer::one()));
                    moved = true;
                }
            }
            if !moved {
                break;
            }
        }
        bounds
    }

    fn bounds_of(&self, expression: &Expression) -> Bounds {
        match expression {
            Expression::Literal {
                value: Value::Integer(value),
                ..
            } => Bounds {
                lower: Some(value.clone()),
                upper: Some(value.clone()),
            },
            Expression::Arithmetic {
                operator,
                left,
                right,
            } => {
                let (left, right) = (self.bounds_of(left), self.bounds_of(right));
                let combine = |a: Option<&Integer>, b: Option<&Integer>, add: bool| match (a, b) {
                    (Some(a), Some(b)) if add => Some(a.add(b)),
                    (Some(a), Some(b)) => Some(a.sub(b)),
                    _ => None,
                };
                match operator {
                    ArithmeticOperator::Add => Bounds {
                        lower: combine(left.lower.as_ref(), right.lower.as_ref(), true),
                        upper: combine(left.upper.as_ref(), right.upper.as_ref(), true),
                    },
                    ArithmeticOperator::Subtract => Bounds {
                        lower: combine(left.lower.as_ref(), right.upper.as_ref(), false),
                        upper: combine(left.upper.as_ref(), right.lower.as_ref(), false),
                    },
                    ArithmeticOperator::Multiply => Bounds::default(),
                }
            }
            _ => self
                .access_path(expression)
                .map(|path| self.path_bounds(&path))
                .unwrap_or_default(),
        }
    }

    fn nonzero(&self, divisor: &Expression) -> Result<PreconditionEvidence, FunctionRefusal> {
        if let Expression::Literal {
            value: Value::Integer(value),
            ..
        } = divisor
        {
            if !value.is_zero() {
                return Ok(PreconditionEvidence::NonzeroLiteral);
            }
        }
        if let Some(path) = self.access_path(divisor) {
            if self.facts.contains(&Fact::NotEqual(path, Integer::zero())) {
                return Ok(PreconditionEvidence::ExcludedZero);
            }
        }
        let bounds = self.bounds_of(divisor);
        let positive = bounds
            .lower
            .as_ref()
            .is_some_and(|lower| !lower.is_negative() && !lower.is_zero());
        let negative = bounds.upper.as_ref().is_some_and(Integer::is_negative);
        if positive || negative {
            return Ok(PreconditionEvidence::Bounds {
                lower: bounds.lower,
                upper: bounds.upper,
            });
        }
        self.refuse(FunctionCause::UnprovedPrecondition(
            Precondition::NonzeroDivisor,
        ))
    }

    fn argument_shape(&self, argument: &Expression) -> ArgumentShape {
        if let Some(path) = self.access_path(argument) {
            return ArgumentShape::Path(path);
        }
        match argument {
            Expression::Arithmetic {
                operator: ArithmeticOperator::Subtract,
                left,
                right,
            } => match (self.access_path(left), &**right) {
                (
                    Some(path),
                    Expression::Literal {
                        value: Value::Integer(amount),
                        ..
                    },
                ) if !amount.is_negative() && !amount.is_zero() => ArgumentShape::Decrement(path),
                _ => ArgumentShape::Other,
            },
            _ => ArgumentShape::Other,
        }
    }

    /// The facts a condition proves on its `then` and `else` paths.
    // SPEC-GAP(119-20): FR-146 proves decreases and preconditions "under the
    // sound FR-044 guard facts" but this layer has no FR-044 fact engine. The
    // accepted evidence is exactly: a guard `path OP integer-literal` (either
    // side) bounding or excluding a value on one branch; `present(path)` on
    // its `then` branch; declared parameter intervals; interval arithmetic for
    // `+` and `-` of bounded operands; a nonzero literal, `!= 0` guard or
    // sign-proving bounds for a divisor; an integer decrease only as
    // `parameter - positive-literal`; containment only as a field projection
    // path of the parameter; and equality only as the parameter itself. An
    // unproved argument interval refuses as `undefined_expression`. No
    // cardinality decrease form exists, and no measure element is an
    // expression other than a parameter. This function, `bounds_of`,
    // `nonzero`, `argument_shape` and `relate` are the only places that decide
    // it.
    fn branch_facts(&self, condition: &Expression) -> [Vec<Fact>; 2] {
        match condition {
            Expression::Present(operand) => match self.access_path(operand) {
                Some(path) => [vec![Fact::Present(path)], Vec::new()],
                None => [Vec::new(), Vec::new()],
            },
            Expression::Compare {
                operator,
                left,
                right,
            } => {
                let (operator, path, literal) = match (&**left, &**right) {
                    (
                        operand,
                        Expression::Literal {
                            value: Value::Integer(literal),
                            ..
                        },
                    ) => (*operator, self.access_path(operand), literal),
                    (
                        Expression::Literal {
                            value: Value::Integer(literal),
                            ..
                        },
                        operand,
                    ) => (operator.flipped(), self.access_path(operand), literal),
                    _ => return [Vec::new(), Vec::new()],
                };
                let Some(path) = path else {
                    return [Vec::new(), Vec::new()];
                };
                let above = literal.add(&Integer::one());
                let below = literal.sub(&Integer::one());
                let fact = |make: fn(AccessPath, Integer) -> Fact, value: Integer| {
                    vec![make(path.clone(), value)]
                };
                match operator {
                    IntegerComparison::Greater => [
                        fact(Fact::AtLeast, above),
                        fact(Fact::AtMost, literal.clone()),
                    ],
                    IntegerComparison::GreaterOrEqual => [
                        fact(Fact::AtLeast, literal.clone()),
                        fact(Fact::AtMost, below),
                    ],
                    IntegerComparison::Less => [
                        fact(Fact::AtMost, below),
                        fact(Fact::AtLeast, literal.clone()),
                    ],
                    IntegerComparison::LessOrEqual => [
                        fact(Fact::AtMost, literal.clone()),
                        fact(Fact::AtLeast, above),
                    ],
                    IntegerComparison::Equal => [
                        vec![
                            Fact::AtLeast(path.clone(), literal.clone()),
                            Fact::AtMost(path.clone(), literal.clone()),
                        ],
                        fact(Fact::NotEqual, literal.clone()),
                    ],
                    IntegerComparison::NotEqual => [
                        fact(Fact::NotEqual, literal.clone()),
                        vec![
                            Fact::AtLeast(path.clone(), literal.clone()),
                            Fact::AtMost(path.clone(), literal.clone()),
                        ],
                    ],
                }
            }
            _ => [Vec::new(), Vec::new()],
        }
    }
}

type Bodies = BTreeMap<NodeKey, (Vec<CallSite>, Vec<DischargedPrecondition>)>;

fn successors(bodies: &Bodies, key: &NodeKey) -> Vec<NodeKey> {
    let mut callees: Vec<NodeKey> = bodies
        .get(key)
        .map(|(calls, _)| calls.iter().map(|call| call.callee).collect())
        .unwrap_or_default();
    callees.sort();
    callees.dedup();
    callees
}

/// Strongly connected components in deterministic order (iterative Tarjan).
fn components(bodies: &Bodies) -> Vec<Vec<NodeKey>> {
    let mut index: BTreeMap<NodeKey, (usize, usize)> = BTreeMap::new();
    let mut on_stack = BTreeSet::new();
    let mut stack = Vec::new();
    let mut found = Vec::new();
    let mut counter = 0_usize;
    for root in bodies.keys().copied() {
        if index.contains_key(&root) {
            continue;
        }
        let mut work: Vec<(NodeKey, Vec<NodeKey>)> = Vec::new();
        index.insert(root, (counter, counter));
        counter = counter.saturating_add(1);
        stack.push(root);
        on_stack.insert(root);
        work.push((root, successors(bodies, &root)));
        while let Some((node, pending)) = work.last_mut() {
            let node = *node;
            if let Some(next) = pending.pop() {
                match index.get(&next) {
                    None => {
                        index.insert(next, (counter, counter));
                        counter = counter.saturating_add(1);
                        stack.push(next);
                        on_stack.insert(next);
                        work.push((next, successors(bodies, &next)));
                    }
                    Some((next_index, _)) if on_stack.contains(&next) => {
                        let next_index = *next_index;
                        if let Some(entry) = index.get_mut(&node) {
                            entry.1 = entry.1.min(next_index);
                        }
                    }
                    Some(_) => {}
                }
                continue;
            }
            work.pop();
            let (node_index, low) = index.get(&node).copied().unwrap_or_default();
            if let Some((parent, _)) = work.last() {
                if let Some(entry) = index.get_mut(parent) {
                    entry.1 = entry.1.min(low);
                }
            }
            if low == node_index {
                let mut component = Vec::new();
                while let Some(member) = stack.pop() {
                    on_stack.remove(&member);
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                component.sort();
                found.push(component);
            }
        }
    }
    found.sort();
    found
}

/// `[from, hop, ..., from]`: the shortest cycle from `from` whose first edge
/// goes to `hop`, within `members`.
fn cycle_through(
    bodies: &Bodies,
    members: &BTreeSet<NodeKey>,
    from: NodeKey,
    hop: NodeKey,
) -> Vec<NodeKey> {
    let mut previous: BTreeMap<NodeKey, NodeKey> = BTreeMap::new();
    let mut seen = BTreeSet::from([hop]);
    let mut queue = VecDeque::from([hop]);
    while let Some(current) = queue.pop_front() {
        if current == from {
            break;
        }
        for next in successors(bodies, &current) {
            if members.contains(&next) && seen.insert(next) {
                previous.insert(next, current);
                queue.push_back(next);
            }
        }
    }
    let mut back = vec![from];
    let mut current = from;
    while current != hop {
        match previous.get(&current) {
            Some(before) => {
                back.push(*before);
                current = *before;
            }
            None => break,
        }
    }
    back.push(from);
    back.reverse();
    back
}

/// A cycle through `member`, via its least in-component successor.
fn cycle_of(bodies: &Bodies, members: &BTreeSet<NodeKey>, member: NodeKey) -> Vec<NodeKey> {
    match successors(bodies, &member)
        .into_iter()
        .find(|next| members.contains(next))
    {
        Some(hop) => cycle_through(bodies, members, member, hop),
        None => vec![member, member],
    }
}

fn termination(
    declarations: &BTreeMap<NodeKey, FunctionDeclaration>,
    bodies: &Bodies,
) -> Result<BTreeMap<NodeKey, Termination>, FunctionRefusal> {
    let mut result = BTreeMap::new();
    for component in components(bodies) {
        let members: BTreeSet<NodeKey> = component.iter().copied().collect();
        let recursive = component.len() > 1
            || component.iter().any(|key| {
                bodies
                    .get(key)
                    .is_some_and(|(calls, _)| calls.iter().any(|call| call.callee == *key))
            });
        if !recursive {
            continue;
        }
        let mut measures = BTreeMap::new();
        for key in &component {
            let Some(declaration) = declarations.get(key) else {
                continue;
            };
            let Some(measure) = &declaration.measure else {
                return Err(refuse(
                    *key,
                    Vec::new(),
                    FunctionCause::MissingMeasure {
                        cycle: cycle_of(bodies, &members, *key),
                    },
                ));
            };
            for (position, element) in measure.iter().enumerate() {
                match measure_admission(*element, &declaration.parameters) {
                    MeasureAdmission::Admitted => {}
                    MeasureAdmission::WrongParameter => {
                        return Err(refuse(
                            *key,
                            Vec::new(),
                            FunctionCause::MeasureParameter(position),
                        ))
                    }
                    MeasureAdmission::NotNonnegative => {
                        return Err(refuse(
                            *key,
                            Vec::new(),
                            FunctionCause::MeasureNotNonnegative(position),
                        ))
                    }
                }
            }
            measures.insert(*key, measure);
        }
        let shape_differs = measures
            .values()
            .zip(measures.values().skip(1))
            .any(|(a, b)| {
                a.len() != b.len() || a.iter().zip(b.iter()).any(|(x, y)| !x.same_kind(*y))
            });
        if let (true, Some(first)) = (shape_differs, component.first()) {
            return Err(refuse(
                *first,
                Vec::new(),
                FunctionCause::MeasureShape {
                    cycle: cycle_of(bodies, &members, *first),
                },
            ));
        }
        for key in &component {
            let (Some(caller), Some((calls, _))) = (measures.get(key), bodies.get(key)) else {
                continue;
            };
            let mut edges = Vec::new();
            for call in calls.iter().filter(|call| members.contains(&call.callee)) {
                let Some(callee) = measures.get(&call.callee) else {
                    continue;
                };
                let relations = caller
                    .iter()
                    .zip(callee.iter())
                    .map(|(caller_element, callee_element)| {
                        relate(*caller_element, *callee_element, &call.arguments)
                    })
                    .collect();
                let obligation = DecreaseObligation {
                    call: Location {
                        function: *key,
                        path: call.path.clone(),
                    },
                    callee: call.callee,
                    relations,
                };
                if obligation.decreasing_position().is_none() {
                    return Err(FunctionRefusal {
                        location: obligation.call.clone(),
                        cause: FunctionCause::NonDecreasing {
                            cycle: cycle_through(bodies, &members, *key, call.callee),
                            obligation: Box::new(obligation),
                        },
                    });
                }
                edges.push(obligation);
            }
            result.insert(
                *key,
                Termination::Recursive {
                    component: component.clone(),
                    measure: (*caller).clone(),
                    edges,
                },
            );
        }
    }
    Ok(result)
}

enum MeasureAdmission {
    Admitted,
    WrongParameter,
    NotNonnegative,
}

fn measure_admission(
    element: MeasureElement,
    parameters: &[ParameterDeclaration],
) -> MeasureAdmission {
    let Some(parameter) = parameters.get(element.parameter()) else {
        return MeasureAdmission::WrongParameter;
    };
    match (element, &parameter.value_type) {
        (MeasureElement::Integer(_), ValueType::Integer) => match &parameter.interval {
            Some(interval) if !interval.lower().is_negative() => MeasureAdmission::Admitted,
            Some(_) | None => MeasureAdmission::NotNonnegative,
        },
        (MeasureElement::Cardinality(_), ValueType::Collection(..))
        | (MeasureElement::Containment(_), ValueType::Composite(_)) => MeasureAdmission::Admitted,
        (
            MeasureElement::Integer(_)
            | MeasureElement::Cardinality(_)
            | MeasureElement::Containment(_),
            _,
        ) => MeasureAdmission::WrongParameter,
    }
}

fn relate(
    caller: MeasureElement,
    callee: MeasureElement,
    arguments: &[ArgumentShape],
) -> ElementRelation {
    let parameter = caller.parameter();
    match (callee, arguments.get(callee.parameter())) {
        (_, Some(ArgumentShape::Path(path))) if path.is_parameter(parameter) => {
            ElementRelation::Equal
        }
        (MeasureElement::Integer(_), Some(ArgumentShape::Decrement(path)))
            if path.is_parameter(parameter) =>
        {
            ElementRelation::Less
        }
        (MeasureElement::Containment(_), Some(ArgumentShape::Path(path)))
            if path.root == Root::Parameter(parameter)
                && path
                    .projections
                    .iter()
                    .any(|projection| matches!(projection, Projection::Field(_))) =>
        {
            ElementRelation::Less
        }
        _ => ElementRelation::Unproved,
    }
}

// ---- evaluation --------------------------------------------------------------

/// One evaluation environment.
struct Frame {
    parameters: Rc<[Value]>,
    locals: Vec<Value>,
}

enum Work<'a> {
    Eval(&'a Expression, Rc<Frame>),
    Call(NodeKey, usize),
    Branch(&'a Expression, &'a Expression, Rc<Frame>),
    Bind(&'a Expression, Rc<Frame>),
    Arithmetic(ArithmeticOperator),
    Compare(IntegerComparison),
    Quotient(AdmittedIntegerDivision),
    Field(NodeKey),
    Present,
    Value,
}

/// A checked-program invariant failed at runtime. Unreachable for an admitted
/// [`CheckedFunctions`] value; reported by name rather than panicking.
fn broken<T>() -> Outcome<T> {
    Outcome::Refused(Refusal::CheckedInvariant)
}

impl CheckedFunctions {
    /// The checked function with this identity.
    pub fn function(&self, key: NodeKey) -> Option<&CheckedFunction> {
        self.functions.get(&key)
    }

    /// Every checked function in key order.
    pub fn functions(&self) -> impl Iterator<Item = &CheckedFunction> {
        self.functions.values()
    }

    /// Evaluate `key` on `arguments`. Every call, including this one, charges
    /// `function.call` before binding its arguments; a denied charge returns
    /// `incomplete`.
    pub fn evaluate(
        &self,
        key: NodeKey,
        arguments: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Outcome<Value>, CallRefusal> {
        let function = self
            .functions
            .get(&key)
            .ok_or(CallRefusal::UnknownFunction)?;
        if function.parameters().len() != arguments.len() {
            return Err(CallRefusal::Arity);
        }
        if let Some(index) = function
            .parameters()
            .iter()
            .zip(&arguments)
            .position(|(parameter, argument)| !parameter.admits(argument))
        {
            return Err(CallRefusal::ArgumentType(index));
        }
        Ok(self.run(key, arguments, meter))
    }

    fn run(&self, key: NodeKey, arguments: Vec<Value>, meter: &mut Meter) -> Outcome<Value> {
        let count = arguments.len();
        let mut values: Vec<Value> = arguments;
        let mut work = vec![Work::Call(key, count)];
        while let Some(item) = work.pop() {
            match item {
                Work::Call(callee, count) => {
                    let Some(function) = self.functions.get(&callee) else {
                        return broken();
                    };
                    if let Err(record) = meter.charge_call() {
                        return Outcome::Incomplete(record);
                    }
                    let start = values.len().saturating_sub(count);
                    let parameters: Rc<[Value]> = values.split_off(start).into();
                    let admitted = parameters.len() == function.parameters().len()
                        && function
                            .parameters()
                            .iter()
                            .zip(parameters.iter())
                            .all(|(parameter, value)| parameter.admits(value));
                    if !admitted {
                        return broken();
                    }
                    let frame = Rc::new(Frame {
                        parameters,
                        locals: Vec::new(),
                    });
                    work.push(Work::Eval(&function.declaration.body, frame));
                }
                Work::Eval(expression, frame) => match expression {
                    Expression::Parameter(index) => match frame.parameters.get(*index) {
                        Some(value) => values.push(value.clone()),
                        None => return broken(),
                    },
                    Expression::Local(index) => match frame.locals.get(*index) {
                        Some(value) => values.push(value.clone()),
                        None => return broken(),
                    },
                    Expression::Literal { value, .. } => values.push(value.clone()),
                    Expression::Effect(_) => return broken(),
                    Expression::Call { callee, arguments } => {
                        work.push(Work::Call(*callee, arguments.len()));
                        work.extend(
                            arguments
                                .iter()
                                .rev()
                                .map(|argument| Work::Eval(argument, Rc::clone(&frame))),
                        );
                    }
                    Expression::Let { value, body } => {
                        work.push(Work::Bind(body, Rc::clone(&frame)));
                        work.push(Work::Eval(value, frame));
                    }
                    Expression::If {
                        condition,
                        then,
                        otherwise,
                    } => {
                        work.push(Work::Branch(then, otherwise, Rc::clone(&frame)));
                        work.push(Work::Eval(condition, frame));
                    }
                    Expression::Arithmetic {
                        operator,
                        left,
                        right,
                    } => {
                        work.push(Work::Arithmetic(*operator));
                        work.push(Work::Eval(right, Rc::clone(&frame)));
                        work.push(Work::Eval(left, frame));
                    }
                    Expression::Compare {
                        operator,
                        left,
                        right,
                    } => {
                        work.push(Work::Compare(*operator));
                        work.push(Work::Eval(right, Rc::clone(&frame)));
                        work.push(Work::Eval(left, frame));
                    }
                    Expression::Quotient {
                        law,
                        dividend,
                        divisor,
                    } => {
                        work.push(Work::Quotient(*law));
                        work.push(Work::Eval(divisor, Rc::clone(&frame)));
                        work.push(Work::Eval(dividend, frame));
                    }
                    Expression::Field { record, field } => {
                        work.push(Work::Field(*field));
                        work.push(Work::Eval(record, frame));
                    }
                    Expression::Present(operand) => {
                        work.push(Work::Present);
                        work.push(Work::Eval(operand, frame));
                    }
                    Expression::Value(operand) => {
                        work.push(Work::Value);
                        work.push(Work::Eval(operand, frame));
                    }
                },
                Work::Branch(then, otherwise, frame) => match values.pop() {
                    Some(Value::Boolean(true)) => work.push(Work::Eval(then, frame)),
                    Some(Value::Boolean(false)) => work.push(Work::Eval(otherwise, frame)),
                    _ => return broken(),
                },
                Work::Bind(body, frame) => {
                    let Some(value) = values.pop() else {
                        return broken();
                    };
                    let mut locals = frame.locals.clone();
                    locals.push(value);
                    let extended = Rc::new(Frame {
                        parameters: Rc::clone(&frame.parameters),
                        locals,
                    });
                    work.push(Work::Eval(body, extended));
                }
                Work::Arithmetic(operator) => {
                    let Some((left, right)) = integers(&mut values) else {
                        return broken();
                    };
                    let result = match operator {
                        ArithmeticOperator::Add => left.add(&right),
                        ArithmeticOperator::Subtract => left.sub(&right),
                        ArithmeticOperator::Multiply => left.mul(&right),
                    };
                    values.push(Value::Integer(result));
                }
                Work::Compare(operator) => {
                    let Some((left, right)) = integers(&mut values) else {
                        return broken();
                    };
                    let result = match operator {
                        IntegerComparison::Less => left < right,
                        IntegerComparison::LessOrEqual => left <= right,
                        IntegerComparison::Greater => left > right,
                        IntegerComparison::GreaterOrEqual => left >= right,
                        IntegerComparison::Equal => left == right,
                        IntegerComparison::NotEqual => left != right,
                    };
                    values.push(Value::Boolean(result));
                }
                Work::Quotient(law) => {
                    let Some((dividend, divisor)) = integers(&mut values) else {
                        return broken();
                    };
                    match divide(
                        &law,
                        &dividend,
                        &divisor,
                        &IntegerDomain::Mathematical,
                        meter,
                    ) {
                        Outcome::Completed(pair) => {
                            values.push(Value::Integer(pair.quotient().clone()));
                        }
                        Outcome::Undefined(reason) => return Outcome::Undefined(reason),
                        Outcome::Refused(reason) => return Outcome::Refused(reason),
                        Outcome::Incomplete(record) => return Outcome::Incomplete(record),
                    }
                }
                Work::Field(field) => {
                    let Some(Value::Composite(composite)) = values.pop() else {
                        return broken();
                    };
                    let slot = self
                        .fields
                        .get(&(composite.declaration(), field))
                        .and_then(|position| composite.slots().get(*position));
                    match slot {
                        Some(FieldValue::Present(value)) => values.push(value.clone()),
                        Some(FieldValue::Absent | FieldValue::Null) | None => return broken(),
                    }
                }
                Work::Present => match values.pop() {
                    Some(Value::Option(option)) => {
                        values.push(Value::Boolean(option.payload().is_some()));
                    }
                    _ => return broken(),
                },
                Work::Value => match values.pop() {
                    Some(Value::Option(option)) => match option.payload() {
                        Some(payload) => values.push(payload.clone()),
                        None => return broken(),
                    },
                    _ => return broken(),
                },
            }
        }
        match (values.pop(), values.is_empty()) {
            (Some(value), true) => Outcome::Completed(value),
            _ => broken(),
        }
    }

    /// `key` as a unary [`ValueFunction`] for the FR-145 collection queries.
    pub fn value_function(&self, key: NodeKey) -> Result<CheckedValueFunction<'_>, CallRefusal> {
        let function = self
            .functions
            .get(&key)
            .ok_or(CallRefusal::UnknownFunction)?;
        match function.parameters() {
            [parameter] => Ok(CheckedValueFunction {
                functions: self,
                function,
                parameter: &parameter.value_type,
            }),
            _ => Err(CallRefusal::Arity),
        }
    }

    /// `key` as a binary [`FoldFunction`] whose first parameter is the
    /// accumulator and result type, with declared algebraic `properties`
    /// (SPEC-GAP(119-13)).
    pub fn fold_function(
        &self,
        key: NodeKey,
        properties: AlgebraicProperties,
    ) -> Result<CheckedFoldFunction<'_>, CallRefusal> {
        let function = self
            .functions
            .get(&key)
            .ok_or(CallRefusal::UnknownFunction)?;
        match function.parameters() {
            [accumulator, element] if &accumulator.value_type == function.result() => {
                Ok(CheckedFoldFunction {
                    functions: self,
                    function,
                    element: &element.value_type,
                    properties,
                })
            }
            [_, _] => Err(CallRefusal::ArgumentType(0)),
            _ => Err(CallRefusal::Arity),
        }
    }

    fn apply_checked(
        &self,
        function: &CheckedFunction,
        arguments: Vec<Value>,
        meter: &mut Meter,
    ) -> Outcome<Value> {
        match self.evaluate(function.key(), arguments, meter) {
            Ok(outcome) => outcome,
            Err(_) => Outcome::Refused(Refusal::FunctionArgumentOutsideType),
        }
    }
}

fn integers(values: &mut Vec<Value>) -> Option<(Integer, Integer)> {
    match (values.pop(), values.pop()) {
        (Some(Value::Integer(right)), Some(Value::Integer(left))) => Some((left, right)),
        _ => None,
    }
}

/// A checked unary function as a collection-query [`ValueFunction`].
#[derive(Clone, Copy, Debug)]
pub struct CheckedValueFunction<'a> {
    functions: &'a CheckedFunctions,
    function: &'a CheckedFunction,
    parameter: &'a ValueType,
}

impl ValueFunction for CheckedValueFunction<'_> {
    fn parameter_type(&self) -> &ValueType {
        self.parameter
    }

    fn result_type(&self) -> &ValueType {
        self.function.result()
    }

    fn apply(&self, argument: &Value, meter: &mut Meter) -> Outcome<Value> {
        self.functions
            .apply_checked(self.function, vec![argument.clone()], meter)
    }
}

/// A checked binary function as a collection-query [`FoldFunction`].
#[derive(Clone, Copy, Debug)]
pub struct CheckedFoldFunction<'a> {
    functions: &'a CheckedFunctions,
    function: &'a CheckedFunction,
    element: &'a ValueType,
    properties: AlgebraicProperties,
}

impl FoldFunction for CheckedFoldFunction<'_> {
    fn accumulator_type(&self) -> &ValueType {
        self.function.result()
    }

    fn element_type(&self) -> &ValueType {
        self.element
    }

    fn properties(&self) -> AlgebraicProperties {
        self.properties
    }

    fn apply(&self, accumulator: &Value, element: &Value, meter: &mut Meter) -> Outcome<Value> {
        self.functions.apply_checked(
            self.function,
            vec![accumulator.clone(), element.clone()],
            meter,
        )
    }
}
