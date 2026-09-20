// SPDX-License-Identifier: AGPL-3.0-or-later
//! NFR-009: charge-before-work accounting for one composed evaluation.

use crate::protocol_artifact::wire::Locus;

/// Version interpreting every composed-evaluation counter.
pub const ACCOUNTING_VERSION: &str = "quire.state.evaluation-work/1";

/// Inclusive caller-lowered ceilings; values above these defaults are clamped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    /// Maximum number of input value nodes the evaluation may charge.
    pub input_value_nodes: usize,
    /// Maximum number of aggregate (population, record or sequence) entries
    /// the evaluation may charge.
    pub input_aggregate_entries: usize,
    /// Maximum cumulative length of input text the evaluation may charge, in bytes.
    pub input_text_bytes: usize,
    /// Maximum nesting depth of supplied input values.
    pub input_structural_depth: usize,
    /// Maximum cumulative expression evaluation work.
    pub expression_work: usize,
    /// Maximum simultaneously active expression call depth.
    pub active_expression_depth: usize,
    /// Maximum simultaneously active predicate call depth.
    pub predicate_call_depth: usize,
    /// Maximum cumulative work spent iterating sequences.
    pub sequence_work: usize,
    /// Maximum cumulative number of output value nodes retained.
    pub retained_output: usize,
    /// Maximum cumulative work spent expanding graph traversals.
    pub graph_expansion: usize,
    /// Maximum cumulative number of graph edges traversed.
    pub graph_edges: usize,
    /// Maximum simultaneously active graph traversal depth.
    pub active_graph_depth: usize,
    /// Maximum cumulative work spent comparing values.
    pub value_comparison: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            input_value_nodes: 100_000,
            input_aggregate_entries: 100_000,
            input_text_bytes: 8_388_608,
            input_structural_depth: 64,
            expression_work: 1_000_000,
            active_expression_depth: 64,
            predicate_call_depth: 64,
            sequence_work: 1_000_000,
            retained_output: 100_000,
            graph_expansion: 10_000,
            graph_edges: 100_000,
            active_graph_depth: 64,
            value_comparison: 100_000,
        }
    }
}

impl Limits {
    /// Clamp every component independently to its hard ceiling, preserving zero.
    pub fn bounded(mut self) -> Self {
        let hard = Self::default();
        macro_rules! clamp {
            ($($field:ident),* $(,)?) => { $(self.$field = self.$field.min(hard.$field);)* };
        }
        clamp!(
            input_value_nodes,
            input_aggregate_entries,
            input_text_bytes,
            input_structural_depth,
            expression_work,
            active_expression_depth,
            predicate_call_depth,
            sequence_work,
            retained_output,
            graph_expansion,
            graph_edges,
            active_graph_depth,
            value_comparison
        );
        self
    }
}

/// Successfully charged cumulative work and greatest active depths.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Usage {
    /// Input value nodes charged so far.
    pub input_value_nodes: usize,
    /// Aggregate (population, record or sequence) entries charged so far.
    pub input_aggregate_entries: usize,
    /// Input text bytes charged so far.
    pub input_text_bytes: usize,
    /// Greatest input structural depth reached so far.
    pub input_structural_depth: usize,
    /// Expression evaluation work charged so far.
    pub expression_work: usize,
    /// Greatest active expression call depth reached so far.
    pub active_expression_depth: usize,
    /// Greatest active predicate call depth reached so far.
    pub predicate_call_depth: usize,
    /// Sequence iteration work charged so far.
    pub sequence_work: usize,
    /// Output value nodes retained so far.
    pub retained_output: usize,
    /// Graph expansion work charged so far.
    pub graph_expansion: usize,
    /// Graph edges traversed so far.
    pub graph_edges: usize,
    /// Greatest active graph traversal depth reached so far.
    pub active_graph_depth: usize,
    /// Value comparison work charged so far.
    pub value_comparison: usize,
}

/// Independently limited evaluation resource.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dimension {
    /// Count of input value nodes.
    InputValueNodes,
    /// Count of aggregate (population, record or sequence) entries.
    InputAggregateEntries,
    /// Cumulative length of input text, in bytes.
    InputTextBytes,
    /// Peak nesting depth of supplied input values.
    InputStructuralDepth,
    /// Cumulative expression evaluation work.
    ExpressionWork,
    /// Peak simultaneously active expression call depth.
    ActiveExpressionDepth,
    /// Peak simultaneously active predicate call depth.
    PredicateCallDepth,
    /// Cumulative work spent iterating sequences.
    SequenceWork,
    /// Count of retained output value nodes.
    RetainedOutput,
    /// Cumulative work spent expanding graph traversals.
    GraphExpansion,
    /// Count of graph edges traversed.
    GraphEdges,
    /// Peak simultaneously active graph traversal depth.
    ActiveGraphDepth,
    /// Cumulative work spent comparing values.
    ValueComparison,
}

/// Why the next bounded operation could not be performed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExhaustionCause {
    /// The charge would exceed the dimension's configured limit.
    Limit,
    /// The charge would overflow the dimension's own counter.
    CounterOverflow,
    /// The charge could not be satisfied because reserving memory for it failed.
    Allocation,
}

/// Exact refused next charge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Exhaustion {
    /// Which resource dimension was exhausted.
    pub dimension: Dimension,
    /// Exact reason the charge was refused.
    pub cause: ExhaustionCause,
    /// Amount already charged against this dimension before the refused charge.
    pub used: usize,
    /// Amount the refused charge would have added.
    pub requested: usize,
    /// Effective limit this dimension was bounded by.
    pub limit: usize,
    /// Source location the charge was attributed to, when one was set.
    pub locus: Option<Locus>,
}

pub(super) struct Work {
    pub limits: Limits,
    pub usage: Usage,
    pub locus: Option<Locus>,
}

impl Work {
    pub fn new(limits: Limits) -> Self {
        Self {
            limits: limits.bounded(),
            usage: Usage::default(),
            locus: None,
        }
    }

    fn fields(&mut self, dimension: Dimension) -> (&mut usize, usize, bool) {
        use Dimension as D;
        match dimension {
            D::InputValueNodes => (
                &mut self.usage.input_value_nodes,
                self.limits.input_value_nodes,
                false,
            ),
            D::InputAggregateEntries => (
                &mut self.usage.input_aggregate_entries,
                self.limits.input_aggregate_entries,
                false,
            ),
            D::InputTextBytes => (
                &mut self.usage.input_text_bytes,
                self.limits.input_text_bytes,
                false,
            ),
            D::InputStructuralDepth => (
                &mut self.usage.input_structural_depth,
                self.limits.input_structural_depth,
                true,
            ),
            D::ExpressionWork => (
                &mut self.usage.expression_work,
                self.limits.expression_work,
                false,
            ),
            D::ActiveExpressionDepth => (
                &mut self.usage.active_expression_depth,
                self.limits.active_expression_depth,
                true,
            ),
            D::PredicateCallDepth => (
                &mut self.usage.predicate_call_depth,
                self.limits.predicate_call_depth,
                true,
            ),
            D::SequenceWork => (
                &mut self.usage.sequence_work,
                self.limits.sequence_work,
                false,
            ),
            D::RetainedOutput => (
                &mut self.usage.retained_output,
                self.limits.retained_output,
                false,
            ),
            D::GraphExpansion => (
                &mut self.usage.graph_expansion,
                self.limits.graph_expansion,
                false,
            ),
            D::GraphEdges => (&mut self.usage.graph_edges, self.limits.graph_edges, false),
            D::ActiveGraphDepth => (
                &mut self.usage.active_graph_depth,
                self.limits.active_graph_depth,
                true,
            ),
            D::ValueComparison => (
                &mut self.usage.value_comparison,
                self.limits.value_comparison,
                false,
            ),
        }
    }

    pub fn charge(&mut self, dimension: Dimension, requested: usize) -> Result<(), Exhaustion> {
        let locus = self.locus.clone();
        let (used, limit, peak) = self.fields(dimension);
        let next = if peak {
            Some((*used).max(requested))
        } else {
            used.checked_add(requested)
        };
        let cause = if next.is_none() {
            ExhaustionCause::CounterOverflow
        } else {
            ExhaustionCause::Limit
        };
        if let Some(next) = next.filter(|next| *next <= limit) {
            *used = next;
            Ok(())
        } else {
            Err(Exhaustion {
                dimension,
                cause,
                used: *used,
                requested,
                limit,
                locus,
            })
        }
    }

    pub fn allocation(&self, dimension: Dimension, requested: usize) -> Exhaustion {
        let (used, limit) = match dimension {
            Dimension::InputAggregateEntries => (
                self.usage.input_aggregate_entries,
                self.limits.input_aggregate_entries,
            ),
            Dimension::RetainedOutput => (self.usage.retained_output, self.limits.retained_output),
            Dimension::GraphExpansion => (self.usage.graph_expansion, self.limits.graph_expansion),
            other => {
                let _ = other;
                (0, 0)
            }
        };
        Exhaustion {
            dimension,
            cause: ExhaustionCause::Allocation,
            used,
            requested,
            limit,
            locus: self.locus.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Dimension, ExhaustionCause, Limits, Work};
    use ix_trace_rs::trace;

    #[test]
    #[trace("TC-137", "NFR-009-AC-3")]
    fn counter_overflow_and_allocation_are_closed_typed_causes() {
        let mut work = Work::new(Limits::default());
        work.usage.value_comparison = usize::MAX;
        let overflow = work
            .charge(Dimension::ValueComparison, 1)
            .expect_err("checked counter overflow");
        assert_eq!(overflow.dimension, Dimension::ValueComparison);
        assert_eq!(overflow.cause, ExhaustionCause::CounterOverflow);
        assert_eq!(overflow.used, usize::MAX);
        assert_eq!(overflow.requested, 1);
        assert_eq!(overflow.limit, Limits::default().value_comparison);

        for dimension in [
            Dimension::InputAggregateEntries,
            Dimension::RetainedOutput,
            Dimension::GraphExpansion,
        ] {
            let allocation = work.allocation(dimension, 1);
            assert_eq!(allocation.dimension, dimension);
            assert_eq!(allocation.cause, ExhaustionCause::Allocation);
            assert_eq!(allocation.requested, 1);
        }
    }
}
