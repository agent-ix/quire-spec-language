// SPDX-License-Identifier: AGPL-3.0-or-later
//! NFR-008: invocation-local ceilings for bounded temporal evaluation.
//!
//! The counters mirror the compiled-protocol accounting contract's shape so a
//! caller can reason about both boundaries the same way, but the dimensions and
//! ceilings are the evaluator's own. An exhausted dimension is a typed stop; it
//! is never reported as a temporal Boolean.

use super::result::Subject;

/// Counter contract, independent of the artifact reader and parser budgets.
pub const ACCOUNTING_VERSION: &str = "quire.native.temporal-work/1";

/// Independently lowered ceilings. Values above the defaults are clamped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    /// Admitted trace positions inspected, at most 1,000,000.
    pub positions: usize,
    /// Atomic valuation lookups, at most 1,000,000.
    pub valuations: usize,
    /// Concurrently active obligation instances, at most 10,000.
    pub instances: usize,
    /// Retained capture records across all instances, at most 100,000.
    pub captures: usize,
    /// Retained valuation records an unsettled obligation still requires,
    /// at most 1,000,000.
    pub retention: usize,
    /// Temporal graph node visits, at most 1,000,000.
    pub visits: usize,
    /// Temporal formula nesting depth, at most 64.
    pub depth: usize,
    /// Largest admitted interval bound, at most `i64::MAX as usize`.
    pub horizon: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            positions: 1_000_000,
            valuations: 1_000_000,
            instances: 10_000,
            captures: 100_000,
            retention: 1_000_000,
            visits: 1_000_000,
            depth: 64,
            horizon: i64::MAX as usize,
        }
    }
}

impl Limits {
    /// Effective ceilings, preserving zero and each caller-lowered component.
    pub fn bounded(mut self) -> Self {
        let hard = Self::default();
        macro_rules! clamp {
            ($($field:ident),* $(,)?) => { $(self.$field = self.$field.min(hard.$field);)* };
        }
        clamp!(positions, valuations, instances, captures, retention, visits, depth, horizon);
        self
    }
}

/// Successful work. `instances`, `retention`, `depth` and `horizon` are peak
/// counters; `positions`, `valuations`, `captures` and `visits` are cumulative.
/// A refused charge never increases usage.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Usage {
    pub positions: usize,
    pub valuations: usize,
    /// Greatest number of instances active at one time.
    pub instances: usize,
    pub captures: usize,
    /// Greatest number of retained valuations required at one time.
    pub retention: usize,
    pub visits: usize,
    /// Greatest checked formula depth.
    pub depth: usize,
    /// Greatest checked interval bound.
    pub horizon: usize,
}

/// The exact exhausted resource, never inferred from a diagnostic string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dimension {
    Positions,
    Valuations,
    Instances,
    Captures,
    Retention,
    Visits,
    Depth,
    Horizon,
}

/// A refused next step; successful prior usage is retained unchanged.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("temporal {dimension:?} limit {limit}: requested {requested} after {used}")]
pub struct Exhaustion {
    /// Resource whose next step was refused.
    pub dimension: Dimension,
    /// Successful usage before that step.
    pub used: usize,
    /// Requested increment, or candidate size for a peak dimension.
    pub requested: usize,
    /// Effective caller ceiling for that resource.
    pub limit: usize,
    /// Obligation, position and node the refused step belonged to.
    pub subject: Subject,
}

pub(super) struct Work {
    pub limits: Limits,
    pub usage: Usage,
    pub subject: Subject,
}

impl Work {
    pub fn new(limits: Limits) -> Self {
        Self {
            limits: limits.bounded(),
            usage: Usage::default(),
            subject: Subject::default(),
        }
    }

    fn fields(&mut self, dimension: Dimension) -> (&mut usize, usize, bool) {
        use Dimension as D;
        match dimension {
            D::Positions => (&mut self.usage.positions, self.limits.positions, false),
            D::Valuations => (&mut self.usage.valuations, self.limits.valuations, false),
            D::Instances => (&mut self.usage.instances, self.limits.instances, true),
            D::Captures => (&mut self.usage.captures, self.limits.captures, false),
            D::Retention => (&mut self.usage.retention, self.limits.retention, true),
            D::Visits => (&mut self.usage.visits, self.limits.visits, false),
            D::Depth => (&mut self.usage.depth, self.limits.depth, true),
            D::Horizon => (&mut self.usage.horizon, self.limits.horizon, true),
        }
    }

    /// Charge one step. Peak dimensions record the candidate; cumulative
    /// dimensions add it with checked arithmetic. Overflow refuses rather than
    /// wrapping or saturating.
    pub fn charge(&mut self, dimension: Dimension, requested: usize) -> Result<(), Exhaustion> {
        let subject = self.subject;
        let (used, limit, peak) = self.fields(dimension);
        let next = if peak {
            Some((*used).max(requested))
        } else {
            used.checked_add(requested)
        };
        if let Some(next) = next.filter(|next| *next <= limit) {
            *used = next;
            Ok(())
        } else {
            let used = *used;
            Err(Exhaustion {
                dimension,
                used,
                requested,
                limit,
                subject,
            })
        }
    }

    pub fn visit(&mut self) -> Result<(), Exhaustion> {
        self.charge(Dimension::Visits, 1)
    }
}
