// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 layer-6 `replay`: the CG-facing replay facade (ADR-011 §6.1,
//! ADR-013 O-26, C-13). `origin/main` had no `replay` module before this
//! change (ADR-011 §6.2's module table: "none today | 6 `replay` | new").
//!
//! **This change (#231, ADR-013 O-24 to O-27) builds only the four typed
//! envelopes this module's public API exposes** -- the proof-result
//! envelope (`proof_result`), the counterexample/witness envelope
//! (`witness`), the replay request (`request`) and the replay result
//! (`result`) -- their round trips, and their redacted rendering
//! (FR-069 through FR-073). It builds no backend invocation, no Kani
//! harness and no replay execution: the executor entry (ADR-013 TK-01,
//! `CheckedPackage::call`) is #243's, which "lands it first with the
//! skeleton spine" (ADR-011 §6.1) and widens it per family thereafter.
//! CG reaches this module's public API and nothing else in QSL (ADR-011
//! FB-05).
//!
//! # Provisional local types
//!
//! Several members these envelopes carry have a canonical home ADR-013
//! assigns to a ticket that has not landed on `origin/main` as of this
//! change (#213 slices S-3/S-4: O-07's occurrence key beyond the kernel
//! `quire_exact::Origin`/`Location` pair, O-09's obligation identity, O-11's
//! `QualifiedName`, O-12's resolved region). `identity` defines this
//! module's own minimal, spec-faithful versions, documented there with
//! exactly which ticket should absorb each one. This is not a
//! compatibility layer or a redesign of ADR-013 -- each type has the shape
//! ADR-013 already specifies -- only a placement decision made necessary by
//! an unmerged dependency (see this ticket's PR description for the full
//! account, including PR #262's independent, unmerged `QualifiedName` in
//! `value::expression`, which this change does not touch or depend on).

mod bounds;
mod identity;
mod proof_result;
mod request;
mod result;
mod witness;

pub use bounds::{BoundExceeded, MAX_ENCODED_BYTES};
pub use identity::{
    Backend, DeclaredDomain, EmptyQualifiedName, ObligationIdentity, OccurrenceKey,
    ProfileSelection, QualifiedName, RawSourceRef, TracePosition,
};
pub use proof_result::{
    read_backend_provider_envelope, BackendProviderSource, IncompleteCause, InconclusiveCause,
    ProofCategory, ProofRefusalCause, ProofResultEnvelope, ProofResultRefusal, TerminalRecord,
    TerminalValue, ToolPin, UnavailabilityCause,
};
pub use request::{
    ByteProvision, ReplayRequest, ReplayRequestRefusal, ReplayRequestWire, StageLimits,
    StateEnvironment,
};
pub use result::{
    read_bounded, DisagreementCause, EvaluatedValue, InputArmResult, InputSettlement, ReplayResult,
    ResolvedRegion, SeparatingWitnessRecord, Verdict, WitnessArmResult, WitnessSettlement,
};
pub use witness::{
    CanonicalAssignment, DecodeRefusal, FamilyPayload, MalformedTranscript, NoPayload,
    ReplaySource, Witness, WitnessEnvelope, WitnessPacket, WitnessRefusal,
};

#[cfg(test)]
mod redaction_tests {
    use ix_trace_rs::trace;

    use super::*;
    use crate::digest::{ByteDigest, DigestDomain, DigestRecord, WireNodeId};
    use crate::value::Identifier;

    fn scalar_limits(seed: u64) -> quire_exact::ScalarLimits {
        quire_exact::ScalarLimits {
            integer_bits: seed,
            decimal_digits: seed,
            scale_expansion: seed,
            text_input_bytes: seed,
            text_scalars: seed,
            normalized_scalars: seed,
            unit_edges: seed,
            value_occurrences: seed,
            work_units: seed,
            result_units: seed,
        }
    }

    /// FR-073-AC-3 (TC-211): a `stale_dependency`/`byte-digest-mismatch`
    /// refusal from the replay request, and a malformed-transcript refusal
    /// from the witness envelope, each render with no unredacted content --
    /// and a separately, validly constructed value carrying the same
    /// content stays fully readable through its typed accessor.
    #[trace("TC-211", "FR-073-AC-3")]
    #[test]
    fn tc_211_refusal_causes_redact_while_typed_accessors_stay_readable() {
        // Half 1: replay request byte-digest mismatch.
        let entry_x: Vec<u8> = (0..4096u32).map(|i| (i % 241) as u8).collect();
        let correct_digest = ByteDigest::of(&entry_x).as_bytes();
        let mut mismatched_bytes = entry_x.clone();
        mismatched_bytes[0] ^= 0xFF;

        let source_digest_record = DigestRecord::mint(DigestDomain::SourceBytesV1, correct_digest);
        let wire = ReplayRequestWire {
            contract_version: "quire.native-runtime/v1".to_owned(),
            capability_vocabulary: Some("quire.capability-kind/v1".to_owned()),
            profile_selections: vec![],
            package_id: (
                Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
                DigestRecord::mint(DigestDomain::PackageSemanticV2, [1; 32]).hex(),
            ),
            package_contract_version: "quire.checked-package/v2".to_owned(),
            source_digests: vec![(
                "registry".to_owned(),
                "pkg-a".to_owned(),
                "rev-1".to_owned(),
                Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                source_digest_record.hex(),
            )],
            selected_function: QualifiedName::new(vec![Identifier::new("f").unwrap()]).unwrap(),
            source: ReplaySource::Input(vec![CanonicalAssignment {
                parameter: WireNodeId::from_digest([9; 32]),
                value: 1,
            }]),
            originating_counterexample_identity: [2; 32],
            backend: (
                "kani-backend-1".to_owned(),
                Some(DigestDomain::ToolManifestJcsV1.as_str().to_owned()),
                DigestRecord::mint(DigestDomain::ToolManifestJcsV1, [3; 32]).hex(),
            ),
            state_environment: StateEnvironment::new(vec![]),
            accounting_limits: scalar_limits(1),
            stage_limits: StageLimits {
                s1: scalar_limits(1),
                s2: scalar_limits(1),
                s3: scalar_limits(1),
                s4: scalar_limits(1),
            },
            byte_provision: vec![(
                Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                source_digest_record.hex(),
                mismatched_bytes,
            )],
        };
        let refusal = ReplayRequest::decode(wire).unwrap_err();
        let rendered = format!("{refusal:?} {refusal}");
        assert!(!contains_bytes(&rendered, &entry_x));

        // A separately, validly constructed request whose byte provision
        // carries entry X's exact bytes under their own correct digest (no
        // mismatch) still returns the full content through the typed
        // accessor.
        let valid_wire = ReplayRequestWire {
            contract_version: "quire.native-runtime/v1".to_owned(),
            capability_vocabulary: Some("quire.capability-kind/v1".to_owned()),
            profile_selections: vec![],
            package_id: (
                Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
                DigestRecord::mint(DigestDomain::PackageSemanticV2, [1; 32]).hex(),
            ),
            package_contract_version: "quire.checked-package/v2".to_owned(),
            source_digests: vec![(
                "registry".to_owned(),
                "pkg-a".to_owned(),
                "rev-1".to_owned(),
                Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                source_digest_record.hex(),
            )],
            selected_function: QualifiedName::new(vec![Identifier::new("f").unwrap()]).unwrap(),
            source: ReplaySource::Input(vec![CanonicalAssignment {
                parameter: WireNodeId::from_digest([9; 32]),
                value: 1,
            }]),
            originating_counterexample_identity: [2; 32],
            backend: (
                "kani-backend-1".to_owned(),
                Some(DigestDomain::ToolManifestJcsV1.as_str().to_owned()),
                DigestRecord::mint(DigestDomain::ToolManifestJcsV1, [3; 32]).hex(),
            ),
            state_environment: StateEnvironment::new(vec![]),
            accounting_limits: scalar_limits(1),
            stage_limits: StageLimits {
                s1: scalar_limits(1),
                s2: scalar_limits(1),
                s3: scalar_limits(1),
                s4: scalar_limits(1),
            },
            byte_provision: vec![(
                Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                source_digest_record.hex(),
                entry_x.clone(),
            )],
        };
        let valid_request = ReplayRequest::decode(valid_wire).unwrap();
        let looked_up = valid_request
            .byte_provision()
            .get(source_digest_record)
            .unwrap();
        assert_eq!(looked_up, entry_x.as_slice());

        // Half 2: witness envelope malformed-transcript refusal.
        let marker = "MARKER-transcript-y-4c2b1a";
        let untrimmed = format!("leading-junk<<<assertion|h|c|{marker}=1>>>");
        let refusal = Witness::parse(untrimmed).unwrap_err();
        let rendered = format!("{refusal:?} {refusal}");
        assert!(!rendered.contains(marker));

        // A separately, validly constructed witness from the trimmed block
        // still returns the marker through its typed accessor.
        let valid_witness = Witness::parse(format!("<<<assertion|h|c|{marker}=1>>>")).unwrap();
        assert!(valid_witness
            .concrete_values()
            .iter()
            .any(|(name, _)| name == marker));
    }

    /// Whether `haystack` contains `needle`'s bytes reinterpreted as a
    /// lossy UTF-8 string (the shape a `{:?}`/`{}` rendering of a `Vec<u8>`
    /// field would produce if it leaked raw bytes through `derive(Debug)`).
    fn contains_bytes(haystack: &str, needle: &[u8]) -> bool {
        let needle_text = String::from_utf8_lossy(needle);
        haystack.contains(needle_text.as_ref())
    }
}
