// SPDX-License-Identifier: AGPL-3.0-or-later
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

/// How one dimension retains successful work.
///
/// This is the accounting contract an independent consumer needs before it can
/// precheck a budget: a peak dimension admits repeated requests up to its
/// ceiling, a cumulative dimension does not.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Accumulation {
    /// Usage retains the largest single candidate; repeated equal requests are free.
    Peak,
    /// Usage retains the checked sum of every successful request.
    Cumulative,
}

/// The exact exhausted resource, never inferred from a diagnostic string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dimension {
    /// The complete offered payload, in bytes.
    PayloadBytes,
    /// The complete encoded output, in bytes.
    OutputBytes,
    /// Each native or foreign original source, in bytes.
    SourceBytes,
    /// Total decoded string UTF-8 content, including JSON keys, in bytes.
    ContentBytes,
    /// Source entries in the complete source inventory.
    Sources,
    /// Dependency entries in the complete dependency inventory.
    Dependencies,
    /// Selected definition entries.
    Definitions,
    /// Selected admitted-model entries.
    Models,
    /// Complete declaration entries.
    Declarations,
    /// Allocated table, member and index records.
    Entries,
    /// Subsequent reference, graph-edge and type visits.
    References,
    /// Cumulative traversed, copied, hashed or compared bytes.
    ByteWork,
    /// Maximum JSON or graph traversal depth.
    Depth,
}

impl Dimension {
    /// Every accounted dimension, in declaration order.
    pub const ALL: [Self; 13] = [
        Self::PayloadBytes,
        Self::OutputBytes,
        Self::SourceBytes,
        Self::ContentBytes,
        Self::Sources,
        Self::Dependencies,
        Self::Definitions,
        Self::Models,
        Self::Declarations,
        Self::Entries,
        Self::References,
        Self::ByteWork,
        Self::Depth,
    ];

    /// How this dimension retains successful work. `Work` charges from this
    /// answer, so a consumer's precheck cannot disagree with the accounting.
    pub const fn accumulation(self) -> Accumulation {
        match self {
            Self::PayloadBytes
            | Self::OutputBytes
            | Self::SourceBytes
            | Self::Sources
            | Self::Dependencies
            | Self::Definitions
            | Self::Models
            | Self::Declarations
            | Self::Depth => Accumulation::Peak,
            Self::ContentBytes | Self::Entries | Self::References | Self::ByteWork => {
                Accumulation::Cumulative
            }
        }
    }
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

pub(crate) struct Work {
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

    /// The counter and ceiling for one dimension. How the counter advances is
    /// not decided here; `Dimension::accumulation` is the single authority.
    fn fields(&mut self, dimension: Dimension) -> (&mut usize, usize) {
        use Dimension as D;
        match dimension {
            D::PayloadBytes => (&mut self.usage.payload_bytes, self.limits.payload_bytes),
            D::OutputBytes => (&mut self.usage.output_bytes, self.limits.output_bytes),
            D::SourceBytes => (&mut self.usage.source_bytes, self.limits.source_bytes),
            D::ContentBytes => (&mut self.usage.content_bytes, self.limits.content_bytes),
            D::Sources => (&mut self.usage.sources, self.limits.sources),
            D::Dependencies => (&mut self.usage.dependencies, self.limits.dependencies),
            D::Definitions => (&mut self.usage.definitions, self.limits.definitions),
            D::Models => (&mut self.usage.models, self.limits.models),
            D::Declarations => (&mut self.usage.declarations, self.limits.declarations),
            D::Entries => (&mut self.usage.entries, self.limits.entries),
            D::References => (&mut self.usage.references, self.limits.references),
            D::ByteWork => (&mut self.usage.byte_work, self.limits.byte_work),
            D::Depth => (&mut self.usage.depth, self.limits.depth),
        }
    }

    pub fn charge(&mut self, dimension: Dimension, requested: usize) -> Result<(), Exhaustion> {
        let accumulation = dimension.accumulation();
        let (used, limit) = self.fields(dimension);
        let next = match accumulation {
            Accumulation::Peak => Some((*used).max(requested)),
            Accumulation::Cumulative => used.checked_add(requested),
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

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::{Accumulation, Dimension, Limits, Work};

    /// `Dimension::accumulation` is published so an independent consumer can
    /// precheck a budget. It is the same answer `charge` acts on, so a copied
    /// peak/cumulative table cannot silently disagree with the accounting.
    #[test]
    #[trace("TC-121", "FR-042-AC-9")]
    fn published_accumulation_matches_every_charged_dimension() {
        for dimension in Dimension::ALL {
            let mut work = Work::new(Limits::default());
            work.charge(dimension, 2).expect("first charge");
            work.charge(dimension, 2).expect("second equal charge");
            let (used, _) = work.fields(dimension);
            let expected = match dimension.accumulation() {
                Accumulation::Peak => 2,
                Accumulation::Cumulative => 4,
            };
            assert_eq!(*used, expected, "{dimension:?}");
        }
    }

    #[test]
    #[trace("TC-121", "FR-042-AC-9")]
    fn every_dimension_is_listed_once() {
        let mut seen = [false; 13];
        for dimension in Dimension::ALL {
            let slot = match dimension {
                Dimension::PayloadBytes => 0,
                Dimension::OutputBytes => 1,
                Dimension::SourceBytes => 2,
                Dimension::ContentBytes => 3,
                Dimension::Sources => 4,
                Dimension::Dependencies => 5,
                Dimension::Definitions => 6,
                Dimension::Models => 7,
                Dimension::Declarations => 8,
                Dimension::Entries => 9,
                Dimension::References => 10,
                Dimension::ByteWork => 11,
                Dimension::Depth => 12,
            };
            assert!(!seen[slot], "{dimension:?} is listed more than once");
            seen[slot] = true;
        }
        assert!(seen.into_iter().all(|present| present));
    }
}
