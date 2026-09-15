// SPDX-License-Identifier: AGPL-3.0-or-later
//! Versioned deterministic charge-before-work contract for guarded definedness.
//!
//! Correspondence is admitted before namespace-ordered declarations. Original
//! value/root visits cost Expressions; type descent/lookups cost Types. Each
//! dependency occurrence costs Edges at insertion and at settled-target delivery.
//! Values, graph nodes, inspected/copied facts, goals and retained records charge
//! their named counters before creation. Every copied/scanned string byte costs
//! Bytes, including IR symbol/source metadata. Each materialized IR node costs
//! Materialized and increases the current goal's GoalNodes high-water mark.
//! One rational literal normalization costs 128; a rational numeric node reserves
//! 1,024 steps for the pinned finite-domain checker before discharge. Depth checks
//! precede native/type/graph descent. No executable artifact is produced.

use super::Site;

/// Stable work-accounting version, separate from semantic source identities.
pub const VERSION: &str = "composed-definedness-work/1";

/// Independent capacities, including per-goal and recursion high-water marks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dimension {
    Bytes,
    Declarations,
    Expressions,
    Types,
    Edges,
    Values,
    GraphNodes,
    Facts,
    Goals,
    Materialized,
    GoalNodes,
    Normalization,
    Records,
    Depth,
}

/// Finite capacities; Default defines each independently enforced hard maximum.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    pub bytes: usize,
    pub declarations: usize,
    pub expressions: usize,
    pub types: usize,
    pub edges: usize,
    pub values: usize,
    pub graph_nodes: usize,
    pub facts: usize,
    pub goals: usize,
    pub materialized: usize,
    pub goal_nodes: usize,
    pub normalization: usize,
    pub records: usize,
    pub depth: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            bytes: 8_388_608,
            declarations: 10_000,
            expressions: 100_000,
            types: 1_000_000,
            edges: 100_000,
            values: 10_000,
            graph_nodes: 100_000,
            facts: 100_000,
            goals: 10_000,
            materialized: 100_000,
            goal_nodes: 10_000,
            normalization: 1_000_000,
            records: 200_000,
            depth: 64,
        }
    }
}
impl Limits {
    /// Clamp each dimension without increasing any caller-supplied capacity.
    pub fn bounded(self) -> Self {
        let hard = Self::default();
        Self {
            bytes: self.bytes.min(hard.bytes),
            declarations: self.declarations.min(hard.declarations),
            expressions: self.expressions.min(hard.expressions),
            types: self.types.min(hard.types),
            edges: self.edges.min(hard.edges),
            values: self.values.min(hard.values),
            graph_nodes: self.graph_nodes.min(hard.graph_nodes),
            facts: self.facts.min(hard.facts),
            goals: self.goals.min(hard.goals),
            materialized: self.materialized.min(hard.materialized),
            goal_nodes: self.goal_nodes.min(hard.goal_nodes),
            normalization: self.normalization.min(hard.normalization),
            records: self.records.min(hard.records),
            depth: self.depth.min(hard.depth),
        }
    }
}

/// Successful work only; rejected charges never advance their counter.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Usage {
    pub bytes: usize,
    pub declarations: usize,
    pub expressions: usize,
    pub types: usize,
    pub edges: usize,
    pub values: usize,
    pub graph_nodes: usize,
    pub facts: usize,
    pub goals: usize,
    pub materialized: usize,
    pub max_goal_nodes: usize,
    pub normalization: usize,
    pub records: usize,
    pub max_depth: usize,
}

/// Source-owned next operation whose charge could not be accepted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Exhaustion {
    pub dimension: Dimension,
    pub limit: usize,
    pub prior: usize,
    pub requested: usize,
    pub site: Site,
}

/// One fresh invocation's bounded accounting; this object grants no proof.
#[derive(Debug)]
pub struct Work {
    limits: Limits,
    usage: Usage,
}
impl Work {
    /// Start at zero successful work under the independently clamped capacities.
    pub fn new(limits: Limits) -> Self {
        Self {
            limits: limits.bounded(),
            usage: Usage::default(),
        }
    }
    /// Effective capacities remain visible after exhaustion.
    pub fn limits(&self) -> Limits {
        self.limits
    }
    /// Accumulated successful charges and observed high-water marks.
    pub fn usage(&self) -> Usage {
        self.usage
    }
    /// Charge before work; Depth and GoalNodes accept absolute high-water values.
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
            Dimension::Types => (&mut self.usage.types, self.limits.types),
            Dimension::Edges => (&mut self.usage.edges, self.limits.edges),
            Dimension::Values => (&mut self.usage.values, self.limits.values),
            Dimension::GraphNodes => (&mut self.usage.graph_nodes, self.limits.graph_nodes),
            Dimension::Facts => (&mut self.usage.facts, self.limits.facts),
            Dimension::Goals => (&mut self.usage.goals, self.limits.goals),
            Dimension::Materialized => (&mut self.usage.materialized, self.limits.materialized),
            Dimension::Normalization => (&mut self.usage.normalization, self.limits.normalization),
            Dimension::Records => (&mut self.usage.records, self.limits.records),
            Dimension::Depth => (&mut self.usage.max_depth, self.limits.depth),
            Dimension::GoalNodes => (&mut self.usage.max_goal_nodes, self.limits.goal_nodes),
        };
        let next = if matches!(dimension, Dimension::Depth | Dimension::GoalNodes) {
            Some((*used).max(amount))
        } else {
            used.checked_add(amount)
        };
        *used = next.filter(|next| *next <= limit).ok_or(Exhaustion {
            dimension,
            limit,
            prior: *used,
            requested: amount,
            site,
        })?;
        Ok(())
    }
}
