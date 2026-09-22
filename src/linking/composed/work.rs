// SPDX-License-Identifier: AGPL-3.0-or-later
//! Invocation-local, charge-before-work accounting for the syntax namespace stage.

/// Version of the charging rules, not a language or package identity.
pub const ACCOUNTING_VERSION: &str = "composed-namespace-work/1";

/// Finite whole-package capacities. Per-unit parser limits remain separate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkLimits {
    /// UTF-8 bytes in inventory language/edition; each expected authority and
    /// source identity/revision; each supplied source identity/revision/path/text.
    /// Each occurrence is counted once, including duplicate/unexpected sources.
    /// Fixed-width ByteDigest values require no textual digest parsing or hashing.
    pub source_bytes: usize,
    /// One entry per expected source, then one per supplied Source, before access.
    pub units: usize,
    /// One per parsed native declaration before indexing its package name.
    pub declarations: usize,
    /// Syntax nodes inspected and native reference resolution attempts. Repeated
    /// occurrences are charged independently; lookup hits do not waive charges.
    pub references: usize,
    /// One per inserted semantic edge and per subsequent visit of that edge.
    /// Shared targets and revisits are charged on every pass.
    pub dependency_edges: usize,
}

/// Hard capacities for this stage; no definition/model artifacts are accepted.
pub const HARD_LIMITS: WorkLimits = WorkLimits {
    source_bytes: 16_777_216,
    units: 512,
    declarations: 8_192,
    references: 1_000_000,
    dependency_edges: 1_000_000,
};

/// Default capacities equal the hard capacities; callers may lower any to zero.
pub const DEFAULT_LIMITS: WorkLimits = HARD_LIMITS;

impl Default for WorkLimits {
    fn default() -> Self {
        DEFAULT_LIMITS
    }
}

impl WorkLimits {
    /// Clamp every requested capacity to its hard ceiling, preserving zero.
    pub fn effective(self) -> Self {
        Self {
            source_bytes: self.source_bytes.min(HARD_LIMITS.source_bytes),
            units: self.units.min(HARD_LIMITS.units),
            declarations: self.declarations.min(HARD_LIMITS.declarations),
            references: self.references.min(HARD_LIMITS.references),
            dependency_edges: self.dependency_edges.min(HARD_LIMITS.dependency_edges),
        }
    }
}

/// Successful charges only; an unaffordable charge leaves counters unchanged.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Usage {
    /// Charged source text and selection/identity/path metadata bytes.
    pub source_bytes: usize,
    /// Charged expected and supplied inventory entries.
    pub units: usize,
    /// Declarations charged before package-name indexing.
    pub declarations: usize,
    /// Charged syntax inspections and native reference resolution attempts.
    pub references: usize,
    /// Charged semantic edge insertions and traversal visits.
    pub dependency_edges: usize,
}

/// Independent counter selected by one charge under ACCOUNTING_VERSION.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dimension {
    /// Source content and inventoried selection/identity/path bytes.
    SourceBytes,
    /// Expected and supplied source entries.
    Units,
    /// Native declarations indexed in the package namespace.
    Declarations,
    /// Syntax inspections and native reference resolution attempts.
    References,
    /// Semantic edge insertion and every subsequent traversal visit.
    DependencyEdges,
}

/// The next operation was not performed, including on arithmetic overflow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Exhaustion {
    /// Counter that rejected the next charge.
    pub dimension: Dimension,
    /// Successful charges in this dimension before rejection.
    pub used: usize,
    /// Additional work refused before it was performed.
    pub requested: usize,
    /// Effective inclusive capacity of the exhausted dimension.
    pub limit: usize,
}

impl Exhaustion {
    /// Native incomplete-work code, distinct from semantic refusal.
    pub fn code(self) -> qsl_foundation::Code {
        qsl_foundation::Code::ResourceExhausted
    }
}

/// Replayable public accounting boundary. Constructing or charging a counter
/// cannot construct an admitted namespace. Each admission creates a fresh Work.
#[derive(Debug)]
pub struct Work {
    limits: WorkLimits,
    usage: Usage,
}

impl Work {
    /// Start fresh zero counters under effective caller-lowered capacities.
    pub fn new(limits: WorkLimits) -> Self {
        Self {
            limits: limits.effective(),
            usage: Usage::default(),
        }
    }

    /// Effective capacities fixed at construction.
    pub fn limits(&self) -> WorkLimits {
        self.limits
    }

    /// Successful charges so far; refused work is excluded.
    pub fn usage(&self) -> Usage {
        self.usage
    }

    /// Reserve work before performing it; failure or overflow changes no counter.
    pub fn charge(&mut self, dimension: Dimension, requested: usize) -> Result<(), Exhaustion> {
        let (used, limit) = match dimension {
            Dimension::SourceBytes => (&mut self.usage.source_bytes, self.limits.source_bytes),
            Dimension::Units => (&mut self.usage.units, self.limits.units),
            Dimension::Declarations => (&mut self.usage.declarations, self.limits.declarations),
            Dimension::References => (&mut self.usage.references, self.limits.references),
            Dimension::DependencyEdges => (
                &mut self.usage.dependency_edges,
                self.limits.dependency_edges,
            ),
        };
        let next = used.checked_add(requested).filter(|next| *next <= limit);
        match next {
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
