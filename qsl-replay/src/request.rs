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

use crate::bounds::BoundExceeded;
use crate::identity::{
    Backend, ObligationIdentity, ProfileSelection, QualifiedName, RawSourceRef, SourceDigestWire,
};
use crate::witness::ReplaySource;
use qsl_foundation::digest::{
    ByteDigest, DigestDomain, DigestRecord, InvalidDigestRecord, ManifestDigest,
};
use qsl_foundation::Code;
use qsl_semantics::library::LibraryName;
use qsl_semantics::model::intake::PackageDocument;

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
    /// Look up an entry by its own declared digest. The only public
    /// accessor this type has; there is no path- or location-typed
    /// alternative.
    pub fn get(&self, digest: DigestRecord) -> Option<&[u8]> {
        self.0.get(&digest).map(Vec::as_slice)
    }

    /// Every entry with its declared digest, ascending by digest: how the
    /// executor finds the domain packages the provision carries under
    /// their `sha256-jcs` digests (QC-1).
    pub(crate) fn entries(&self) -> impl Iterator<Item = (DigestRecord, &[u8])> {
        self.0
            .iter()
            .map(|(digest, bytes)| (*digest, bytes.as_slice()))
    }
}

/// One entry of the package reference's `dependencies` (QSpec FR-323,
/// ADR-015 D-4): a dependency of the proved package, with its own lock
/// `sources`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyEntry {
    identity: LibraryName,
    version: String,
    package_id: DigestRecord,
    sources: Vec<RawSourceRef>,
}

impl DependencyEntry {
    /// The library identity.
    pub fn identity(&self) -> &LibraryName {
        &self.identity
    }
    /// The library version.
    pub fn version(&self) -> &str {
        &self.version
    }
    /// The dependency's `package_id` as the proving run recorded it: a
    /// `quire.package.semantic/v2` claim, compared with a recomputed
    /// `PackageId` and never itself one (ADR-015 D-2).
    pub fn package_id(&self) -> DigestRecord {
        self.package_id
    }
    /// The dependency's own lock `sources`.
    pub fn sources(&self) -> &[RawSourceRef] {
        &self.sources
    }
}

/// A [`DependencyEntry`] in wire-packet shape: `identity`, `version`,
/// `(package_id digest domain, digest hex)` and its source references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyEntryWire {
    /// The library identity; non-empty.
    pub identity: String,
    /// The library version; non-empty.
    pub version: String,
    /// `(digest domain, digest hex)`, in `quire.package.semantic/v2`.
    pub package_id: (Option<String>, String),
    /// The dependency's lock `sources`.
    pub sources: Vec<SourceDigestWire>,
}

/// FR-071/ADR-013 O-26: the typed replay request. Digest-only: every
/// recompilation input is reachable only through [`ByteProvision::get`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayRequest {
    package_id: DigestRecord,
    package_contract_version: String,
    source_digests: Vec<RawSourceRef>,
    dependencies: Vec<DependencyEntry>,
    selected_function: QualifiedName,
    source: ReplaySource,
    profile_selections: Vec<ProfileSelection>,
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
    /// The package reference's `dependencies`, in request order (QSpec
    /// FR-323, ADR-015 D-4).
    pub fn dependencies(&self) -> &[DependencyEntry] {
        &self.dependencies
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
    /// The semantic profile selections in effect for the proving run
    /// (ADR-013 O-25/QC-8: one of the #231 envelope members O-26 carries
    /// into the request). Each was validated against the closed known set
    /// at decode time (FR-071-AC-4).
    pub fn profile_selections(&self) -> &[ProfileSelection] {
        &self.profile_selections
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
}

/// The wire shape [`ReplayRequest::decode`] reads: exactly the FR-323
/// `quire.native-runtime/v1` members (`package`, `selection`,
/// `state_environment`, `limits`, `replay`) plus the QC-1 byte provision.
pub struct ReplayRequestWire {
    /// Must equal `REQUEST_CONTRACT_VERSION` (`"quire.native-runtime/v1"`)
    /// or decoding refuses.
    pub contract_version: String,
    /// Must equal `KNOWN_CAPABILITY_VOCABULARY`
    /// (`"quire.capability-kind/v1"`) or decoding refuses.
    pub capability_vocabulary: Option<String>,
    /// The semantic profile selections; each must name a known profile.
    pub profile_selections: Vec<ProfileSelection>,
    /// `(digest domain, digest hex)` naming the package this request
    /// replays against.
    pub package_id: (Option<String>, String),
    /// The package's declared contract version.
    pub package_contract_version: String,
    /// `(authority, identity, revision namespace, revision, digest domain,
    /// digest hex)` per declared source reference.
    pub source_digests: Vec<SourceDigestWire>,
    /// The package reference's `dependencies`, one per entry of the proved
    /// package's `dependency_selections`.
    pub dependencies: Vec<DependencyEntryWire>,
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
}

const REQUEST_CONTRACT_VERSION: &str = "quire.native-runtime/v1";
/// The one package contract version this reader admits for
/// `package_contract_version` (ADR-013 O-22): the layer-4 `package` byte
/// reader's own `quire.checked-package/v2` wire (QSpec FR-322). A request
/// naming any other version refuses -- this reader never negotiates or
/// infers a version from content (QSL-235).
const PACKAGE_CONTRACT_VERSION: &str = "quire.checked-package/v2";
const KNOWN_CAPABILITY_VOCABULARY: &str = "quire.capability-kind/v1";
const KNOWN_SEMANTIC_PROFILES: &[&str] = &["quire.profile.v1"];
/// The role a semantic-profile selection fills in a replay request.
const SEMANTIC_PROFILE_ROLE: &str = "replay.semantic_profile_selections";

/// [`ReplayRequest::decode`]'s structured refusal.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReplayRequestRefusal {
    /// `contract_version` is not exactly `quire.native-runtime/v1`.
    #[error("unknown_wire/unsupported-wire: {0:?} is not the admitted quire.native-runtime/v1 contract version")]
    UnknownContractVersion(String),
    /// `package_contract_version` is not exactly `quire.checked-package/v2`
    /// (ADR-013 O-22, QSL-235): the catalog's unsupported-wire refusal,
    /// naming the actual version the request carried.
    #[error("unknown_wire/unsupported-wire: {0:?} is not the admitted quire.checked-package/v2 package contract version")]
    UnknownPackageContractVersion(String),
    /// `capability_vocabulary` is absent, or not exactly
    /// `quire.capability-kind/v1`.
    #[error("invalid_capability/unsupported-version: capability_vocabulary is not the admitted quire.capability-kind/v1")]
    UnknownCapabilityVocabulary,
    /// A profile selection names a semantic profile outside the closed
    /// known set: catalog `unknown_profile`/`unsupported-selection`
    /// (revision `1-draft.3`), not a capability-vocabulary refusal. It
    /// retains the supplied selection and the role it was supplied for, as
    /// that catalog row requires.
    #[error(
        "unknown_profile/unsupported-selection: semantic profile {:?} (value {:?}) is outside the closed set for role {required_role}",
        selection.profile(),
        selection.value()
    )]
    UnknownSemanticProfile {
        /// The supplied selection.
        selection: ProfileSelection,
        /// The role the selection was supplied for.
        required_role: &'static str,
    },
    /// A digest names a domain outside the closed FR-201 set, or supplies no
    /// domain at all.
    #[error("stale_dependency/digest-domain-mismatch: {0}")]
    DigestDomainMismatch(#[source] InvalidDigestRecord),
    /// A digest names a domain FR-201 admits, but its own hex encoding is
    /// malformed (wrong length, or not lowercase hex) -- not a domain
    /// problem, so it is never spelled `digest-domain-mismatch`.
    #[error("invalid_digest/malformed-encoding: {0}")]
    MalformedDigest(#[source] InvalidDigestRecord),
    /// A byte-provision entry names an FR-201 domain that is neither
    /// raw-byte-addressed nor `sha256-jcs` (e.g. a structural-preimage
    /// domain): this reader recomputes only those, so such a domain can
    /// never be verified here and is refused before any hashing is
    /// attempted, distinct from an actual byte/digest mismatch.
    #[error("invalid_digest/ineligible-domain: {0} is neither a raw-byte-addressed digest domain nor sha256-jcs and cannot be admitted as a byte-provision entry")]
    IneligibleByteProvisionDomain(DigestDomain),
    /// A byte-provision entry under a `sha256-jcs` digest is not a domain
    /// package document at all, so it has no `sha256-jcs` digest to match.
    #[error("invalid_model_binding/malformed-declaration: entry under {0} is not a domain package document")]
    NotAPackageDocument(String),
    /// A byte-provision entry does not hash to its own declared digest.
    #[error("stale_dependency/byte-digest-mismatch: entry under {0} does not hash to its own declared digest")]
    ByteDigestMismatch(String),
    /// The package reference names a source digest with no matching
    /// byte-provision entry.
    #[error("missing_declaration: package reference names digest {0} with no matching byte-provision entry")]
    IncompleteByteProvision(String),
    /// A `dependencies` entry has an empty identity or version
    /// (`invalid_identifier`, FR-071-AC-9).
    #[error("invalid_identifier: dependencies entry {index} has an empty identity or version")]
    EmptyDependencySelection {
        /// The entry's position in `dependencies`.
        index: usize,
    },
    /// The encoded request exceeds the configured reader bound.
    #[error(transparent)]
    BoundExceeded(#[from] BoundExceeded),
}

/// `bytes`' digest under `domain`: SHA-256 of the raw bytes for a
/// raw-byte-addressed domain (source and definition documents), and for
/// `sha256-jcs` the domain package document's own digest, SHA-256 of its
/// RFC 8785 bytes, exactly as I1 keys its package input
/// (`model::intake::package_input`, ADR-013 QC-1). Bytes that are not a
/// domain package document refuse as such, and every other domain refuses
/// as ineligible.
fn digest_of(digest: DigestRecord, bytes: &[u8]) -> Result<[u8; 32], ReplayRequestRefusal> {
    let domain = digest.domain();
    if domain == DigestDomain::Sha256Jcs {
        return PackageDocument::parse(bytes)
            .map(|document| document.jcs_digest())
            .map_err(|_| ReplayRequestRefusal::NotAPackageDocument(format!("{digest:?}")));
    }
    if domain.is_raw_byte_addressed() {
        return Ok(ByteDigest::of(bytes).as_bytes());
    }
    Err(ReplayRequestRefusal::IneligibleByteProvisionDomain(domain))
}

impl ReplayRequestRefusal {
    /// The catalog code of this refusal. The native catalog has no
    /// `invalid_capability` code, so an unknown capability vocabulary
    /// reports `unknown_wire`, the code of an unadmitted wire version.
    pub fn code(&self) -> Code {
        match self {
            Self::UnknownContractVersion(_)
            | Self::UnknownCapabilityVocabulary
            | Self::UnknownPackageContractVersion(_) => Code::UnknownWire,
            Self::UnknownSemanticProfile { .. } => Code::UnknownProfile,
            Self::DigestDomainMismatch(_) | Self::ByteDigestMismatch(_) => Code::StaleDependency,
            Self::MalformedDigest(_) | Self::IneligibleByteProvisionDomain(_) => {
                Code::InvalidDigest
            }
            Self::NotAPackageDocument(_) => Code::InvalidModelBinding,
            Self::IncompleteByteProvision(_) => Code::MissingDeclaration,
            Self::EmptyDependencySelection { .. } => Code::InvalidIdentifier,
            Self::BoundExceeded(_) => Code::StageLimitExceeded,
        }
    }
}

/// Route `err` to [`ReplayRequestRefusal::DigestDomainMismatch`] or
/// [`ReplayRequestRefusal::MalformedDigest`] by its real cause, so a wrong
/// hex length is never reported as a domain problem (N5).
fn classify_digest_error(err: InvalidDigestRecord) -> ReplayRequestRefusal {
    if err.is_domain_mismatch() {
        ReplayRequestRefusal::DigestDomainMismatch(err)
    } else {
        ReplayRequestRefusal::MalformedDigest(err)
    }
}

/// The reader's own measurement of `wire`'s encoded size (FR-071-AC-7): the
/// sum of every variable-length member's own byte length -- the
/// byte-provision entries' actual raw bytes among them -- never a
/// caller-declared number a request could understate to launder an
/// oversized byte provision past the bound (B3).
fn measured_encoded_bytes(wire: &ReplayRequestWire) -> usize {
    wire.contract_version.len()
        + wire.capability_vocabulary.as_deref().map_or(0, str::len)
        + wire
            .profile_selections
            .iter()
            .map(|p| p.profile().len() + p.value().len())
            .sum::<usize>()
        + wire.package_id.1.len()
        + wire.package_contract_version.len()
        + wire
            .source_digests
            .iter()
            .map(RawSourceRef::wire_len)
            .sum::<usize>()
        + wire
            .dependencies
            .iter()
            .map(|entry| {
                entry.identity.len()
                    + entry.version.len()
                    + entry.package_id.1.len()
                    + entry
                        .sources
                        .iter()
                        .map(RawSourceRef::wire_len)
                        .sum::<usize>()
            })
            .sum::<usize>()
        + wire.selected_function.to_string().len()
        + wire.backend.0.len()
        + wire.backend.2.len()
        + wire
            .state_environment
            .entries()
            .iter()
            .map(|(name, value)| name.len() + value.len())
            .sum::<usize>()
        + wire
            .byte_provision
            .iter()
            .map(|(_, hex, bytes)| hex.len() + bytes.len())
            .sum::<usize>()
        + match &wire.source {
            ReplaySource::Witness(witness) => witness.transcript().len(),
            // A 32-byte node id plus an 8-byte integer value per entry
            // (QC-1's digest-addressed shape), a fixed size independent of
            // any `Debug`-rendered text.
            ReplaySource::Input(assignments) => assignments.len() * (32 + 8),
        }
}

impl ReplayRequest {
    /// Decode a request from `wire`. Version, capability-vocabulary and
    /// semantic-profile checks, and the size-bound check, all happen before
    /// the byte provision or package reference is read at all
    /// (FR-071-AC-4, FR-071-AC-8): no recompilation, package lookup or
    /// byte-provision access is observed before any of these refusals.
    pub fn decode(wire: ReplayRequestWire) -> Result<Self, ReplayRequestRefusal> {
        BoundExceeded::check(measured_encoded_bytes(&wire))?;
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
                return Err(ReplayRequestRefusal::UnknownSemanticProfile {
                    selection: selection.clone(),
                    required_role: SEMANTIC_PROFILE_ROLE,
                });
            }
        }
        // ADR-013 O-22 (QSL-235): the package's own declared contract
        // version is checked here too, before the package reference or byte
        // provision is read, exactly like the request envelope's own
        // `contract_version` above.
        if wire.package_contract_version != PACKAGE_CONTRACT_VERSION {
            return Err(ReplayRequestRefusal::UnknownPackageContractVersion(
                wire.package_contract_version,
            ));
        }

        // Only past this point does decoding touch the package reference or
        // the byte provision.
        let (package_domain, package_hex) = wire.package_id;
        let package_id = DigestRecord::from_wire(package_domain.as_deref(), &package_hex)
            .map_err(classify_digest_error)?;

        let source_digests = wire
            .source_digests
            .into_iter()
            .map(RawSourceRef::from_wire)
            .collect::<Result<Vec<_>, _>>()
            .map_err(classify_digest_error)?;

        let dependencies = wire
            .dependencies
            .into_iter()
            .enumerate()
            .map(|(index, entry)| {
                let empty = || ReplayRequestRefusal::EmptyDependencySelection { index };
                let identity = LibraryName::new(entry.identity).map_err(|_| empty())?;
                if entry.version.is_empty() {
                    return Err(empty());
                }
                let (domain, hex) = entry.package_id;
                let package_id = DigestRecord::from_wire_expecting(
                    DigestDomain::PackageSemanticV2,
                    domain.as_deref(),
                    &hex,
                )
                .map_err(classify_digest_error)?;
                let sources = entry
                    .sources
                    .into_iter()
                    .map(RawSourceRef::from_wire)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(classify_digest_error)?;
                Ok(DependencyEntry {
                    identity,
                    version: entry.version,
                    package_id,
                    sources,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut provision = BTreeMap::new();
        for (domain, hex, bytes) in wire.byte_provision {
            let digest =
                DigestRecord::from_wire(domain.as_deref(), &hex).map_err(classify_digest_error)?;
            // N3: only a domain this reader can recompute passes the check
            // below. A structural domain digests a form this reader never
            // builds, so admitting one here would always refuse with a
            // misleading staleness cause instead of the real "wrong domain
            // for this position" one.
            if digest_of(digest, &bytes)? != *digest.as_bytes() {
                return Err(ReplayRequestRefusal::ByteDigestMismatch(format!(
                    "{digest:?}"
                )));
            }
            provision.insert(digest, bytes);
        }
        let byte_provision = ByteProvision(provision);

        // Completeness: every named source digest, the proved package's and
        // each dependency's, must have a matching entry (FR-071-AC-5, AC-9).
        for source_digest in source_digests
            .iter()
            .chain(dependencies.iter().flat_map(|entry| &entry.sources))
        {
            if byte_provision.get(source_digest.digest()).is_none() {
                return Err(ReplayRequestRefusal::IncompleteByteProvision(format!(
                    "{:?}",
                    source_digest.digest()
                )));
            }
        }

        let (backend_identity, backend_domain, backend_hex) = wire.backend;
        // ADR-013 C-27 (QSL-227): the backend digest is not just any FR-201
        // domain -- it must be `quire.tool-manifest.jcs/v1`, checked before
        // the hex bytes are read. `ManifestDigest::from_wire` cannot
        // construct anything else.
        let backend_digest = ManifestDigest::from_wire(backend_domain.as_deref(), &backend_hex)
            .map_err(classify_digest_error)?;

        Ok(Self {
            package_id,
            package_contract_version: wire.package_contract_version,
            source_digests,
            dependencies,
            selected_function: wire.selected_function,
            source: wire.source,
            profile_selections: wire.profile_selections,
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
            profile_selections: self.profile_selections.clone(),
            package_id: (
                Some(self.package_id.domain().as_str().to_owned()),
                self.package_id.hex(),
            ),
            package_contract_version: self.package_contract_version.clone(),
            source_digests: self
                .source_digests
                .iter()
                .map(RawSourceRef::to_wire)
                .collect(),
            dependencies: self
                .dependencies
                .iter()
                .map(|entry| DependencyEntryWire {
                    identity: entry.identity.as_str().to_owned(),
                    version: entry.version.clone(),
                    package_id: (
                        Some(entry.package_id.domain().as_str().to_owned()),
                        entry.package_id.hex(),
                    ),
                    sources: entry.sources.iter().map(RawSourceRef::to_wire).collect(),
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bounds::MAX_ENCODED_BYTES;
    use ix_trace_rs::trace;
    use qsl_foundation::digest::{ByteDigest, DigestDomain, WireNodeId};
    use quire_exact::Identifier;

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
        let source_digest = ByteDigest::of(&source_bytes).as_bytes();
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
                "git".to_owned(),
                "rev-1".to_owned(),
                Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                DigestRecord::mint(DigestDomain::SourceBytesV1, source_digest).hex(),
            )],
            dependencies: Vec::new(),
            selected_function: QualifiedName::new(vec![
                Identifier::new("module").unwrap(),
                Identifier::new("f").unwrap(),
            ])
            .unwrap(),
            source: ReplaySource::Input(vec![crate::witness::CanonicalAssignment {
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
        }
    }

    /// FR-071-AC-1 (TC-185): the request carries exactly the O-26 members
    /// (checked here by the round trip touching every field, including the
    /// semantic profile selections B1 restored -- `to_wire` used to emit an
    /// empty `Vec` regardless of what `decode` read, silently discarding the
    /// validated selections), invents none, preserves the S1-to-S4 stage
    /// limits from the proving run (never QSL's own defaults), and two
    /// requests differing in one stage limit compare unequal under the
    /// type's own structural `PartialEq` (ADR-013 O-26's "Equality" row).
    #[trace("TC-185", "FR-071-AC-1")]
    #[test]
    fn tc_185_carries_exactly_o26_members_and_round_trips() {
        let request = ReplayRequest::decode(wire(999)).unwrap();
        assert_eq!(request.stage_limits().s1.integer_bits, 999);
        assert_eq!(
            request.profile_selections(),
            &[ProfileSelection::new(
                "quire.profile.v1".to_owned(),
                "finite-state".to_owned(),
            )]
        );

        let round_tripped = ReplayRequest::decode(request.to_wire()).unwrap();
        assert_eq!(request, round_tripped);
        assert_eq!(
            round_tripped.profile_selections(),
            request.profile_selections()
        );

        let other = ReplayRequest::decode(wire(1000)).unwrap();
        assert_ne!(request, other);
    }

    /// FR-071-AC-2, FR-071-AC-5, FR-071-AC-6, FR-071-AC-7 (TC-186): no
    /// path-typed member exists (structural: `ReplayRequest`'s only byte
    /// accessor is digest-keyed `ByteProvision::get`); an out-of-domain
    /// byte-provision digest refuses; a byte/digest mismatch refuses; an
    /// incomplete byte provision refuses; an oversized encoding refuses.
    /// A `dependencies` entry over one `fill`-byte source under `identity`.
    fn dependency(identity: &str, version: &str, fill: u8) -> (DependencyEntryWire, Vec<u8>) {
        let bytes = source_bytes(fill, 32);
        let digest = DigestRecord::mint(
            DigestDomain::SourceBytesV1,
            ByteDigest::of(&bytes).as_bytes(),
        );
        (
            DependencyEntryWire {
                identity: identity.to_owned(),
                version: version.to_owned(),
                package_id: (
                    Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
                    DigestRecord::mint(DigestDomain::PackageSemanticV2, [fill; 32]).hex(),
                ),
                sources: vec![(
                    "registry".to_owned(),
                    format!("lib-{fill}"),
                    "git".to_owned(),
                    "rev-1".to_owned(),
                    Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                    digest.hex(),
                )],
            },
            bytes,
        )
    }

    /// `wire(1)` with `entries` as its `dependencies`, each entry's source
    /// in the byte provision.
    fn with_dependencies(entries: Vec<(DependencyEntryWire, Vec<u8>)>) -> ReplayRequestWire {
        let mut wire = wire(1);
        for (entry, bytes) in entries {
            let (domain, hex) = (entry.sources[0].4.clone(), entry.sources[0].5.clone());
            wire.byte_provision.push((domain, hex, bytes));
            wire.dependencies.push(entry);
        }
        wire
    }

    /// FR-071-AC-9 (TC-186): two `dependencies` entries round-trip exactly,
    /// in order. An entry's source with no byte-provision entry refuses at
    /// construction; an empty identity or version, and a `package_id` in
    /// the `quire.source.bytes/v1` domain, refuse at decode.
    #[trace("TC-186", "FR-071-AC-9")]
    #[test]
    fn tc_186_dependencies_round_trip_and_refuse_at_decode() {
        let entries = vec![
            dependency("test/b", "2", 0x0B),
            dependency("test/a", "1", 0x0A),
        ];
        let request = ReplayRequest::decode(with_dependencies(entries.clone())).unwrap();
        let identities: Vec<&str> = request
            .dependencies()
            .iter()
            .map(|entry| entry.identity().as_str())
            .collect();
        assert_eq!(identities, ["test/b", "test/a"]);
        assert_eq!(request.dependencies()[0].version(), "2");
        let wire = request.to_wire();
        assert_eq!(
            wire.dependencies,
            entries
                .iter()
                .map(|(entry, _)| entry.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(ReplayRequest::decode(wire).unwrap(), request);

        // An entry's source absent from the byte provision.
        let mut incomplete = with_dependencies(vec![dependency("test/a", "1", 0x0A)]);
        incomplete.byte_provision.pop();
        assert!(matches!(
            ReplayRequest::decode(incomplete),
            Err(ReplayRequestRefusal::IncompleteByteProvision(_))
        ));

        // An empty identity, then an empty version.
        for (identity, version) in [("", "1"), ("test/a", "")] {
            let refused =
                ReplayRequest::decode(with_dependencies(vec![dependency(identity, version, 0x0A)]))
                    .unwrap_err();
            assert_eq!(
                refused,
                ReplayRequestRefusal::EmptyDependencySelection { index: 0 }
            );
            assert_eq!(refused.code(), Code::InvalidIdentifier);
        }

        // A `package_id` in the source-bytes domain.
        let (mut entry, bytes) = dependency("test/a", "1", 0x0A);
        entry.package_id.0 = Some(DigestDomain::SourceBytesV1.as_str().to_owned());
        assert!(matches!(
            ReplayRequest::decode(with_dependencies(vec![(entry, bytes)])),
            Err(ReplayRequestRefusal::DigestDomainMismatch(_))
        ));
    }

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
            Err(ReplayRequestRefusal::DigestDomainMismatch(_))
        ));

        // N5: a valid FR-201 domain with a malformed hex encoding refuses
        // distinctly from a domain mismatch -- never spelled
        // `digest-domain-mismatch`, since the domain itself named no
        // problem.
        let mut malformed_hex = wire(1);
        malformed_hex.byte_provision[0].1 = "ab".repeat(31); // 62 chars, not 64
        assert!(matches!(
            ReplayRequest::decode(malformed_hex),
            Err(ReplayRequestRefusal::MalformedDigest(_))
        ));

        // N3: a valid FR-201 domain this reader cannot recompute (a
        // structural domain, which digests a form this reader never builds)
        // refuses with the real cause instead of a spurious
        // `byte-digest-mismatch`.
        let mut ineligible_domain = wire(1);
        ineligible_domain.byte_provision[0].0 =
            Some(DigestDomain::CheckedSemanticNodeV1.as_str().to_owned());
        ineligible_domain.byte_provision[0].1 =
            DigestRecord::mint(DigestDomain::CheckedSemanticNodeV1, [0xAB; 32]).hex();
        assert!(matches!(
            ReplayRequest::decode(ineligible_domain),
            Err(ReplayRequestRefusal::IneligibleByteProvisionDomain(
                DigestDomain::CheckedSemanticNodeV1
            ))
        ));

        // QC-1: a domain package document enters under its `sha256-jcs`
        // digest, verified over its RFC 8785 bytes; other bytes under that
        // digest refuse as a byte/digest mismatch.
        let document = br#"{"b": 1, "a": [true]}"#.to_vec();
        let jcs = qsl_semantics::model::intake::PackageDocument::parse(&document)
            .expect("the document parses")
            .jcs_digest();
        let mut domain_package = wire(1);
        domain_package.byte_provision.push((
            Some(DigestDomain::Sha256Jcs.as_str().to_owned()),
            DigestRecord::mint(DigestDomain::Sha256Jcs, jcs).hex(),
            document,
        ));
        let admitted = ReplayRequest::decode(domain_package).unwrap();
        assert!(admitted
            .byte_provision()
            .get(DigestRecord::mint(DigestDomain::Sha256Jcs, jcs))
            .is_some());
        let mut stale_package = wire(1);
        stale_package.byte_provision.push((
            Some(DigestDomain::Sha256Jcs.as_str().to_owned()),
            DigestRecord::mint(DigestDomain::Sha256Jcs, jcs).hex(),
            br#"{"b": 2}"#.to_vec(),
        ));
        assert!(matches!(
            ReplayRequest::decode(stale_package),
            Err(ReplayRequestRefusal::ByteDigestMismatch(_))
        ));
        let mut not_a_document = wire(1);
        not_a_document.byte_provision.push((
            Some(DigestDomain::Sha256Jcs.as_str().to_owned()),
            DigestRecord::mint(DigestDomain::Sha256Jcs, jcs).hex(),
            b"not json".to_vec(),
        ));
        let refused = ReplayRequest::decode(not_a_document).unwrap_err();
        assert!(matches!(
            refused,
            ReplayRequestRefusal::NotAPackageDocument(_)
        ));
        assert_eq!(refused.code(), Code::InvalidModelBinding);

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

        // Oversized encoding. B3: the bound check measures the wire value's
        // own content -- there is no `encoded_bytes` field a caller could
        // understate -- so an oversized request has to actually carry
        // oversized content.
        let mut oversized = wire(1);
        oversized.package_contract_version = "x".repeat(MAX_ENCODED_BYTES + 1);
        assert!(matches!(
            ReplayRequest::decode(oversized),
            Err(ReplayRequestRefusal::BoundExceeded(_))
        ));
    }

    /// QSL-227 positive control: a real request whose `backend` digest is in
    /// the required `quire.tool-manifest.jcs/v1` domain (as `wire` already
    /// builds it) decodes, and the resulting request's backend carries that
    /// domain.
    #[test]
    fn backend_digest_in_the_required_domain_decodes() {
        let request = ReplayRequest::decode(wire(1)).unwrap();
        assert_eq!(
            request.backend().manifest_digest().domain(),
            DigestDomain::ToolManifestJcsV1
        );
    }

    /// QSL-227 (ADR-013 C-27): a `backend` digest in a recognized FR-201
    /// domain other than `quire.tool-manifest.jcs/v1` refuses with the same
    /// typed cause the reader already uses for a byte-provision domain
    /// mismatch, pinned to the exact `WrongDomain` cause so a different
    /// refusal variant cannot pass, and the domain is checked before the
    /// digest bytes: pairing the wrong domain with malformed hex still
    /// reports the same domain mismatch, not a hex-encoding problem. No
    /// FR-071 AC names the replay request's `backend` member, so this test
    /// is untraced here; it is recorded at ADR-013 C-27 instead.
    #[test]
    fn backend_digest_in_any_other_fr201_domain_refuses() {
        let wrong_domain =
            ReplayRequestRefusal::DigestDomainMismatch(InvalidDigestRecord::WrongDomain {
                expected: DigestDomain::ToolManifestJcsV1,
                found: DigestDomain::SourceBytesV1,
            });

        let mut bad_domain = wire(1);
        bad_domain.backend.1 = Some(DigestDomain::SourceBytesV1.as_str().to_owned());
        assert_eq!(ReplayRequest::decode(bad_domain), Err(wrong_domain.clone()));

        let mut bad_domain_and_hex = wire(1);
        bad_domain_and_hex.backend.1 = Some(DigestDomain::SourceBytesV1.as_str().to_owned());
        bad_domain_and_hex.backend.2 = "not-hex".to_owned();
        assert_eq!(ReplayRequest::decode(bad_domain_and_hex), Err(wrong_domain));
    }

    /// FR-071-AC-3 (TC-187): the selected-function accessor and wire field
    /// are both typed `QualifiedName`; a multi-segment name round-trips its
    /// full segment sequence. (The "fails to compile" half of TC-187 -- no
    /// `&str`/`String` entry point exists at all -- is a structural fact
    /// about this module's public API, checked by inspection: `decode` and
    /// `ReplayRequestWire::selected_function` are the only two entry points
    /// that set this member, and both are typed `QualifiedName`.)
    ///
    /// N1: no `#[trace]` tag -- this test cannot fail on TC-187/FR-071-AC-3's
    /// full claim (that no bare-string entry point exists at all), only on
    /// the positive round-trip half; `spec/tests.md` keeps the row `Planned`.
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
        // The refusal reports the catalog's profile-selection code and
        // cause, and retains the supplied selection and its required role.
        let refused = ReplayRequest::decode(bad_profile).map(|_| ()).unwrap_err();
        let ReplayRequestRefusal::UnknownSemanticProfile {
            selection,
            required_role,
        } = &refused
        else {
            panic!("expected UnknownSemanticProfile, got {refused:?}");
        };
        assert_eq!(selection.profile(), "quire.profile.unknown/v1");
        assert_eq!(selection.value(), "x");
        assert_eq!(*required_role, "replay.semantic_profile_selections");
        assert_eq!(
            refused.to_string(),
            "unknown_profile/unsupported-selection: semantic profile \"quire.profile.unknown/v1\" (value \"x\") is outside the closed set for role replay.semantic_profile_selections"
        );

        let mut bad_capability = wire(1);
        bad_capability.capability_vocabulary = Some("quire.capability-kind/v2-draft".to_owned());
        assert!(matches!(
            ReplayRequest::decode(bad_capability),
            Err(ReplayRequestRefusal::UnknownCapabilityVocabulary)
        ));
    }

    /// FR-071-AC-8 (TC-445, QSL-235, ADR-013 O-22): a real, otherwise
    /// well-formed request wire mutated to an unknown
    /// `package_contract_version` refuses with the catalog's
    /// unsupported-wire refusal, naming the actual version supplied, before
    /// the package reference or byte provision is read -- a malformed
    /// byte-provision entry alongside the bad version proves the ordering,
    /// exactly like `tc_188` proves it for `contract_version`.
    #[trace("TC-445", "FR-071-AC-8")]
    #[test]
    fn tc_445_refuses_an_unknown_package_contract_version() {
        let mut bad_package_version = wire(1);
        bad_package_version.package_contract_version = "quire.checked-package/v3".to_owned();
        bad_package_version.byte_provision[0].2 = source_bytes(0xFF, 4);
        assert_eq!(
            ReplayRequest::decode(bad_package_version),
            Err(ReplayRequestRefusal::UnknownPackageContractVersion(
                "quire.checked-package/v3".to_owned()
            ))
        );
        assert_eq!(
            ReplayRequestRefusal::UnknownPackageContractVersion(
                "quire.checked-package/v3".to_owned()
            )
            .to_string(),
            "unknown_wire/unsupported-wire: \"quire.checked-package/v3\" is not the admitted quire.checked-package/v2 package contract version"
        );
        assert_eq!(
            ReplayRequestRefusal::UnknownPackageContractVersion("x".to_owned()).code(),
            Code::UnknownWire
        );
    }

    /// FR-073-AC-2 (TC-210): neither `Debug` nor `Display` of a constructed
    /// request reproduces a byte-provision entry's raw bytes, or a
    /// concrete-argument value from an `Input`-arm [`ReplaySource`] (B2: the
    /// leak `derive(Debug)` used to reproduce through
    /// `ReplaySource::Input(Vec<CanonicalAssignment>)`).
    #[trace("TC-210", "FR-073-AC-2")]
    #[test]
    fn tc_210_debug_never_reproduces_byte_provision_raw_bytes() {
        let mut request_wire = wire(1);
        let distinctive: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();
        let digest = ByteDigest::of(&distinctive).as_bytes();
        request_wire.source_digests[0] = (
            "registry".to_owned(),
            "pkg-a".to_owned(),
            "git".to_owned(),
            "rev-1".to_owned(),
            Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
            DigestRecord::mint(DigestDomain::SourceBytesV1, digest).hex(),
        );
        request_wire.byte_provision = vec![(
            Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
            DigestRecord::mint(DigestDomain::SourceBytesV1, digest).hex(),
            distinctive.clone(),
        )];
        // B2's second half: a distinctive concrete-argument value on the
        // `Input` arm.
        let distinctive_value: i64 = 918_273_645;
        request_wire.source = ReplaySource::Input(vec![crate::witness::CanonicalAssignment {
            parameter: WireNodeId::from_digest([42; 32]),
            value: distinctive_value,
        }]);
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
        // The planted concrete argument value does not appear either.
        assert!(!debug.contains(&distinctive_value.to_string()));

        // The typed accessors still return the full content.
        let looked_up = request
            .byte_provision()
            .get(DigestRecord::mint(DigestDomain::SourceBytesV1, digest))
            .unwrap();
        assert_eq!(looked_up, distinctive.as_slice());
        match request.source() {
            ReplaySource::Input(assignments) => {
                assert_eq!(assignments[0].value, distinctive_value);
            }
            ReplaySource::Witness(_) => panic!("expected the Input arm"),
        }
    }
}
