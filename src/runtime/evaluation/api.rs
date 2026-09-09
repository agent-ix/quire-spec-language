// SPDX-License-Identifier: AGPL-3.0-only
//! FR-008: borrowed, source-bound reference observations and explicit limits.

use super::super::ValidatedContext;
use crate::syntax::ExprId;
use crate::{Diagnostic, Span};

/// Inclusive caller-lowered ceilings, clamped separately to their defaults.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvaluationLimits {
    /// Entered non-Group AST nodes, at most 1,000,000.
    pub expression_steps: usize,
    /// New object expansions across reaches calls, at most 10,000.
    pub graph_steps: usize,
    /// Inspected value pairs, including record roots, at most 100,000.
    pub comparisons: usize,
    /// Text scalar iterator advances including end checks, at most 1 MiB.
    pub text_steps: usize,
    /// Stored implication observations, at most 10,000.
    pub events: usize,
    /// Active expression or comparison depth, separately at most 64.
    pub depth: usize,
}

impl Default for EvaluationLimits {
    fn default() -> Self {
        Self {
            expression_steps: 1_000_000,
            graph_steps: 10_000,
            comparisons: 100_000,
            text_steps: 1_048_576,
            events: 10_000,
            depth: 64,
        }
    }
}

impl EvaluationLimits {
    pub(super) fn bounded(self) -> Self {
        let hard = Self::default();
        Self {
            expression_steps: self.expression_steps.min(hard.expression_steps),
            graph_steps: self.graph_steps.min(hard.graph_steps),
            comparisons: self.comparisons.min(hard.comparisons),
            text_steps: self.text_steps.min(hard.text_steps),
            events: self.events.min(hard.events),
            depth: self.depth.min(hard.depth),
        }
    }
}

/// Work admitted during this call; earlier runs and validation contribute nothing.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EvaluationUsage {
    /// Entered non-Group expression nodes.
    pub expression_steps: usize,
    /// Admitted new object expansions.
    pub graph_steps: usize,
    /// Admitted value-pair inspections.
    pub comparisons: usize,
    /// Admitted per-side scalar iterator advances.
    pub text_steps: usize,
    /// Retained implication observations.
    pub events: usize,
    /// Greatest active non-Group expression depth.
    pub expression_depth: usize,
    /// Greatest active value comparison depth, independent of expression depth.
    pub comparison_depth: usize,
}

/// Actual implication operand observation; entry alone does not assert completion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImplicationEventKind {
    /// Began evaluating the antecedent.
    AntecedentEntered,
    /// Finished the antecedent with its actual truth value.
    AntecedentCompleted(bool),
    /// Began evaluating the consequent after a true antecedent.
    ConsequentEntered,
}

/// Original syntax lineage within the report's exact retained context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImplicationEvent {
    /// Original implication AST handle.
    pub implication: ExprId,
    /// Original operand handle, including any Group node.
    pub operand: ExprId,
    /// Original operand byte region, including grouping delimiters.
    pub span: Span,
    /// Actual observation at this point in execution.
    pub kind: ImplicationEventKind,
}

/// A concrete Boolean exists only after complete execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvaluationOutcome {
    /// The selected predicate finished with this truth value.
    Completed(bool),
    /// Cancellation or an explicit implementation ceiling stopped execution.
    Incomplete(Box<Diagnostic>),
    /// A defensive established-invariant check failed; this is not predicate false.
    Refused(Box<Diagnostic>),
}

/// Immutable execution result borrowing exactly the validated context it observed.
#[derive(Debug)]
pub struct EvaluationReport<'context, 'checked, 'model> {
    pub(super) context: &'context ValidatedContext<'checked, 'model>,
    pub(super) outcome: EvaluationOutcome,
    pub(super) usage: EvaluationUsage,
    pub(super) events: Vec<ImplicationEvent>,
}

impl<'context, 'checked, 'model> EvaluationReport<'context, 'checked, 'model> {
    /// Move measured observations out without retaining a borrow of a local context.
    pub(in crate::runtime) fn into_parts(
        self,
    ) -> (EvaluationOutcome, EvaluationUsage, Vec<ImplicationEvent>) {
        (self.outcome, self.usage, self.events)
    }

    /// Exact input, authored clause, checked model and original source correspondence.
    pub fn context(&self) -> &'context ValidatedContext<'checked, 'model> {
        self.context
    }
    /// Actual truth or classified stop, without a success value on failed work.
    pub fn outcome(&self) -> &EvaluationOutcome {
        &self.outcome
    }
    /// Actual admitted work for this call.
    pub fn usage(&self) -> EvaluationUsage {
        self.usage
    }
    /// Actual ordered event prefix; no events are synthesized for skipped work.
    pub fn events(&self) -> &[ImplicationEvent] {
        &self.events
    }
    /// Reference accounting version, separate from language and backend fuel.
    pub fn cost_model(&self) -> &'static str {
        "native-ref-cost/1-draft"
    }
}
