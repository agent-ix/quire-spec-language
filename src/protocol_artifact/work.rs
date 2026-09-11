// SPDX-License-Identifier: AGPL-3.0-only
//! FR-042: invocation-local limits for the compiled protocol wire boundary.

use super::wire::Locus;

/// Counter contract, independent of native parser and model-admission budgets.
pub const ACCOUNTING_VERSION: &str = "quire.protocol.artifact-work/1";

/// Independently lowered ceilings. Values above the defaults are clamped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    /// Complete offered bytes, at most 8 MiB.
    pub payload_bytes: usize,
    /// Complete encoded bytes, at most 8 MiB.
    pub output_bytes: usize,
    /// Each native or foreign original source, at most 1 MiB.
    pub source_bytes: usize,
    /// Total decoded string UTF-8 bytes, including JSON keys, at most 8 MiB.
    pub content_bytes: usize,
    /// Source entries in either complete inventory, at most 10,000.
    pub sources: usize,
    /// Dependency entries in either complete inventory, at most 10,000.
    pub dependencies: usize,
    /// Selected definition entries, at most 10,000.
    pub definitions: usize,
    /// Selected admitted-model entries, at most 10,000.
    pub models: usize,
    /// Complete declaration entries, at most 10,000.
    pub declarations: usize,
    /// Allocated table/member/index records, at most 100,000.
    pub entries: usize,
    /// Subsequent reference, graph-edge and type visits, at most 1,000,000.
    pub references: usize,
    /// Cumulative traversed, copied, hashed or compared bytes, at most 64 MiB.
    pub byte_work: usize,
    /// Maximum JSON or graph traversal depth, at most 64.
    pub depth: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            payload_bytes: 8 * 1_048_576,
            output_bytes: 8 * 1_048_576,
            source_bytes: 1_048_576,
            content_bytes: 8 * 1_048_576,
            sources: 10_000,
            dependencies: 10_000,
            definitions: 10_000,
            models: 10_000,
            declarations: 10_000,
            entries: 100_000,
            references: 1_000_000,
            byte_work: 64 * 1_048_576,
            depth: 64,
        }
    }
}

/// Successful cumulative work; size and depth dimensions retain their peak.
/// Entries include decoded object members/array elements and validation indexes.
/// Inline terminal errors allocate no entry. References count each subsequent
/// inspected graph edge.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Usage {
    /// Largest offered payload in bytes.
    pub payload_bytes: usize,
    /// Largest successfully written output prefix in bytes.
    pub output_bytes: usize,
    /// Largest checked original source in bytes.
    pub source_bytes: usize,
    /// Successfully reserved decoded string content in UTF-8 bytes.
    pub content_bytes: usize,
    /// Largest checked source inventory.
    pub sources: usize,
    /// Largest checked dependency inventory.
    pub dependencies: usize,
    /// Selected definition count.
    pub definitions: usize,
    /// Selected model count.
    pub models: usize,
    /// Largest checked declaration inventory.
    pub declarations: usize,
    /// Cumulative successfully reserved records.
    pub entries: usize,
    /// Cumulative successful reference/edge/type inspections.
    pub references: usize,
    /// Cumulative successfully charged byte work.
    pub byte_work: usize,
    /// Greatest checked JSON or graph traversal depth.
    pub depth: usize,
}

/// The exact exhausted resource, never inferred from a diagnostic string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dimension {
    PayloadBytes,
    OutputBytes,
    SourceBytes,
    ContentBytes,
    Sources,
    Dependencies,
    Definitions,
    Models,
    Declarations,
    Entries,
    References,
    ByteWork,
    Depth,
}

/// A refused next step; successful prior usage is retained unchanged.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("compiled protocol {dimension:?} limit {limit}: requested {requested} after {used}")]
pub struct Exhaustion {
    /// Resource whose next step was refused.
    pub dimension: Dimension,
    /// Successful usage before that step.
    pub used: usize,
    /// Requested increment, or candidate size for a peak dimension.
    pub requested: usize,
    /// Effective caller ceiling for that resource.
    pub limit: usize,
    /// Original source region when already available.
    pub locus: Option<Locus>,
}

impl Limits {
    /// Effective ceilings, preserving zero and each caller-lowered component.
    pub fn bounded(mut self) -> Self {
        let hard = Self::default();
        macro_rules! clamp {
            ($($field:ident),* $(,)?) => { $(self.$field = self.$field.min(hard.$field);)* };
        }
        clamp!(
            payload_bytes,
            output_bytes,
            source_bytes,
            content_bytes,
            sources,
            dependencies,
            definitions,
            models,
            declarations,
            entries,
            references,
            byte_work,
            depth
        );
        self
    }
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
            D::PayloadBytes => (
                &mut self.usage.payload_bytes,
                self.limits.payload_bytes,
                true,
            ),
            D::OutputBytes => (&mut self.usage.output_bytes, self.limits.output_bytes, true),
            D::SourceBytes => (&mut self.usage.source_bytes, self.limits.source_bytes, true),
            D::ContentBytes => (
                &mut self.usage.content_bytes,
                self.limits.content_bytes,
                false,
            ),
            D::Sources => (&mut self.usage.sources, self.limits.sources, true),
            D::Dependencies => (&mut self.usage.dependencies, self.limits.dependencies, true),
            D::Definitions => (&mut self.usage.definitions, self.limits.definitions, true),
            D::Models => (&mut self.usage.models, self.limits.models, true),
            D::Declarations => (&mut self.usage.declarations, self.limits.declarations, true),
            D::Entries => (&mut self.usage.entries, self.limits.entries, false),
            D::References => (&mut self.usage.references, self.limits.references, false),
            D::ByteWork => (&mut self.usage.byte_work, self.limits.byte_work, false),
            D::Depth => (&mut self.usage.depth, self.limits.depth, true),
        }
    }

    pub fn charge(&mut self, dimension: Dimension, requested: usize) -> Result<(), Exhaustion> {
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
                locus: self.locus.clone(),
            })
        }
    }

    pub fn visit(&mut self) -> Result<(), super::Error> {
        self.charge(Dimension::References, 1)
            .map_err(super::Error::Incomplete)
    }

    pub fn bytes(&mut self, count: usize) -> Result<(), super::Error> {
        self.charge(Dimension::ByteWork, count)
            .map_err(super::Error::Incomplete)
    }
}
