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

use quire_exact::{Identifier, Rational};

mod cause;
mod model;
mod term;

pub use cause::{HybridCause, MalformedReason};
pub use model::HybridDeclaration;
pub(super) use model::{check_models, not_emitted};

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
}

#[cfg(test)]
mod tests;
