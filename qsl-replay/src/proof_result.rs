// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-069 (ADR-013 O-24): the typed proof-result envelope. Reads one FR-331
//! `quire.backend-provider/v1` terminal record into exactly one of the
//! seven ADR-013 O-16 outcome categories the proof column produces.
//!
//! `undefined` is not a variant of [`ProofCategory`] at all (as distinct
//! from being a variant no arm maps into): the O-16 category table marks it
//! "not produced" for the proof column, so no FR-331 result value can ever
//! select it. A future arm attempting to map some FR-331 value to
//! `undefined` is a compile error here, not merely untested dead code.

use crate::bounds::{BoundExceeded, MAX_ENCODED_BYTES};
use crate::identity::Backend;
use qsl_foundation::digest::ManifestDigest;

/// One of the seven ADR-013 O-16 categories an FR-331 proof column can
/// produce (ADR-013 O-16 category table; FR-069). `Category` in
/// `crate::diagnostic` has an eighth, `Undefined`, that this type omits by
/// construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ProofCategory {
    /// A completed, non-vacuous positive result.
    Success,
    /// A false predicate: the property does not hold.
    Violation,
    /// The operation is defined but the request itself is not admitted.
    Refusal,
    /// A named, catalogued capability this build, backend or solver does not
    /// supply.
    Unsupported,
    /// A named charge point was unavailable and no partial value exists.
    Incomplete,
    /// No decisive result: a vacuous proof, or (outside this type) a
    /// settlement disagreement.
    Inconclusive,
    /// The tool itself failed, not the property under test.
    InternalFailure,
}

impl ProofCategory {
    /// Every value, in the ADR-013 O-16 category table's row order (skipping
    /// the `undefined` row, which the proof column never produces).
    pub const ALL: [Self; 7] = [
        Self::Success,
        Self::Violation,
        Self::Refusal,
        Self::Unsupported,
        Self::Incomplete,
        Self::Inconclusive,
        Self::InternalFailure,
    ];
}

/// FR-331's `incomplete` cause: which charge point the run failed to complete.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum IncompleteCause {
    /// The run exceeded its configured time budget.
    TimedOut,
    /// The run was cancelled before it produced a result.
    Cancelled,
    /// The run exhausted a configured resource bound before completing.
    ResourceExhausted,
}

/// FR-331's `declined` (refusal) cause: `Refused`, `InvalidInput` or
/// `IncompleteInput` all collapse to one FR-331 result with this typed
/// cause (ADR-013 O-16, QC-9).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ProofRefusalCause {
    /// The backend declined the request outright.
    Refused,
    /// The request's input was invalid.
    InvalidInput,
    /// The request's input was incomplete.
    IncompleteInput,
}

/// FR-331's `unsupported` cause: the solver or backend is absent after
/// negotiation (ADR-013 O-16).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum UnavailabilityCause {
    /// No solver satisfying the request's capability negotiation is present.
    SolverAbsent,
    /// No backend satisfying the request's capability negotiation is
    /// present.
    BackendAbsent,
}

/// FR-331's `inconclusive` cause. `KaniVacuousProof` is the O-16 vacuity
/// row: a `Proved` run with zero SUCCESS checks in the obligation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum InconclusiveCause {
    /// A `Proved` run with zero SUCCESS checks in the obligation.
    KaniVacuousProof,
}

/// One FR-331 terminal record's result value (ADR-013 O-16 proof column).
/// Exactly the eight wire values FR-331's `results` vocabulary admits, with
/// `Proved` distinguishing a vacuous run (zero SUCCESS checks) from an
/// ordinary one by its own field rather than by a second tag, so the
/// category-mapping reader can tell them apart without inspecting anything
/// but this value.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum TerminalValue {
    /// A Kani run whose obligation completed with `success_checks` SUCCESS
    /// checks. Zero is vacuous (maps to `inconclusive`); at least one is an
    /// ordinary proof (maps to `success`).
    Proved {
        /// The count of SUCCESS checks the obligation completed with. Zero
        /// marks a vacuous proof.
        success_checks: u32,
    },
    /// A backend result of `tested`: `success` category, but never promoted
    /// to `proved` and never counted as proof evidence.
    Tested,
    /// A backend result of `refuted`: the property does not hold.
    Refuted,
    /// A backend result of `declined`, `invalid-input` or
    /// `incomplete-input`, collapsed to one typed cause.
    Declined(ProofRefusalCause),
    /// A backend result of `unsupported`.
    Unsupported(UnavailabilityCause),
    /// A backend result of `incomplete`.
    Incomplete(IncompleteCause),
    /// A backend result of `failed`: the tool itself failed.
    Failed,
}

impl TerminalValue {
    /// Map this value to its ADR-013 O-16 category. Exhaustive with no `_`
    /// arm; a ninth `TerminalValue` variant fails this match to compile
    /// rather than silently falling into an existing row (FR-069's
    /// Behavior: "one exhaustive function with no `_` fallback arm").
    pub fn category(self) -> ProofCategory {
        match self {
            Self::Proved { success_checks: 0 } => ProofCategory::Inconclusive,
            Self::Proved { .. } | Self::Tested => ProofCategory::Success,
            Self::Refuted => ProofCategory::Violation,
            Self::Declined(_) => ProofCategory::Refusal,
            Self::Unsupported(_) => ProofCategory::Unsupported,
            Self::Incomplete(_) => ProofCategory::Incomplete,
            Self::Failed => ProofCategory::InternalFailure,
        }
    }

    /// The typed cause of a vacuous proof, if this value is one.
    pub fn vacuous_proof_cause(self) -> Option<InconclusiveCause> {
        match self {
            Self::Proved { success_checks: 0 } => Some(InconclusiveCause::KaniVacuousProof),
            _ => None,
        }
    }
}

/// One requested item's FR-331 terminal record: its request-scoped item
/// identity and its result value.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct TerminalRecord {
    item: String,
    value: TerminalValue,
}

impl TerminalRecord {
    /// Build a terminal record for `item` (the request-scoped item identity)
    /// carrying `value`.
    pub fn new(item: impl Into<String>, value: TerminalValue) -> Self {
        Self {
            item: item.into(),
            value,
        }
    }

    /// The request-scoped item identity this record was read for.
    pub fn item(&self) -> &str {
        &self.item
    }

    /// The FR-331 result value this record carries.
    pub fn value(&self) -> TerminalValue {
        self.value
    }
}

/// The executor/tool pin an FR-331 manifest carries alongside its `backend`
/// member.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ToolPin(String);

impl ToolPin {
    /// Wrap an already-read tool pin string.
    pub fn new(pin: impl Into<String>) -> Self {
        Self(pin.into())
    }

    /// The tool pin's raw string form.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// FR-069/ADR-013 O-24: the typed proof-result envelope for one requested
/// item. No public constructor other than [`read_backend_provider_envelope`]
/// (FR-069-CON-1): a caller cannot name an arbitrary category directly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofResultEnvelope {
    category: ProofCategory,
    record: TerminalRecord,
    backend: Backend,
    tool_pin: ToolPin,
    inconclusive_cause: Option<InconclusiveCause>,
}

impl ProofResultEnvelope {
    /// The item's ADR-013 O-16 category.
    pub fn category(&self) -> ProofCategory {
        self.category
    }

    /// The FR-331 terminal record this category was read from.
    pub fn record(&self) -> &TerminalRecord {
        &self.record
    }

    /// The `backend` member (ADR-013 O-19).
    pub fn backend(&self) -> &Backend {
        &self.backend
    }

    /// The executor/tool pin.
    pub fn tool_pin(&self) -> &ToolPin {
        &self.tool_pin
    }

    /// FR-069 Behavior's typed vacuous-proof cause: `Some(KaniVacuousProof)`
    /// when this envelope's category is `Inconclusive` because the run was a
    /// vacuous `Proved` (zero SUCCESS checks); `None` for every other
    /// category. Reachable directly here, so a consumer that reads
    /// `category() == Inconclusive` never has to re-derive the cause by
    /// re-counting SUCCESS checks against [`Self::record`] itself.
    pub fn inconclusive_cause(&self) -> Option<InconclusiveCause> {
        self.inconclusive_cause
    }
}

/// [`read_backend_provider_envelope`]'s structured refusal.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProofResultRefusal {
    /// `contract_version` is not exactly `quire.backend-provider/v1`.
    #[error("unknown_wire/unsupported-wire: {0:?} is not quire.backend-provider/v1")]
    UnknownContractVersion(String),
    /// `capability_vocabulary` is absent, or not exactly
    /// `quire.capability-kind/v1`.
    #[error("invalid_capability/unsupported-version: capability_vocabulary is not quire.capability-kind/v1")]
    UnknownCapabilityVocabulary,
    /// The encoded envelope exceeds the configured reader bound.
    #[error(transparent)]
    BoundExceeded(#[from] BoundExceeded),
}

const CONTRACT_VERSION: &str = "quire.backend-provider/v1";
const CAPABILITY_VOCABULARY: &str = "quire.capability-kind/v1";

/// A minimal, already-parsed representation of one FR-331
/// `quire.backend-provider/v1` envelope: exactly the members FR-069's
/// Inputs section names (`results`, `manifest`, tool pin), enough to build
/// and round-trip a [`ProofResultEnvelope`] per item. This is not a general
/// FR-331 wire reader; it is the input shape FR-069's reader consumes.
#[derive(Clone, Debug)]
pub struct BackendProviderSource {
    /// The envelope's `contract_version` member; must equal
    /// `quire.backend-provider/v1` or the reader refuses.
    pub contract_version: String,
    /// The envelope's `capability_vocabulary` member; must equal
    /// `quire.capability-kind/v1` or the reader refuses.
    pub capability_vocabulary: Option<String>,
    /// The provider identity, as the FR-331 manifest states it.
    pub backend_identity: String,
    /// The FR-331 manifest's digest (ADR-013 O-19), always domain
    /// `quire.tool-manifest.jcs/v1` (QSL-227: [`ManifestDigest`] cannot be
    /// constructed with any other domain).
    pub manifest_digest: ManifestDigest,
    /// The executor/tool pin the manifest carries alongside `backend`.
    pub tool_pin: String,
    /// The per-item terminal records this envelope reports.
    pub items: Vec<TerminalRecord>,
}

/// The reader's own measurement of `source`'s encoded size (FR-069-AC-4):
/// every string member's byte length plus a fixed per-item allowance for the
/// terminal-value tag -- never a caller-declared number a source could
/// understate to launder an oversized item list past the bound (B3).
fn measured_encoded_bytes(source: &BackendProviderSource) -> usize {
    source.contract_version.len()
        + source.capability_vocabulary.as_deref().map_or(0, str::len)
        + source.backend_identity.len()
        + source.manifest_digest.record().as_bytes().len()
        + source.tool_pin.len()
        + source
            .items
            .iter()
            .map(|record| record.item().len() + std::mem::size_of::<TerminalValue>())
            .sum::<usize>()
}

/// FR-069's reader: read every item of `source` into its
/// [`ProofResultEnvelope`], refusing before any `results`/`dispositions`/
/// `counterexamples`/`accounting` member is read if `contract_version` or
/// `capability_vocabulary` mismatch, or if the encoded size is over the
/// configured reader bound (FR-069-AC-2, FR-069-AC-4).
pub fn read_backend_provider_envelope(
    source: &BackendProviderSource,
) -> Result<Vec<ProofResultEnvelope>, ProofResultRefusal> {
    // Bound, version and vocabulary checks happen strictly first: nothing
    // below this point touches `source.items` until every one of these
    // three checks has passed.
    let measured = measured_encoded_bytes(source);
    if measured > MAX_ENCODED_BYTES {
        return Err(BoundExceeded { actual: measured }.into());
    }
    if source.contract_version != CONTRACT_VERSION {
        return Err(ProofResultRefusal::UnknownContractVersion(
            source.contract_version.clone(),
        ));
    }
    match source.capability_vocabulary.as_deref() {
        Some(CAPABILITY_VOCABULARY) => {}
        _ => return Err(ProofResultRefusal::UnknownCapabilityVocabulary),
    }

    let backend = Backend::new(source.backend_identity.clone(), source.manifest_digest);
    let tool_pin = ToolPin::new(source.tool_pin.clone());
    Ok(source
        .items
        .iter()
        .map(|record| ProofResultEnvelope {
            category: record.value.category(),
            inconclusive_cause: record.value.vacuous_proof_cause(),
            record: record.clone(),
            backend: backend.clone(),
            tool_pin: tool_pin.clone(),
        })
        .collect())
}

impl ProofResultEnvelope {
    /// Re-serialize `envelopes` (every one read from a single source) back
    /// into the [`BackendProviderSource`] shape [`read_backend_provider_envelope`]
    /// consumes, for round-tripping a positive read through the reader again
    /// (FR-069-AC-3's construct -> serialize -> read round trip). Every
    /// envelope this reader ever produces shares one `backend`/`tool_pin`
    /// (they all come from the one source it read), so the first envelope's
    /// is representative. An empty slice has no `backend` to serialize and
    /// is refused with [`EmptyEnvelopeSet`].
    pub fn to_source(envelopes: &[Self]) -> Result<BackendProviderSource, EmptyEnvelopeSet> {
        let first = envelopes.first().ok_or(EmptyEnvelopeSet)?;
        Ok(BackendProviderSource {
            contract_version: CONTRACT_VERSION.to_owned(),
            capability_vocabulary: Some(CAPABILITY_VOCABULARY.to_owned()),
            backend_identity: first.backend.identity().to_owned(),
            manifest_digest: first.backend.manifest_digest(),
            tool_pin: first.tool_pin.as_str().to_owned(),
            items: envelopes.iter().map(|e| e.record.clone()).collect(),
        })
    }
}

/// [`ProofResultEnvelope::to_source`]'s refusal: there is no envelope to
/// take the source's `backend` and tool pin from.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("no proof-result envelope to serialize")]
pub struct EmptyEnvelopeSet;

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn manifest_digest() -> ManifestDigest {
        ManifestDigest::from_digest([0x11; 32])
    }

    fn source(items: Vec<TerminalRecord>) -> BackendProviderSource {
        BackendProviderSource {
            contract_version: CONTRACT_VERSION.to_owned(),
            capability_vocabulary: Some(CAPABILITY_VOCABULARY.to_owned()),
            backend_identity: "kani-backend-1".to_owned(),
            manifest_digest: manifest_digest(),
            tool_pin: "kani-0.67.0".to_owned(),
            items,
        }
    }

    /// FR-069-AC-1 (TC-177): every FR-331 wire value maps to its exact
    /// O-16 category, `proved`/`tested` stay distinct within `success`, and
    /// a vacuous `Proved` (zero SUCCESS checks) maps to `inconclusive`, not
    /// `success` -- distinguished from an ordinary `Proved` only by the
    /// SUCCESS-check count, not by a different outer tag.
    #[trace("TC-177", "FR-069-AC-1")]
    #[test]
    fn tc_177_every_fr331_value_maps_to_its_exact_category() {
        let cases = [
            (
                TerminalValue::Proved { success_checks: 1 },
                ProofCategory::Success,
            ),
            (
                TerminalValue::Proved { success_checks: 0 },
                ProofCategory::Inconclusive,
            ),
            (TerminalValue::Tested, ProofCategory::Success),
            (TerminalValue::Refuted, ProofCategory::Violation),
            (
                TerminalValue::Declined(ProofRefusalCause::Refused),
                ProofCategory::Refusal,
            ),
            (
                TerminalValue::Unsupported(UnavailabilityCause::SolverAbsent),
                ProofCategory::Unsupported,
            ),
            (
                TerminalValue::Incomplete(IncompleteCause::TimedOut),
                ProofCategory::Incomplete,
            ),
            (
                TerminalValue::Incomplete(IncompleteCause::Cancelled),
                ProofCategory::Incomplete,
            ),
            (TerminalValue::Failed, ProofCategory::InternalFailure),
        ];
        let items: Vec<TerminalRecord> = cases
            .iter()
            .enumerate()
            .map(|(i, (value, _))| TerminalRecord::new(format!("item-{i}"), *value))
            .collect();
        let envelopes = read_backend_provider_envelope(&source(items)).unwrap();
        for (envelope, (value, expected_category)) in envelopes.iter().zip(cases.iter()) {
            assert_eq!(
                envelope.category(),
                *expected_category,
                "{value:?} should map to {expected_category:?}"
            );
            assert_eq!(envelope.record().value(), *value);
        }
        // `tested` never gets rewritten to `proved`, despite sharing a
        // category with it.
        assert_eq!(
            envelopes[0].record().value(),
            TerminalValue::Proved { success_checks: 1 }
        );
        assert_eq!(envelopes[2].record().value(), TerminalValue::Tested);
        assert_ne!(envelopes[0].record().value(), envelopes[2].record().value());
        // Step 4: an ordinary and a vacuous `Proved` record take different
        // categories, showing the reader inspects the SUCCESS-check count
        // and not merely the outer `Proved` tag.
        assert_ne!(envelopes[0].category(), envelopes[1].category());

        // B4: the vacuous `Proved` envelope carries the typed cause, and no
        // other envelope does.
        assert_eq!(envelopes[0].inconclusive_cause(), None);
        assert_eq!(
            envelopes[1].inconclusive_cause(),
            Some(InconclusiveCause::KaniVacuousProof)
        );
        for (i, envelope) in envelopes.iter().enumerate() {
            if i != 1 {
                assert_eq!(envelope.inconclusive_cause(), None, "item {i}");
            }
        }
    }

    /// B4: [`ProofCategory::ALL`] names exactly the set of categories every
    /// [`TerminalValue`] case actually produces -- a real caller and test
    /// for an array that previously had neither.
    #[test]
    fn all_categories_are_exactly_the_ones_terminal_value_produces() {
        let mut produced: Vec<ProofCategory> = vec![
            TerminalValue::Proved { success_checks: 1 }.category(),
            TerminalValue::Proved { success_checks: 0 }.category(),
            TerminalValue::Tested.category(),
            TerminalValue::Refuted.category(),
            TerminalValue::Declined(ProofRefusalCause::Refused).category(),
            TerminalValue::Unsupported(UnavailabilityCause::SolverAbsent).category(),
            TerminalValue::Incomplete(IncompleteCause::TimedOut).category(),
            TerminalValue::Failed.category(),
        ];
        produced.sort_by_key(|c| format!("{c:?}"));
        produced.dedup();

        let mut all: Vec<ProofCategory> = ProofCategory::ALL.to_vec();
        all.sort_by_key(|c| format!("{c:?}"));

        assert_eq!(produced, all);
    }

    /// FR-069-AC-2/AC-4 (TC-178): an unknown `contract_version`, a mismatched
    /// or absent `capability_vocabulary`, and an oversized encoding each
    /// refuse before any item is read, with no partial envelope returned.
    #[trace("TC-178", "FR-069-AC-2", "FR-069-AC-4")]
    #[test]
    fn tc_178_refuses_unknown_version_vocabulary_or_oversized_envelope() {
        let mut bad_version = source(vec![TerminalRecord::new("x", TerminalValue::Tested)]);
        bad_version.contract_version = "quire.backend-provider/v2-draft".to_owned();
        assert!(matches!(
            read_backend_provider_envelope(&bad_version),
            Err(ProofResultRefusal::UnknownContractVersion(_))
        ));

        let mut bad_vocab = source(vec![TerminalRecord::new("x", TerminalValue::Tested)]);
        bad_vocab.capability_vocabulary = Some("quire.capability-kind/v2-draft".to_owned());
        assert!(matches!(
            read_backend_provider_envelope(&bad_vocab),
            Err(ProofResultRefusal::UnknownCapabilityVocabulary)
        ));

        let mut absent_vocab = source(vec![TerminalRecord::new("x", TerminalValue::Tested)]);
        absent_vocab.capability_vocabulary = None;
        assert!(matches!(
            read_backend_provider_envelope(&absent_vocab),
            Err(ProofResultRefusal::UnknownCapabilityVocabulary)
        ));

        // B3: the bound check measures the source's own content -- there is
        // no `encoded_bytes` field a caller could understate -- so an
        // oversized source has to actually carry oversized content.
        let oversized = source(vec![TerminalRecord::new(
            "x".repeat(MAX_ENCODED_BYTES + 1),
            TerminalValue::Tested,
        )]);
        let result = read_backend_provider_envelope(&oversized);
        assert!(matches!(result, Err(ProofResultRefusal::BoundExceeded(_))));
    }

    /// FR-069-AC-3 (TC-179): a positive envelope's construct -> serialize ->
    /// read round trip -- construct via [`read_backend_provider_envelope`],
    /// serialize via [`ProofResultEnvelope::to_source`] (#231 builds no
    /// byte-level wire serializer for the backend envelope, so this is the
    /// in-process shape that stands in for one), read via
    /// [`read_backend_provider_envelope`] again -- preserves the `backend`
    /// member, tool pin and every per-item disposition byte for byte, with
    /// no re-derivation of the manifest digest. N2: this is a real
    /// construct/serialize/read round trip through `to_source`, not two
    /// reads of the same untouched source.
    #[trace("TC-179", "FR-069-AC-3")]
    #[test]
    fn tc_179_round_trip_preserves_backend_tool_pin_and_dispositions() {
        let items = vec![
            TerminalRecord::new("a", TerminalValue::Proved { success_checks: 3 }),
            TerminalRecord::new("b", TerminalValue::Refuted),
        ];
        let original = source(items);
        let first = read_backend_provider_envelope(&original).unwrap();
        let serialized = ProofResultEnvelope::to_source(&first).unwrap();
        let second = read_backend_provider_envelope(&serialized).unwrap();
        assert_eq!(first, second);
        for envelope in &first {
            assert_eq!(envelope.backend().identity(), "kani-backend-1");
            assert_eq!(envelope.backend().manifest_digest(), manifest_digest());
            assert_eq!(envelope.tool_pin().as_str(), "kani-0.67.0");
        }

        // Step 4: a mutated manifest digest is a different `Backend`, never
        // silently regenerated to match.
        let mut mutated = original.clone();
        let mut bytes = *manifest_digest().record().as_bytes();
        bytes[0] ^= 0xFF;
        mutated.manifest_digest = ManifestDigest::from_digest(bytes);
        let mutated_envelopes = read_backend_provider_envelope(&mutated).unwrap();
        assert_ne!(
            mutated_envelopes[0].backend().manifest_digest(),
            first[0].backend().manifest_digest()
        );
    }

    /// `to_source` on an empty slice returns [`EmptyEnvelopeSet`] instead of
    /// panicking: there is no envelope to take the `backend` from.
    #[test]
    fn to_source_refuses_an_empty_envelope_set() {
        assert_eq!(
            ProofResultEnvelope::to_source(&[]).unwrap_err(),
            EmptyEnvelopeSet
        );
    }
}
