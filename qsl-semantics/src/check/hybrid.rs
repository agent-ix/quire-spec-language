// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-447: the S3 check of a `hybrid` declaration into a
//! [`CheckedHybridModel`] (QSpec FR-193 "Source form and the clock line",
//! "Initial states", "Continuous values and expressions", "Resets").
//!
//! The checked model is the one form the request writer and an in-process
//! hybrid provider read, and the form S4 emits as QSpec FR-481's `hybrid`
//! nodes; none re-reads source. It records each mode's rate of each
//! continuous variable and classifies each variable as a clock or as hybrid
//! dynamics, which is ADR-026 HY-1's line between EN-6 and an external
//! solver.

use quire_exact::{Identifier, Rational, ValueType};
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};

use super::profile::TemporalProfile;

mod callee;
mod cause;
mod claim;
mod model;
mod term;

pub(super) use callee::Callees;
pub use cause::{HybridCause, MalformedReason};
pub(super) use claim::{check_claims, ClaimInputs};
pub use claim::{hybrid_horizon, HorizonOutOfRange, HybridClaimDeclaration};
pub(super) use model::check_models;
pub use model::HybridDeclaration;

/// A mode of a [`CheckedHybridModel`], by its position in
/// [`CheckedHybridModel::modes`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModeIndex(usize);

impl ModeIndex {
    /// The mode at position `index`.
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    /// The position in [`CheckedHybridModel::modes`].
    pub fn get(self) -> usize {
        self.0
    }
}

/// A continuous variable of a [`CheckedHybridModel`], by its position in
/// [`CheckedHybridModel::variables`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VariableIndex(usize);

impl VariableIndex {
    /// The variable at position `index`.
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    /// The position in [`CheckedHybridModel::variables`].
    pub fn get(self) -> usize {
        self.0
    }
}

/// A transition of a [`CheckedMode`], by its position in
/// [`CheckedMode::transitions`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TransitionIndex(usize);

impl TransitionIndex {
    /// The transition at position `index`.
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    /// The position in [`CheckedMode::transitions`].
    pub fn get(self) -> usize {
        self.0
    }
}

/// A hybrid model of the package, by its position in
/// `CheckedGraph::hybrid_models`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HybridModelIndex(usize);

impl HybridModelIndex {
    /// The model at position `index`.
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    /// The position in `CheckedGraph::hybrid_models`.
    pub fn get(self) -> usize {
        self.0
    }
}

/// A pure function a term calls, by its position in the package's function
/// list (FR-062). S4 emits the call as an application of that function's
/// node.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CheckedFunctionRef(usize);

impl CheckedFunctionRef {
    /// The function at position `index`.
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    /// The position in the package's function list.
    pub fn get(self) -> usize {
        self.0
    }
}

/// A capture of a hybrid claim, by its position in the claim's captures.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CaptureIndex(usize);

impl CaptureIndex {
    /// The capture at position `index`.
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    /// The position in the claim's captures.
    pub fn get(self) -> usize {
        self.0
    }
}

/// One checked `hybrid` declaration (FR-447 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedHybridModel {
    /// The declared name.
    pub name: Identifier,
    /// The continuous variables, in order of first appearance as an
    /// equation's left side.
    pub variables: Vec<ContinuousVariable>,
    /// The initial entries, in source order;
    /// `[InitialEntry { mode: 0, condition: Boolean(true) }]` when the
    /// declaration writes no `initial` clause.
    pub initial: Vec<InitialEntry>,
    /// The modes, in declared order.
    pub modes: Vec<CheckedMode>,
}

/// One initial entry, `mode M when { c };`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitialEntry {
    /// The mode the entry names.
    pub mode: ModeIndex,
    /// The Boolean condition over the continuous variables.
    pub condition: HybridTerm,
}

/// One continuous variable and its class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContinuousVariable {
    /// The variable's name.
    pub name: Identifier,
    /// Its class (ADR-026 HY-1).
    pub class: VariableClass,
}

/// Whether a continuous variable is a clock or hybrid dynamics.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VariableClass {
    /// The rate is the numeric literal `1` in every mode.
    Clock,
    /// Any other variable.
    Hybrid,
}

/// One checked mode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedMode {
    /// The mode's name.
    pub name: Identifier,
    /// The Boolean invariant.
    pub invariant: HybridTerm,
    /// `rates[i]` is the rate of `variables[i]` in this mode.
    pub rates: Vec<HybridTerm>,
    /// The transitions, in declared order.
    pub transitions: Vec<CheckedTransition>,
}

/// One checked transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedTransition {
    /// The mode the transition jumps to.
    pub target: ModeIndex,
    /// The Boolean guard.
    pub guard: HybridTerm,
    /// The reset's conjuncts, in declared order; empty for `reset { true }`.
    pub resets: Vec<ResetConstraint>,
}

/// One reset conjunct `x ~ e`: the post-jump `x` against `e` over the
/// pre-jump state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResetConstraint {
    /// The variable whose post-jump value the conjunct constrains.
    pub variable: VariableIndex,
    /// The relation `~`.
    pub relation: CompareOp,
    /// The right side, read over the pre-jump state.
    pub value: HybridTerm,
}

/// A comparison operator.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CompareOp {
    /// `=`.
    Eq,
    /// `!=`.
    Ne,
    /// `<`.
    Lt,
    /// `<=`.
    Le,
    /// `>=`.
    Ge,
    /// `>`.
    Gt,
}

/// An arithmetic operator over the reals.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ArithmeticOp {
    /// `+`.
    Add,
    /// `-`.
    Sub,
    /// `*`.
    Mul,
    /// `/`.
    Div,
}

/// A binary Boolean connective.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LogicOp {
    /// `and`.
    And,
    /// `or`.
    Or,
    /// `implies`.
    Implies,
}

/// A checked expression of a hybrid declaration or claim, over the reals and
/// Booleans. S3 folds no constant and expands no call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HybridTerm {
    /// A continuous variable at the current position; in a reset, its
    /// pre-jump value.
    Variable(VariableIndex),
    /// `q.x` of an `on each (q: D)` activation: the value of `x` at the
    /// activation position.
    ActivationVariable(VariableIndex),
    /// A capture's value, read throughout its obligation.
    Capture(CaptureIndex),
    /// A numeric constant, exact.
    Constant(Rational),
    /// `true` or `false`.
    Boolean(bool),
    /// Unary `-`.
    Negate(Box<HybridTerm>),
    /// `+`, `-`, `*` or `/`.
    Arithmetic(ArithmeticOp, Box<HybridTerm>, Box<HybridTerm>),
    /// `=`, `!=`, `<`, `<=`, `>=` or `>`.
    Compare(CompareOp, Box<HybridTerm>, Box<HybridTerm>),
    /// `not`.
    Not(Box<HybridTerm>),
    /// `and`, `or` or `implies`.
    Logic(LogicOp, Box<HybridTerm>, Box<HybridTerm>),
    /// `if c then a else b`.
    Conditional(Box<HybridTerm>, Box<HybridTerm>, Box<HybridTerm>),
    /// A call of a pure function whose parameters and result are `Boolean`
    /// or numeric, arguments in order. S3 expands no call.
    Call(CheckedFunctionRef, Vec<HybridTerm>),
}

/// One checked hybrid claim: a `temporal` clause whose `over` parameter is
/// typed by a `hybrid` declaration (FR-447 "Hybrid claims").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedHybridClaim {
    /// The model the `over` parameter's type names.
    pub model: HybridModelIndex,
    /// The temporal profile the claim's `using` alias selects.
    pub profile: TemporalProfile,
    /// The `over` parameter's name.
    pub over: Identifier,
    /// The clock the claim is stepped by.
    pub clock: HybridClock,
    /// The activation.
    pub activation: HybridActivation,
    /// The captures, in source order.
    pub captures: Vec<HybridCapture>,
    /// The fairness constraints, in source order of first occurrence.
    pub fairness: Vec<HybridFairness>,
    /// The formula.
    pub formula: HybridFormula,
    /// The reach of the formula; `None` for an unbounded claim.
    pub horizon: Option<HybridHorizon>,
}

/// The clock a hybrid claim is bound to (QSpec FR-252).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HybridClock {
    /// `model-time`: positions are instants, intervals count seconds.
    ModelTime,
    /// `model-steps`: positions are the start state and each jump's state,
    /// intervals count positions.
    ModelSteps,
}

/// A hybrid claim's activation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HybridActivation {
    /// `on origin`: one obligation, at the first position.
    Origin,
    /// `on each (q: D) [when (g)]`: one obligation at every position, or at
    /// every position where the guard holds.
    Each {
        /// The `on each` parameter's name.
        parameter: Identifier,
        /// The guard, when written.
        guard: Option<HybridTerm>,
    },
}

/// One checked capture, `capture n: T = e;`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridCapture {
    /// The capture's name.
    pub name: Identifier,
    /// Its declared type: `Boolean` or numeric.
    pub ty: ValueType,
    /// The initializer, evaluated once at the activation position.
    pub value: HybridTerm,
}

/// One checked fairness constraint over hybrid transitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridFairness {
    /// Weak or strong.
    pub kind: FairnessKind,
    /// One constraint over the named transitions, or one per transition.
    pub granularity: FairnessGranularity,
    /// The named transitions, in source order.
    pub transitions: Vec<(ModeIndex, TransitionIndex)>,
}

/// The reach of a claim's formula (FR-447 "Horizon").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HybridHorizon {
    /// Seconds after the activation position, under `model-time`.
    Seconds(Rational),
    /// Positions after the activation position, under `model-steps`.
    Positions(u64),
}

/// A hybrid claim's formula.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HybridFormula {
    /// `holds(e)`, and `true` and `false`.
    Holds(HybridTerm),
    /// `not`.
    Not(Box<HybridFormula>),
    /// `and`, `or` or `implies`.
    Logic(LogicOp, Box<HybridFormula>, Box<HybridFormula>),
    /// `always`.
    Always(Option<HybridInterval>, Box<HybridFormula>),
    /// `eventually`.
    Eventually(Option<HybridInterval>, Box<HybridFormula>),
    /// `until`.
    Until(
        Option<HybridInterval>,
        Box<HybridFormula>,
        Box<HybridFormula>,
    ),
    /// `release`.
    Release(
        Option<HybridInterval>,
        Box<HybridFormula>,
        Box<HybridFormula>,
    ),
    /// `historically`.
    Historically(Option<HybridInterval>, Box<HybridFormula>),
    /// `once`.
    Once(Option<HybridInterval>, Box<HybridFormula>),
    /// `since`.
    Since(
        Option<HybridInterval>,
        Box<HybridFormula>,
        Box<HybridFormula>,
    ),
    /// `triggered`.
    Triggered(
        Option<HybridInterval>,
        Box<HybridFormula>,
        Box<HybridFormula>,
    ),
}

/// An operator's interval: seconds under `model-time`, positions under
/// `model-steps`, every bound an exact rational.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridInterval {
    /// The lower bound.
    pub lower: Rational,
    /// Whether the lower end is open.
    pub lower_open: bool,
    /// The upper bound; `None` for `*`.
    pub upper: Option<Rational>,
    /// Whether the upper end is open.
    pub upper_open: bool,
}

#[cfg(test)]
mod tests;
