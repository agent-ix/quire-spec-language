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
use qsl_foundation::diagnostic::JsonPointer;
use qsl_foundation::digest::{
    ByteDigest, DigestDomain, DigestRecord, InvalidDigestRecord, ManifestDigest,
};
use qsl_foundation::Code;
use qsl_semantics::library::LibraryName;
use qsl_semantics::model::intake::PackageDocument;
use qsl_semantics::model::normalize::ModelRefusal;
use qsl_semantics::model::refusal::{Inexact, IntakeLimit, ModelRefusalCause};

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
    package_id: DigestRecord,
    sources: Vec<RawSourceRef>,
}

impl DependencyEntry {
    /// The library identity.
    pub fn identity(&self) -> &LibraryName {
        &self.identity
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

/// A [`DependencyEntry`] in wire-packet shape: `identity`,
/// `(package_id digest domain, digest hex)` and its source references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyEntryWire {
    /// The library identity; non-empty.
    pub identity: String,
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
    source_digests: Vec<RawSourceRef>,
    dependencies: Vec<DependencyEntry>,
    selected_function: QualifiedName,
    source: ReplaySource,
    profile_selections: Vec<ProfileSelection>,
    obligation_identity: ObligationIdentity,
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
    /// at decode time.
    pub fn profile_selections(&self) -> &[ProfileSelection] {
        &self.profile_selections
    }
    /// The obligation this request replays: the ADR-013 O-09 obligation
    /// digest, as on the frame and state-clause paths (ADR-013 O-26).
    pub fn obligation_identity(&self) -> ObligationIdentity {
        self.obligation_identity
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

/// The in-process shape [`ReplayRequest::decode`] reads: the FR-323
/// members (`package`, `selection`, `state_environment`, `limits`,
/// `replay`) plus the QC-1 byte provision. It is never read from bytes, so
/// it carries no contract version or vocabulary member: the version check
/// belongs to a byte reader, and none exists yet.
pub struct ReplayRequestWire {
    /// The semantic profile selections; each must name a known profile.
    pub profile_selections: Vec<ProfileSelection>,
    /// `(digest domain, digest hex)` naming the package this request
    /// replays against.
    pub package_id: (Option<String>, String),
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
    /// The obligation this request replays: the ADR-013 O-09 obligation
    /// digest.
    pub obligation_identity: [u8; 32],
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

const KNOWN_SEMANTIC_PROFILES: &[&str] = &["quire.profile.v1"];
/// The role a semantic-profile selection fills in a replay request.
const SEMANTIC_PROFILE_ROLE: &str = "replay.semantic_profile_selections";

/// [`ReplayRequest::decode`]'s structured refusal.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReplayRequestRefusal {
    /// A profile selection names a semantic profile outside the closed
    /// known set: catalog `unknown_profile`/`unsupported-selection`. It
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
    /// A byte-provision entry under a `sha256-jcs` digest reached one of
    /// intake's limits while it was read for its digest: intake's own limit
    /// outcome, `resource_exhausted`/`intake-limit-exceeded` (FR-056).
    #[error(
        "resource_exhausted/intake-limit-exceeded: entry under {entry} exceeds intake's {} limit of {bound}",
        limit.as_str()
    )]
    IntakeLimitExceeded {
        /// The entry's declared digest.
        entry: String,
        /// The limit reached.
        limit: IntakeLimit,
        /// That limit's bound.
        bound: usize,
    },
    /// Reading or digesting a byte-provision entry under a `sha256-jcs`
    /// digest could not reserve memory: `resource_exhausted`/
    /// `allocation-failed` (FR-259 B6).
    #[error("resource_exhausted/allocation-failed: entry under {entry} could not reserve {requested} bytes of memory")]
    AllocationFailed {
        /// The entry's declared digest.
        entry: String,
        /// The size in bytes of the reservation that failed.
        requested: usize,
    },
    /// A byte-provision entry under a `sha256-jcs` digest holds a number
    /// with no exact RFC 8785 spelling: intake's own refusal,
    /// `noncanonical_wire` with its cause and `document_pointer` (FR-056).
    #[error(
        "noncanonical_wire/{}: entry under {entry} holds a number at {:?} with no exact RFC 8785 spelling",
        inexact.as_str(),
        document_pointer.as_str()
    )]
    NoncanonicalNumber {
        /// The entry's declared digest.
        entry: String,
        /// Why the number has no exact spelling; the cause tag.
        inexact: Inexact,
        /// The RFC 6901 pointer of the first such number in the entry's
        /// document.
        document_pointer: JsonPointer,
    },
    /// A byte-provision entry under a raw-byte domain does not hash to its
    /// own declared digest: the raw bytes' own digest differs.
    #[error("stale_dependency/byte-digest-mismatch: entry under {0} does not hash to its own declared digest")]
    ByteDigestMismatch(String),
    /// A `sha256-jcs` byte-provision entry's document recomputes to a digest
    /// other than the one it is provided under
    /// (`stale_dependency`/`content-mismatch`).
    #[error(
        "stale_dependency/content-mismatch: entry under {selected} recomputes to {recomputed}"
    )]
    ContentMismatch {
        /// The digest the entry is provided under.
        selected: String,
        /// The digest recomputed from the supplied document.
        recomputed: String,
    },
    /// The package reference names a source digest with no matching
    /// byte-provision entry.
    #[error("missing_declaration: package reference names digest {0} with no matching byte-provision entry")]
    IncompleteByteProvision(String),
    /// A `dependencies` entry has an empty identity
    /// (`invalid_identifier`, FR-071-AC-9).
    #[error("invalid_identifier: dependencies entry {index} has an empty identity")]
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
/// (`model::intake::package_input`, ADR-013 QC-1). A refusal of intake's
/// read keeps its cause ([`package_document_refusal`]), and every other
/// domain refuses as ineligible.
fn digest_of(digest: DigestRecord, bytes: &[u8]) -> Result<[u8; 32], ReplayRequestRefusal> {
    let domain = digest.domain();
    if domain == DigestDomain::Sha256Jcs {
        return PackageDocument::parse(bytes)
            .map(|document| document.jcs_digest())
            .map_err(|refusal| package_document_refusal(digest, refusal));
    }
    if domain.is_raw_byte_addressed() {
        return Ok(ByteDigest::of(bytes).as_bytes());
    }
    Err(ReplayRequestRefusal::IneligibleByteProvisionDomain(domain))
}

/// The refusal for a `sha256-jcs` entry under `digest` that intake's read
/// refused, by its cause: an intake limit stays intake's limit outcome, an
/// allocation failure stays `allocation-failed` (FR-259 B6), a number with
/// no exact RFC 8785 spelling stays `noncanonical_wire` with its pointer
/// (FR-056), and only a document intake cannot read is not a domain package
/// document.
fn package_document_refusal(digest: DigestRecord, refusal: ModelRefusal) -> ReplayRequestRefusal {
    let entry = format!("{digest:?}");
    match refusal.cause {
        ModelRefusalCause::IntakeLimitExceeded { limit, bound } => {
            ReplayRequestRefusal::IntakeLimitExceeded {
                entry,
                limit,
                bound,
            }
        }
        ModelRefusalCause::AllocationFailed { requested } => {
            ReplayRequestRefusal::AllocationFailed { entry, requested }
        }
        ModelRefusalCause::NoncanonicalNumber {
            inexact,
            document_pointer,
        } => ReplayRequestRefusal::NoncanonicalNumber {
            entry,
            inexact,
            document_pointer,
        },
        _ => ReplayRequestRefusal::NotAPackageDocument(entry),
    }
}

impl ReplayRequestRefusal {
    /// The catalog code of this refusal.
    pub fn code(&self) -> Code {
        match self {
            Self::UnknownSemanticProfile { .. } => Code::UnknownProfile,
            Self::DigestDomainMismatch(_)
            | Self::ByteDigestMismatch(_)
            | Self::ContentMismatch { .. } => Code::StaleDependency,
            Self::MalformedDigest(_) | Self::IneligibleByteProvisionDomain(_) => {
                Code::InvalidDigest
            }
            Self::NotAPackageDocument(_) => Code::InvalidModelBinding,
            Self::IntakeLimitExceeded { .. } | Self::AllocationFailed { .. } => {
                Code::ResourceExhausted
            }
            Self::NoncanonicalNumber { .. } => Code::NoncanonicalWire,
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
    wire.profile_selections
        .iter()
        .map(|p| p.profile().len() + p.value().len())
        .sum::<usize>()
        + wire.package_id.1.len()
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
            // A 32-byte node id plus a value of at most 8 bytes per entry
            // (QC-1's digest-addressed shape), a fixed bound independent of
            // any `Debug`-rendered text.
            ReplaySource::Input(assignments) => assignments.len() * (32 + 8),
        }
}

impl ReplayRequest {
    /// Decode a request from `wire`. The size-bound and semantic-profile
    /// checks happen before the byte provision or package reference is read
    /// at all: no package lookup or byte-provision access is observed
    /// before either refusal.
    #[qsl_attrs::string_edge]
    pub fn decode(wire: ReplayRequestWire) -> Result<Self, ReplayRequestRefusal> {
        BoundExceeded::check(measured_encoded_bytes(&wire))?;
        for selection in &wire.profile_selections {
            if !KNOWN_SEMANTIC_PROFILES.contains(&selection.profile()) {
                return Err(ReplayRequestRefusal::UnknownSemanticProfile {
                    selection: selection.clone(),
                    required_role: SEMANTIC_PROFILE_ROLE,
                });
            }
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
                let identity = LibraryName::new(entry.identity)
                    .map_err(|_| ReplayRequestRefusal::EmptyDependencySelection { index })?;
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
                Ok::<_, ReplayRequestRefusal>(DependencyEntry {
                    identity,
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
            let recomputed = digest_of(digest, &bytes)?;
            if recomputed != *digest.as_bytes() {
                return Err(if digest.domain() == DigestDomain::Sha256Jcs {
                    ReplayRequestRefusal::ContentMismatch {
                        selected: format!("{digest:?}"),
                        recomputed: format!(
                            "{:?}",
                            DigestRecord::mint(digest.domain(), recomputed)
                        ),
                    }
                } else {
                    ReplayRequestRefusal::ByteDigestMismatch(format!("{digest:?}"))
                });
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
        // ADR-013 C-27: the backend digest is not just any FR-201
        // domain -- it must be `quire.tool-manifest.jcs/v1`, checked before
        // the hex bytes are read. `ManifestDigest::from_wire` cannot
        // construct anything else.
        let backend_digest = ManifestDigest::from_wire(backend_domain.as_deref(), &backend_hex)
            .map_err(classify_digest_error)?;

        Ok(Self {
            package_id,
            source_digests,
            dependencies,
            selected_function: wire.selected_function,
            source: wire.source,
            profile_selections: wire.profile_selections,
            obligation_identity: ObligationIdentity::from_digest(wire.obligation_identity),
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
            profile_selections: self.profile_selections.clone(),
            package_id: (
                Some(self.package_id.domain().as_str().to_owned()),
                self.package_id.hex(),
            ),
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
                    package_id: (
                        Some(entry.package_id.domain().as_str().to_owned()),
                        entry.package_id.hex(),
                    ),
                    sources: entry.sources.iter().map(RawSourceRef::to_wire).collect(),
                })
                .collect(),
            selected_function: self.selected_function.clone(),
            source: self.source.clone(),
            obligation_identity: *self.obligation_identity.as_bytes(),
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
            profile_selections: vec![ProfileSelection::new(
                "quire.profile.v1".to_owned(),
                "finite-state".to_owned(),
            )],
            package_id: (
                Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
                DigestRecord::mint(DigestDomain::PackageSemanticV2, [4; 32]).hex(),
            ),
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
                value: crate::witness::WitnessValue::Integer(42),
            }]),
            obligation_identity: [1; 32],
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

    /// A `dependencies` entry over one `fill`-byte source under `identity`.
    fn dependency(identity: &str, fill: u8) -> (DependencyEntryWire, Vec<u8>) {
        let bytes = source_bytes(fill, 32);
        let digest = DigestRecord::mint(
            DigestDomain::SourceBytesV1,
            ByteDigest::of(&bytes).as_bytes(),
        );
        (
            DependencyEntryWire {
                identity: identity.to_owned(),
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
    /// construction; an empty identity, and a `package_id` in
    /// the `quire.source.bytes/v1` domain, refuse at decode.
    #[trace("TC-186", "FR-071-AC-9")]
    #[test]
    fn tc_186_dependencies_round_trip_and_refuse_at_decode() {
        let entries = vec![dependency("test/b", 0x0B), dependency("test/a", 0x0A)];
        let request = ReplayRequest::decode(with_dependencies(entries.clone())).unwrap();
        let identities: Vec<&str> = request
            .dependencies()
            .iter()
            .map(|entry| entry.identity().as_str())
            .collect();
        assert_eq!(identities, ["test/b", "test/a"]);
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
        let mut incomplete = with_dependencies(vec![dependency("test/a", 0x0A)]);
        incomplete.byte_provision.pop();
        assert!(matches!(
            ReplayRequest::decode(incomplete),
            Err(ReplayRequestRefusal::IncompleteByteProvision(_))
        ));

        // An empty identity.
        let refused =
            ReplayRequest::decode(with_dependencies(vec![dependency("", 0x0A)])).unwrap_err();
        assert_eq!(
            refused,
            ReplayRequestRefusal::EmptyDependencySelection { index: 0 }
        );
        assert_eq!(refused.code(), Code::InvalidIdentifier);

        // A `package_id` in the source-bytes domain.
        let (mut entry, bytes) = dependency("test/a", 0x0A);
        entry.package_id.0 = Some(DigestDomain::SourceBytesV1.as_str().to_owned());
        assert!(matches!(
            ReplayRequest::decode(with_dependencies(vec![(entry, bytes)])),
            Err(ReplayRequestRefusal::DigestDomainMismatch(_))
        ));
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
        // digest refuse as a content mismatch.
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
            Err(ReplayRequestRefusal::ContentMismatch { .. })
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
        oversized.state_environment =
            StateEnvironment::new(vec![("x".repeat(MAX_ENCODED_BYTES + 1), String::new())]);
        assert!(matches!(
            ReplayRequest::decode(oversized),
            Err(ReplayRequestRefusal::BoundExceeded(_))
        ));
        // The bound measures the `dependencies` entries too.
        let (mut entry, bytes) = dependency("test/a", 0x0A);
        entry.identity = "x".repeat(MAX_ENCODED_BYTES + 1);
        assert!(matches!(
            ReplayRequest::decode(with_dependencies(vec![(entry, bytes)])),
            Err(ReplayRequestRefusal::BoundExceeded(_))
        ));
    }

    /// Positive control: a real request whose `backend` digest is in
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

    /// ADR-013 C-27: a `backend` digest in a recognized FR-201
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

    /// A `sha256-jcs` byte-provision entry that intake's read refuses keeps
    /// the refusal's cause (FR-071-AC-11, TC-186 step 9): bytes intake
    /// cannot read are not a domain package document, a document over an
    /// intake limit refuses with intake's limit outcome, an allocation
    /// failure refuses `resource_exhausted`/`allocation-failed` carrying the
    /// bytes requested (FR-259 B6), and a number with no exact RFC 8785
    /// spelling refuses `noncanonical_wire` with its cause and
    /// `document_pointer` (FR-056).
    #[trace("TC-186", "FR-071-AC-11")]
    #[test]
    fn a_package_document_refusal_keeps_its_cause() {
        let digest = DigestRecord::mint(DigestDomain::Sha256Jcs, [0xAB; 32]);
        let entry = format!("{digest:?}");
        let decode = |bytes: Vec<u8>| {
            let mut request = wire(1);
            request.byte_provision.push((
                Some(DigestDomain::Sha256Jcs.as_str().to_owned()),
                digest.hex(),
                bytes,
            ));
            ReplayRequest::decode(request).unwrap_err()
        };

        let malformed = decode(br#"{"a":1,"a":2}"#.to_vec());
        assert_eq!(
            malformed,
            ReplayRequestRefusal::NotAPackageDocument(entry.clone())
        );
        assert_eq!(malformed.code(), Code::InvalidModelBinding);

        // Nested far past the semantic-IR reader's depth limit, and far
        // below the request's encoded-size bound.
        let deep = format!("{}{}", "[".repeat(1000), "]".repeat(1000)).into_bytes();
        let ModelRefusalCause::IntakeLimitExceeded { limit, bound } =
            PackageDocument::parse(&deep).unwrap_err().cause
        else {
            panic!("a 1000-deep document is over intake's depth limit");
        };
        assert_eq!(limit, IntakeLimit::NestingDepth);
        let over_limit = decode(deep);
        assert_eq!(
            over_limit,
            ReplayRequestRefusal::IntakeLimitExceeded {
                entry: entry.clone(),
                limit,
                bound,
            }
        );
        assert_eq!(over_limit.code(), Code::ResourceExhausted);

        let allocation = package_document_refusal(
            digest,
            ModelRefusal {
                code: Code::ResourceExhausted,
                cause: ModelRefusalCause::AllocationFailed { requested: 4096 },
                detail: String::new(),
            },
        );
        assert_eq!(
            allocation,
            ReplayRequestRefusal::AllocationFailed {
                entry: entry.clone(),
                requested: 4096,
            }
        );
        assert_eq!(allocation.code(), Code::ResourceExhausted);
        assert!(allocation
            .to_string()
            .starts_with("resource_exhausted/allocation-failed:"));

        for (bytes, inexact, pointer) in [
            (
                br#"{"package":{"count":18446744073709551616}}"#.to_vec(),
                Inexact::Integer,
                "/package/count",
            ),
            (
                br#"{"a/b":[0.1000000000000000000001]}"#.to_vec(),
                Inexact::Number,
                "/a~1b/0",
            ),
        ] {
            let noncanonical = decode(bytes);
            assert_eq!(
                noncanonical,
                ReplayRequestRefusal::NoncanonicalNumber {
                    entry: entry.clone(),
                    inexact,
                    document_pointer: pointer.parse().unwrap(),
                }
            );
            assert_eq!(noncanonical.code(), Code::NoncanonicalWire);
            assert!(noncanonical
                .to_string()
                .starts_with(&format!("noncanonical_wire/{}:", inexact.as_str())));
        }
    }

    /// A semantic-profile identifier outside the closed set refuses at
    /// decode with the catalog's profile-selection refusal, before the byte
    /// provision is read: a malformed byte-provision entry alongside it
    /// would otherwise refuse `ByteDigestMismatch` (FR-071-AC-10, TC-186
    /// step 8).
    #[trace("TC-186", "FR-071-AC-10")]
    #[test]
    fn refuses_an_unknown_semantic_profile_before_the_byte_provision() {
        let mut bad_profile = wire(1);
        bad_profile.profile_selections = vec![ProfileSelection::new(
            "quire.profile.unknown/v1".to_owned(),
            "x".to_owned(),
        )];
        bad_profile.byte_provision[0].2 = source_bytes(0xFF, 4);
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
        assert_eq!(refused.code(), Code::UnknownProfile);
        assert_eq!(
            refused.to_string(),
            "unknown_profile/unsupported-selection: semantic profile \"quire.profile.unknown/v1\" (value \"x\") is outside the closed set for role replay.semantic_profile_selections"
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
            value: crate::witness::WitnessValue::Integer(distinctive_value),
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
                assert_eq!(
                    assignments[0].value,
                    crate::witness::WitnessValue::Integer(distinctive_value)
                );
            }
            ReplaySource::Witness(_) => panic!("expected the Input arm"),
        }
    }
}
