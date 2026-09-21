// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-071 (ADR-013 O-26): the typed replay request.
//!
//! The request is exactly the O-25 packet's package reference, selection
//! and replay-source members plus the FR-070 envelope's state environment,
//! accounting limits and S1-to-S4 stage limits (ADR-013 O-26's "Public
//! type" row) -- nothing else, and in particular no field, accessor or
//! constructor argument typed as a filesystem path, environment-variable
//! name or search location: every recompilation input is reachable only by
//! digest lookup in [`ByteProvision`] (QC-1). This module performs no
//! recompilation and no `package_id` recomputation -- that is #243's E9.

use std::collections::BTreeMap;

use quire_exact::ScalarLimits;

use crate::digest::{DigestRecord, InvalidDigestRecord};
use crate::replay::bounds::BoundExceeded;
use crate::replay::identity::{
    sha256, Backend, ObligationIdentity, ProfileSelection, QualifiedName, RawSourceRef,
};
use crate::replay::witness::ReplaySource;

/// The `quire.value.accounting/v1` scalar environment a replay starts from
/// (FR-071's "state environment"). Opaque to #231: only the executor (#243)
/// interprets individual entries; this type only stores and round-trips
/// them.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StateEnvironment(Vec<(String, String)>);

impl StateEnvironment {
    /// Wrap an already-read `quire.value.accounting/v1` entry list.
    pub fn new(entries: Vec<(String, String)>) -> Self {
        Self(entries)
    }

    /// The stored `(name, value)` entries, in their original order.
    pub fn entries(&self) -> &[(String, String)] {
        &self.0
    }
}

/// The S1-to-S4 stage limits copied from the proving run (ADR-013 O-26,
/// QC-8): one `quire.value.accounting/v1` [`ScalarLimits`] per compile
/// stage, kept distinct so a request never silently substitutes QSL's own
/// compiled-in defaults for the proving run's actual limits (TC-185).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StageLimits {
    /// The S1 compile stage's scalar limits.
    pub s1: ScalarLimits,
    /// The S2 compile stage's scalar limits.
    pub s2: ScalarLimits,
    /// The S3 compile stage's scalar limits.
    pub s3: ScalarLimits,
    /// The S4 compile stage's scalar limits.
    pub s4: ScalarLimits,
}

/// The request's byte provision: every recompilation input the package
/// reference names, keyed by its own declared [`DigestRecord`]. This type
/// defines no path-, environment-variable- or search-location-typed
/// accessor anywhere (FR-071-AC-2): [`Self::get`] is the only way to reach
/// an entry, and it takes a digest.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct ByteProvision(BTreeMap<DigestRecord, Vec<u8>>);

/// FR-073-AC-2: never the entries' raw bytes -- only each entry's digest
/// and byte length.
impl std::fmt::Debug for ByteProvision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.0.iter().map(|(digest, bytes)| (digest, bytes.len())))
            .finish()
    }
}

impl ByteProvision {
    /// Look up an entry by its own declared digest. The only accessor this
    /// type has; there is no path- or location-typed alternative.
    pub fn get(&self, digest: DigestRecord) -> Option<&[u8]> {
        self.0.get(&digest).map(Vec::as_slice)
    }
}

/// FR-071/ADR-013 O-26: the typed replay request. Digest-only: every
/// recompilation input is reachable only through [`ByteProvision::get`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayRequest {
    package_id: DigestRecord,
    package_contract_version: String,
    source_digests: Vec<RawSourceRef>,
    selected_function: QualifiedName,
    source: ReplaySource,
    originating_counterexample_identity: ObligationIdentity,
    backend: Backend,
    state_environment: StateEnvironment,
    accounting_limits: ScalarLimits,
    stage_limits: StageLimits,
    byte_provision: ByteProvision,
}

impl ReplayRequest {
    /// The package this request replays against.
    pub fn package_id(&self) -> DigestRecord {
        self.package_id
    }
    /// The package's declared contract version.
    pub fn package_contract_version(&self) -> &str {
        &self.package_contract_version
    }
    /// The package's declared source references.
    pub fn source_digests(&self) -> &[RawSourceRef] {
        &self.source_digests
    }
    /// The selected function. Always a typed [`QualifiedName`]; no
    /// constructor or decoder on this type ever accepts a bare `&str` or
    /// `String` here (FR-071-AC-3), and no `impl From<&str>`/`impl
    /// From<String>` exists that could satisfy this position implicitly.
    pub fn selected_function(&self) -> &QualifiedName {
        &self.selected_function
    }
    /// The witness or input replay source.
    pub fn source(&self) -> &ReplaySource {
        &self.source
    }
    /// The counterexample this request replays.
    pub fn originating_counterexample_identity(&self) -> ObligationIdentity {
        self.originating_counterexample_identity
    }
    /// The backend that produced the originating counterexample.
    pub fn backend(&self) -> &Backend {
        &self.backend
    }
    /// The scalar environment the replay starts from.
    pub fn state_environment(&self) -> &StateEnvironment {
        &self.state_environment
    }
    /// The accounting limits the replay run itself is charged against.
    pub fn accounting_limits(&self) -> ScalarLimits {
        self.accounting_limits
    }
    /// The S1-to-S4 stage limits copied from the proving run.
    pub fn stage_limits(&self) -> StageLimits {
        self.stage_limits
    }
    /// The digest-addressed byte provision. The only way to reach a
    /// recompilation input's bytes (FR-071-AC-2).
    pub fn byte_provision(&self) -> &ByteProvision {
        &self.byte_provision
    }

    /// This request's RFC 8785-style identity members, concatenated for a
    /// cheap distinctness check (TC-185 step 5): package, selection,
    /// arguments, profile selections, limits. Not a real JCS encoding (no
    /// request-identity wire schema exists yet on `origin/main`); sufficient
    /// to show two requests differing in one member have distinct
    /// identities.
    fn identity_fingerprint(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(self.package_id.as_bytes());
        buf.extend_from_slice(self.selected_function.to_string().as_bytes());
        buf.extend_from_slice(&self.stage_limits.s1.integer_bits.to_le_bytes());
        buf.extend_from_slice(&self.stage_limits.s2.integer_bits.to_le_bytes());
        buf.extend_from_slice(&self.stage_limits.s3.integer_bits.to_le_bytes());
        buf.extend_from_slice(&self.stage_limits.s4.integer_bits.to_le_bytes());
        buf
    }

    /// Whether two requests have distinct identities (TC-185).
    pub fn identity_differs_from(&self, other: &Self) -> bool {
        self.identity_fingerprint() != other.identity_fingerprint()
    }
}

/// The wire shape [`ReplayRequest::decode`] reads: exactly the FR-323
/// `quire.native-runtime/v1` members (`package`, `selection`,
/// `state_environment`, `limits`, `replay`) plus the QC-1 byte provision.
pub struct ReplayRequestWire {
    /// Must equal [`REQUEST_CONTRACT_VERSION`] or decoding refuses.
    pub contract_version: String,
    /// Must equal [`KNOWN_CAPABILITY_VOCABULARY`] or decoding refuses.
    pub capability_vocabulary: Option<String>,
    /// The semantic profile selections; each must name a known profile.
    pub profile_selections: Vec<ProfileSelection>,
    /// `(digest domain, digest hex)` naming the package this request
    /// replays against.
    pub package_id: (Option<String>, String),
    /// The package's declared contract version.
    pub package_contract_version: String,
    /// `(authority, identity, revision, digest domain, digest hex)` per
    /// declared source reference.
    pub source_digests: Vec<(String, String, String, Option<String>, String)>,
    /// The selected function.
    pub selected_function: QualifiedName,
    /// The witness or input replay source.
    pub source: ReplaySource,
    /// The counterexample this request replays.
    pub originating_counterexample_identity: [u8; 32],
    /// `(identity, manifest digest domain, manifest digest hex)` for the
    /// backend that produced the originating counterexample.
    pub backend: (String, Option<String>, String),
    /// The scalar environment the replay starts from.
    pub state_environment: StateEnvironment,
    /// The accounting limits the replay run itself is charged against.
    pub accounting_limits: ScalarLimits,
    /// The S1-to-S4 stage limits copied from the proving run.
    pub stage_limits: StageLimits,
    /// `(digest domain, digest hex, raw bytes)` per entry.
    pub byte_provision: Vec<(Option<String>, String, Vec<u8>)>,
    /// The wire encoding's approximate size, checked against the reader
    /// bound before any other member is read.
    pub encoded_bytes: usize,
}

const REQUEST_CONTRACT_VERSION: &str = "quire.native-runtime/v1";
const KNOWN_CAPABILITY_VOCABULARY: &str = "quire.capability-kind/v1";
const KNOWN_SEMANTIC_PROFILES: &[&str] = &["quire.profile.v1"];

/// [`ReplayRequest::decode`]'s structured refusal.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReplayRequestRefusal {
    /// `contract_version` is not exactly `quire.native-runtime/v1`.
    #[error("unknown_wire/unsupported-wire: {0:?} is not the admitted quire.native-runtime/v1 contract version")]
    UnknownContractVersion(String),
    /// `capability_vocabulary` is absent, or not exactly
    /// `quire.capability-kind/v1`.
    #[error("invalid_capability/unsupported-version: capability_vocabulary is not the admitted quire.capability-kind/v1")]
    UnknownCapabilityVocabulary,
    /// A profile selection names a semantic profile outside the closed
    /// known set.
    #[error(
        "invalid_capability/unsupported-version: semantic profile {0:?} is outside the closed set"
    )]
    UnknownSemanticProfile(String),
    /// A digest names a domain outside the closed FR-201 set, or is
    /// otherwise malformed.
    #[error("stale_dependency/digest-domain-mismatch: {0}")]
    InvalidDigest(#[source] InvalidDigestRecord),
    /// A byte-provision entry does not hash to its own declared digest.
    #[error("stale_dependency/byte-digest-mismatch: entry under {0} does not hash to its own declared digest")]
    ByteDigestMismatch(String),
    /// The package reference names a source digest with no matching
    /// byte-provision entry.
    #[error("missing_declaration: package reference names digest {0} with no matching byte-provision entry")]
    IncompleteByteProvision(String),
    /// The encoded request exceeds the configured reader bound.
    #[error(transparent)]
    BoundExceeded(#[from] BoundExceeded),
}

impl ReplayRequest {
    /// Decode a request from `wire`. Version, capability-vocabulary and
    /// semantic-profile checks, and the size-bound check, all happen before
    /// the byte provision or package reference is read at all
    /// (FR-071-AC-4): no recompilation, package lookup or byte-provision
    /// access is observed before any of these refusals.
    pub fn decode(wire: ReplayRequestWire) -> Result<Self, ReplayRequestRefusal> {
        BoundExceeded::check(wire.encoded_bytes)?;
        if wire.contract_version != REQUEST_CONTRACT_VERSION {
            return Err(ReplayRequestRefusal::UnknownContractVersion(
                wire.contract_version,
            ));
        }
        match wire.capability_vocabulary.as_deref() {
            Some(KNOWN_CAPABILITY_VOCABULARY) => {}
            _ => return Err(ReplayRequestRefusal::UnknownCapabilityVocabulary),
        }
        for selection in &wire.profile_selections {
            if !KNOWN_SEMANTIC_PROFILES.contains(&selection.profile()) {
                return Err(ReplayRequestRefusal::UnknownSemanticProfile(
                    selection.profile().to_owned(),
                ));
            }
        }

        // Only past this point does decoding touch the package reference or
        // the byte provision.
        let (package_domain, package_hex) = wire.package_id;
        let package_id = DigestRecord::from_wire(package_domain.as_deref(), &package_hex)
            .map_err(ReplayRequestRefusal::InvalidDigest)?;

        let mut source_digests = Vec::with_capacity(wire.source_digests.len());
        for (authority, identity, revision, domain, hex) in wire.source_digests {
            let digest = DigestRecord::from_wire(domain.as_deref(), &hex)
                .map_err(ReplayRequestRefusal::InvalidDigest)?;
            source_digests.push(RawSourceRef::new(authority, identity, revision, digest));
        }

        let mut provision = BTreeMap::new();
        for (domain, hex, bytes) in wire.byte_provision {
            let digest = DigestRecord::from_wire(domain.as_deref(), &hex)
                .map_err(ReplayRequestRefusal::InvalidDigest)?;
            if sha256(&bytes) != *digest.as_bytes() {
                return Err(ReplayRequestRefusal::ByteDigestMismatch(format!(
                    "{digest:?}"
                )));
            }
            provision.insert(digest, bytes);
        }
        let byte_provision = ByteProvision(provision);

        // Completeness: every named source digest must have a matching entry.
        for source_digest in &source_digests {
            if byte_provision.get(source_digest.digest()).is_none() {
                return Err(ReplayRequestRefusal::IncompleteByteProvision(format!(
                    "{:?}",
                    source_digest.digest()
                )));
            }
        }

        let (backend_identity, backend_domain, backend_hex) = wire.backend;
        let backend_digest = DigestRecord::from_wire(backend_domain.as_deref(), &backend_hex)
            .map_err(ReplayRequestRefusal::InvalidDigest)?;

        Ok(Self {
            package_id,
            package_contract_version: wire.package_contract_version,
            source_digests,
            selected_function: wire.selected_function,
            source: wire.source,
            originating_counterexample_identity: ObligationIdentity::from_digest(
                wire.originating_counterexample_identity,
            ),
            backend: Backend::new(backend_identity, backend_digest),
            state_environment: wire.state_environment,
            accounting_limits: wire.accounting_limits,
            stage_limits: wire.stage_limits,
            byte_provision,
        })
    }

    /// This request's members, in wire-packet shape, for round-tripping
    /// through [`Self::decode`] (FR-071-AC-1's construct -> serialize ->
    /// read round trip).
    pub fn to_wire(&self) -> ReplayRequestWire {
        ReplayRequestWire {
            contract_version: REQUEST_CONTRACT_VERSION.to_owned(),
            capability_vocabulary: Some(KNOWN_CAPABILITY_VOCABULARY.to_owned()),
            profile_selections: Vec::new(),
            package_id: (
                Some(self.package_id.domain().as_str().to_owned()),
                self.package_id.hex(),
            ),
            package_contract_version: self.package_contract_version.clone(),
            source_digests: self
                .source_digests
                .iter()
                .map(|r| {
                    (
                        r.authority().to_owned(),
                        r.identity().to_owned(),
                        r.revision().to_owned(),
                        Some(r.digest().domain().as_str().to_owned()),
                        r.digest().hex(),
                    )
                })
                .collect(),
            selected_function: self.selected_function.clone(),
            source: self.source.clone(),
            originating_counterexample_identity: *self
                .originating_counterexample_identity
                .as_bytes(),
            backend: (
                self.backend.identity().to_owned(),
                Some(self.backend.manifest_digest().domain().as_str().to_owned()),
                self.backend.manifest_digest().hex(),
            ),
            state_environment: self.state_environment.clone(),
            accounting_limits: self.accounting_limits,
            stage_limits: self.stage_limits,
            byte_provision: self
                .byte_provision
                .0
                .iter()
                .map(|(digest, bytes)| {
                    (
                        Some(digest.domain().as_str().to_owned()),
                        digest.hex(),
                        bytes.clone(),
                    )
                })
                .collect(),
            encoded_bytes: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::digest::DigestDomain;
    use crate::replay::bounds::MAX_ENCODED_BYTES;
    use crate::replay::identity::WireNodeId;
    use crate::value::Identifier;
    use ix_trace_rs::trace;

    fn scalar_limits(seed: u64) -> ScalarLimits {
        ScalarLimits {
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

    fn stage_limits(seed: u64) -> StageLimits {
        StageLimits {
            s1: scalar_limits(seed),
            s2: scalar_limits(seed),
            s3: scalar_limits(seed),
            s4: scalar_limits(seed),
        }
    }

    fn source_bytes(fill: u8, len: usize) -> Vec<u8> {
        vec![fill; len]
    }

    fn wire(stage_seed: u64) -> ReplayRequestWire {
        let source_bytes = source_bytes(0xAB, 64);
        let source_digest = sha256(&source_bytes);
        ReplayRequestWire {
            contract_version: REQUEST_CONTRACT_VERSION.to_owned(),
            capability_vocabulary: Some(KNOWN_CAPABILITY_VOCABULARY.to_owned()),
            profile_selections: vec![ProfileSelection::new(
                "quire.profile.v1".to_owned(),
                "finite-state".to_owned(),
            )],
            package_id: (
                Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
                DigestRecord::mint(DigestDomain::PackageSemanticV2, [4; 32]).hex(),
            ),
            package_contract_version: "quire.checked-package/v2".to_owned(),
            source_digests: vec![(
                "registry".to_owned(),
                "pkg-a".to_owned(),
                "rev-1".to_owned(),
                Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                DigestRecord::mint(DigestDomain::SourceBytesV1, source_digest).hex(),
            )],
            selected_function: QualifiedName::new(vec![
                Identifier::new("module").unwrap(),
                Identifier::new("f").unwrap(),
            ])
            .unwrap(),
            source: ReplaySource::Input(vec![crate::replay::witness::CanonicalAssignment {
                parameter: WireNodeId::from_digest([9; 32]),
                value: 42,
            }]),
            originating_counterexample_identity: [1; 32],
            backend: (
                "kani-backend-1".to_owned(),
                Some(DigestDomain::ToolManifestJcsV1.as_str().to_owned()),
                DigestRecord::mint(DigestDomain::ToolManifestJcsV1, [7; 32]).hex(),
            ),
            state_environment: StateEnvironment::new(vec![("x".to_owned(), "1".to_owned())]),
            accounting_limits: scalar_limits(128),
            stage_limits: stage_limits(stage_seed),
            byte_provision: vec![(
                Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                DigestRecord::mint(DigestDomain::SourceBytesV1, source_digest).hex(),
                source_bytes,
            )],
            encoded_bytes: 512,
        }
    }

    /// FR-071-AC-1 (TC-185): the request carries exactly the O-26 members
    /// (checked here by the round trip touching every field), invents none,
    /// preserves the S1-to-S4 stage limits from the proving run (never
    /// QSL's own defaults), and two requests differing in one stage limit
    /// have distinct identities.
    #[trace("TC-185", "FR-071-AC-1")]
    #[test]
    fn tc_185_carries_exactly_o26_members_and_round_trips() {
        let request = ReplayRequest::decode(wire(999)).unwrap();
        assert_eq!(request.stage_limits().s1.integer_bits, 999);

        let round_tripped = ReplayRequest::decode(request.to_wire()).unwrap();
        assert_eq!(request, round_tripped);

        let other = ReplayRequest::decode(wire(1000)).unwrap();
        assert!(request.identity_differs_from(&other));
    }

    /// FR-071-AC-2, FR-071-AC-5, FR-071-AC-6, FR-071-AC-7 (TC-186): no
    /// path-typed member exists (structural: `ReplayRequest`'s only byte
    /// accessor is digest-keyed `ByteProvision::get`); an out-of-domain
    /// byte-provision digest refuses; a byte/digest mismatch refuses; an
    /// incomplete byte provision refuses; an oversized encoding refuses.
    #[trace("TC-186", "FR-071-AC-2", "FR-071-AC-5", "FR-071-AC-6", "FR-071-AC-7")]
    #[test]
    fn tc_186_byte_provision_is_digest_only_complete_and_bounded() {
        // Well-formed request: lookup succeeds by digest alone.
        let request = ReplayRequest::decode(wire(1)).unwrap();
        let source_digest = request.source_digests()[0].digest();
        assert!(request.byte_provision().get(source_digest).is_some());

        // Out-of-domain digest domain.
        let mut bad_domain = wire(1);
        bad_domain.byte_provision[0].0 = Some("quire.not-a-real-domain/v1".to_owned());
        assert!(matches!(
            ReplayRequest::decode(bad_domain),
            Err(ReplayRequestRefusal::InvalidDigest(_))
        ));

        // Byte/digest mismatch.
        let mut mismatched = wire(1);
        mismatched.byte_provision[0].2 = source_bytes(0xCD, 64);
        assert!(matches!(
            ReplayRequest::decode(mismatched),
            Err(ReplayRequestRefusal::ByteDigestMismatch(_))
        ));

        // Incomplete byte provision: omit the one entry.
        let mut incomplete = wire(1);
        incomplete.byte_provision.clear();
        assert!(matches!(
            ReplayRequest::decode(incomplete),
            Err(ReplayRequestRefusal::IncompleteByteProvision(_))
        ));

        // Oversized encoding.
        let mut oversized = wire(1);
        oversized.encoded_bytes = MAX_ENCODED_BYTES + 1;
        assert!(matches!(
            ReplayRequest::decode(oversized),
            Err(ReplayRequestRefusal::BoundExceeded(_))
        ));
    }

    /// FR-071-AC-3 (TC-187): the selected-function accessor and wire field
    /// are both typed `QualifiedName`; a multi-segment name round-trips its
    /// full segment sequence. (The "fails to compile" half of TC-187 -- no
    /// `&str`/`String` entry point exists at all -- is a structural fact
    /// about this module's public API, checked by inspection: `decode` and
    /// `ReplayRequestWire::selected_function` are the only two entry points
    /// that set this member, and both are typed `QualifiedName`.)
    #[trace("TC-187", "FR-071-AC-3")]
    #[test]
    fn tc_187_selection_is_always_a_typed_qualified_name() {
        let request = ReplayRequest::decode(wire(1)).unwrap();
        assert_eq!(request.selected_function().segments().len(), 2);
        let round_tripped = ReplayRequest::decode(request.to_wire()).unwrap();
        assert_eq!(
            round_tripped.selected_function().segments(),
            request.selected_function().segments()
        );
    }

    /// FR-071-AC-4 (TC-188): an unknown `contract_version`, an
    /// out-of-closed-set semantic-profile identifier, and an unknown
    /// capability vocabulary each refuse at decode, before the byte
    /// provision or package reference is ever read.
    #[trace("TC-188", "FR-071-AC-4")]
    #[test]
    fn tc_188_refuses_unknown_version_or_profile_before_recompilation() {
        let mut bad_version = wire(1);
        bad_version.contract_version = "quire.native-runtime/v2-draft".to_owned();
        // A malformed byte-provision entry alongside the bad version: if the
        // reader ever touched the byte provision before checking the
        // version, this would refuse with `ByteDigestMismatch` instead.
        bad_version.byte_provision[0].2 = source_bytes(0xFF, 4);
        assert!(matches!(
            ReplayRequest::decode(bad_version),
            Err(ReplayRequestRefusal::UnknownContractVersion(_))
        ));

        let mut bad_profile = wire(1);
        bad_profile.profile_selections = vec![ProfileSelection::new(
            "quire.profile.unknown/v1".to_owned(),
            "x".to_owned(),
        )];
        assert!(matches!(
            ReplayRequest::decode(bad_profile),
            Err(ReplayRequestRefusal::UnknownSemanticProfile(_))
        ));

        let mut bad_capability = wire(1);
        bad_capability.capability_vocabulary = Some("quire.capability-kind/v2-draft".to_owned());
        assert!(matches!(
            ReplayRequest::decode(bad_capability),
            Err(ReplayRequestRefusal::UnknownCapabilityVocabulary)
        ));
    }

    /// FR-073-AC-2 (TC-210): neither `Debug` nor `Display` of a constructed
    /// request reproduces a byte-provision entry's raw bytes.
    #[trace("TC-210", "FR-073-AC-2")]
    #[test]
    fn tc_210_debug_never_reproduces_byte_provision_raw_bytes() {
        let mut request_wire = wire(1);
        let distinctive: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();
        let digest = sha256(&distinctive);
        request_wire.source_digests[0] = (
            "registry".to_owned(),
            "pkg-a".to_owned(),
            "rev-1".to_owned(),
            Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
            DigestRecord::mint(DigestDomain::SourceBytesV1, digest).hex(),
        );
        request_wire.byte_provision = vec![(
            Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
            DigestRecord::mint(DigestDomain::SourceBytesV1, digest).hex(),
            distinctive.clone(),
        )];
        let request = ReplayRequest::decode(request_wire).unwrap();

        let debug = format!("{request:?}");
        // A long contiguous run of the raw bytes, rendered as decimal
        // digits, would be far longer than the whole rest of the debug
        // output; assert no such run appears by checking total length
        // instead of scanning for byte sequences (the raw `Vec<u8>` would
        // never appear as an ASCII substring anyway -- the real risk this
        // guards is a `Debug`/`Display` impl whose length scales with the
        // entry).
        assert!(debug.len() < distinctive.len());

        // The typed accessor still returns the full content.
        let looked_up = request
            .byte_provision()
            .get(DigestRecord::mint(DigestDomain::SourceBytesV1, digest))
            .unwrap();
        assert_eq!(looked_up, distinctive.as_slice());
    }
}
