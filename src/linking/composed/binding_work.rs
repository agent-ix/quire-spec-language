// SPDX-License-Identifier: AGPL-3.0-only
//! FR-036: invocation-local work for definition, model and lexical binding.

/// Charging contract for static binding; independent of source namespace work.
pub const ACCOUNTING_VERSION: &str = "composed-binding-work/1";

/// Inclusive capacities, checked before work. Defaults are the hard ceilings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    /// Supplied artifact and selection metadata UTF-8 bytes, once per occurrence.
    pub bytes: usize,
    /// Supplied definition entries, including duplicate and unselected entries.
    pub definitions: usize,
    /// Supplied model entries, including duplicate and unselected entries.
    pub models: usize,
    /// Created binder, role, export, native type layer, cause or declaration records.
    pub bindings: usize,
    /// Inspected syntax nodes, selections and name resolution attempts.
    pub references: usize,
    /// Inserted dependency edges and every subsequent traversal visit.
    pub edges: usize,
}

/// Hard/default capacities; zero remains zero when a caller lowers a capacity.
pub const HARD_LIMITS: Limits = Limits {
    bytes: 33_554_432,
    definitions: 512,
    models: 128,
    bindings: 262_144,
    references: 2_000_000,
    edges: 2_000_000,
};

impl Default for Limits {
    fn default() -> Self {
        HARD_LIMITS
    }
}

impl Limits {
    /// Clamp every dimension independently without changing any semantic input.
    pub fn effective(self) -> Self {
        Self {
            bytes: self.bytes.min(HARD_LIMITS.bytes),
            definitions: self.definitions.min(HARD_LIMITS.definitions),
            models: self.models.min(HARD_LIMITS.models),
            bindings: self.bindings.min(HARD_LIMITS.bindings),
            references: self.references.min(HARD_LIMITS.references),
            edges: self.edges.min(HARD_LIMITS.edges),
        }
    }
}

/// Successful charges only; rejecting an operation leaves its counter unchanged.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Usage {
    /// Artifact and metadata bytes.
    pub bytes: usize,
    /// Supplied definitions.
    pub definitions: usize,
    /// Supplied models.
    pub models: usize,
    /// Created binding records.
    pub bindings: usize,
    /// Inspections and resolution attempts.
    pub references: usize,
    /// Edge insertions and visits.
    pub edges: usize,
}

/// One independent bounded dimension.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dimension {
    /// Artifact and metadata bytes.
    Bytes,
    /// Supplied definitions.
    Definitions,
    /// Supplied models.
    Models,
    /// Created binding records.
    Bindings,
    /// Inspections and resolution attempts.
    References,
    /// Edge insertions and visits.
    Edges,
}

/// First operation refused before execution, including arithmetic overflow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Exhaustion {
    /// Counter that refused the charge.
    pub dimension: Dimension,
    /// Successful prior charges.
    pub used: usize,
    /// Cost of the operation that was not performed.
    pub requested: usize,
    /// Effective inclusive capacity.
    pub limit: usize,
}

impl Exhaustion {
    /// Incomplete processing is distinct from a semantic refusal.
    pub fn code(self) -> crate::Code {
        crate::Code::ResourceExhausted
    }
}

/// Fresh counters for one binding invocation; never a semantic admission token.
#[derive(Debug)]
pub struct Work {
    limits: Limits,
    usage: Usage,
}

impl Work {
    /// Begin a new invocation with no prior charges.
    pub fn new(limits: Limits) -> Self {
        Self {
            limits: limits.effective(),
            usage: Usage::default(),
        }
    }
    /// Effective capacities fixed at entry.
    pub fn limits(&self) -> Limits {
        self.limits
    }
    /// Successfully reserved work.
    pub fn usage(&self) -> Usage {
        self.usage
    }
    /// Charge before performing work. Neither exhaustion nor overflow increments.
    pub fn charge(&mut self, dimension: Dimension, requested: usize) -> Result<(), Exhaustion> {
        let (used, limit) = match dimension {
            Dimension::Bytes => (&mut self.usage.bytes, self.limits.bytes),
            Dimension::Definitions => (&mut self.usage.definitions, self.limits.definitions),
            Dimension::Models => (&mut self.usage.models, self.limits.models),
            Dimension::Bindings => (&mut self.usage.bindings, self.limits.bindings),
            Dimension::References => (&mut self.usage.references, self.limits.references),
            Dimension::Edges => (&mut self.usage.edges, self.limits.edges),
        };
        match used.checked_add(requested).filter(|next| *next <= limit) {
            Some(next) => {
                *used = next;
                Ok(())
            }
            None => Err(Exhaustion {
                dimension,
                used: *used,
                requested,
                limit,
            }),
        }
    }
}
