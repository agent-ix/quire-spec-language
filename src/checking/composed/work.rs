// SPDX-License-Identifier: AGPL-3.0-only
//! Versioned charge-before-work contract for composed type admission.
//!
//! Declarations are visited in namespace order; each expression/control and each
//! binary arena-boundary probe costs one Expression. Constraint creation, solving,
//! model/export lookup and each visited/copied native type wrapper cost Constraints.
//! Native dependency visits cost Edges. Every copied/scanned string byte is Bytes.
//! Each retained node/binder/cause/obligation is Records. IR gcd normalization is
//! charged a fixed 128-step allowance before one bounded signed-64 invocation.
//! No proof work is performed. Depth checks precede type descent and syntax use.

use super::Site;

/// Accounting contract for this prototype type-admission pass.
pub const VERSION: &str = "composed-type-work/1";

/// Independently bounded kinds of type-admission work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dimension {
    /// Bytes of inspected/copied source, identity and type strings.
    Bytes,
    /// Original declarations entered by the pass.
    Declarations,
    /// Syntax visits and declaration-arena boundary probes.
    Expressions,
    /// Constraint, lookup, type-wrapper and provenance operations.
    Constraints,
    /// Original native dependency visits.
    Edges,
    /// Reserved work for existing bounded IR rational normalization.
    Normalization,
    /// Retained index, type, refusal and obligation records.
    Records,
    /// Maximum inspected syntax/type depth, rather than a cumulative counter.
    Depth,
}

/// Caller-lowered capacities; defaults are also the unraisable hard ceilings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    /// Maximum inspected/copied string bytes.
    pub bytes: usize,
    /// Maximum entered declarations.
    pub declarations: usize,
    /// Maximum syntax visits and boundary probes.
    pub expressions: usize,
    /// Maximum constraint, lookup, type-wrapper and provenance operations.
    pub constraints: usize,
    /// Maximum dependency visits.
    pub edges: usize,
    /// Maximum reserved IR normalization work.
    pub normalization: usize,
    /// Maximum retained records.
    pub records: usize,
    /// Maximum syntax/type depth.
    pub depth: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            bytes: 8_388_608,
            declarations: 10_000,
            expressions: 100_000,
            constraints: 1_000_000,
            edges: 100_000,
            normalization: 1_000_000,
            records: 200_000,
            depth: 64,
        }
    }
}
impl Limits {
    /// Clamp each capacity independently to its hard ceiling.
    pub fn bounded(self) -> Self {
        let hard = Self::default();
        Self {
            bytes: self.bytes.min(hard.bytes),
            declarations: self.declarations.min(hard.declarations),
            expressions: self.expressions.min(hard.expressions),
            constraints: self.constraints.min(hard.constraints),
            edges: self.edges.min(hard.edges),
            normalization: self.normalization.min(hard.normalization),
            records: self.records.min(hard.records),
            depth: self.depth.min(hard.depth),
        }
    }
}

/// Successful charges and observed depth; failed charges remain separate.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Usage {
    /// Inspected/copied string bytes.
    pub bytes: usize,
    /// Entered declarations.
    pub declarations: usize,
    /// Syntax visits and boundary probes.
    pub expressions: usize,
    /// Constraint, lookup, type-wrapper and provenance operations.
    pub constraints: usize,
    /// Dependency visits.
    pub edges: usize,
    /// Reserved IR normalization work.
    pub normalization: usize,
    /// Retained records.
    pub records: usize,
    /// Greatest successfully admitted depth.
    pub max_depth: usize,
}

/// The first unaffordable charge, retained without converting it into a type error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Exhaustion {
    /// Capacity that would have been exceeded.
    pub dimension: Dimension,
    /// Effective limit for that capacity.
    pub limit: usize,
    /// Successful prior usage, or prior maximum for depth.
    pub prior: usize,
    /// Unpaid increment, or requested depth for a depth check.
    pub requested: usize,
    /// Original declaration/expression locus where work stopped.
    pub site: Site,
}

/// A fresh charge-before-work meter for one type-admission attempt.
#[derive(Debug)]
pub struct Work {
    limits: Limits,
    usage: Usage,
}
impl Work {
    /// Start with zero usage and independently clamped caller limits.
    pub fn new(limits: Limits) -> Self {
        Self {
            limits: limits.bounded(),
            usage: Usage::default(),
        }
    }
    /// Effective capacities for this attempt.
    pub fn limits(&self) -> Limits {
        self.limits
    }
    /// Successfully charged work so far.
    pub fn usage(&self) -> Usage {
        self.usage
    }
    /// Reserve an increment before work, or check the requested maximum depth.
    /// Overflow and exceeded capacities leave the prior usage unchanged.
    pub fn charge(
        &mut self,
        dimension: Dimension,
        amount: usize,
        site: Site,
    ) -> Result<(), Exhaustion> {
        let (used, limit) = match dimension {
            Dimension::Bytes => (&mut self.usage.bytes, self.limits.bytes),
            Dimension::Declarations => (&mut self.usage.declarations, self.limits.declarations),
            Dimension::Expressions => (&mut self.usage.expressions, self.limits.expressions),
            Dimension::Constraints => (&mut self.usage.constraints, self.limits.constraints),
            Dimension::Edges => (&mut self.usage.edges, self.limits.edges),
            Dimension::Normalization => (&mut self.usage.normalization, self.limits.normalization),
            Dimension::Records => (&mut self.usage.records, self.limits.records),
            Dimension::Depth => {
                if amount > self.limits.depth {
                    return Err(Exhaustion {
                        dimension,
                        limit: self.limits.depth,
                        prior: self.usage.max_depth,
                        requested: amount,
                        site,
                    });
                }
                self.usage.max_depth = self.usage.max_depth.max(amount);
                return Ok(());
            }
        };
        let next = used
            .checked_add(amount)
            .filter(|next| *next <= limit)
            .ok_or(Exhaustion {
                dimension,
                limit,
                prior: *used,
                requested: amount,
                site,
            })?;
        *used = next;
        Ok(())
    }
}
