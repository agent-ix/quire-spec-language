// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §2.1 I2, §4 verified binding: the layer-4 `package` byte reader
//! for `quire.checked-package/v2` wire artifacts (QSpec FR-322).
//!
//! Envelope parsing, contract-version dispatch and the FR-322 `package_id`
//! recompute are delegated to `quire_contract_ir`'s I04 `checked_package`
//! reader ([`quire_contract_ir::read_checked_package`]): this module defines
//! no wire member set, digest domain or contract-version spelling of its
//! own, and never mints a `NodeKey` or `PackageId` from wire text (R-10,
//! ADR-013 O-02). ADR-011 §4 condition 2 (`package_id` recompute-and-match)
//! is primarily IR's own job, enforced before it ever admits a wire; this
//! reader additionally mints its own typed `PackageId` (never trusting the
//! wire's raw hex) from the same already-admitted preimage and asserts that
//! mint agrees with IR's own already-verified `package_id.digest`, refusing
//! if it does not (QSL-6 M-4 L2: defense against this reader's own
//! canonicalization ever diverging from IR's -- e.g. if IR's `digest_json`
//! were to switch to a real JCS encoder -- not a second, independent
//! admission decision). What remains this layer's own job:
//!
//! - minting the package's own typed `PackageId` from the admitted identity
//!   preimage via [`qsl_semantics::library::PackageId::of_preimage`] -- the only
//!   constructor;
//! - deriving the FR-307 export node keys from that preimage
//!   (`qsl_semantics::library::declared_exports`/`verify_package`, layer-3);
//! - the ADR-011 §4 verified binding itself (`qsl_semantics::library::verify_binding`,
//!   layer-3, QSL-6 slice A1): once IR admits the wire, this reader mints
//!   the condition-1 witness (`qsl_semantics::library::SupportedV2Wire`, which only
//!   this module constructs) and hands it, its digest-checked candidate and
//!   the caller's `pinned` request to `library`, which applies conditions 2
//!   and 3 and constructs the `VerifiedPackage` (FR-087-AC-1, AC-3) this
//!   reader returns.
//!
//! IR's v2 reader validates the whole I04 contract unconditionally (lock
//! staleness against caller-supplied evidence, the complete semantic graph,
//! source-map and diagnostics) -- there is no narrower "envelope and
//! `package_id` only" entry point. A caller therefore supplies
//! [`quire_contract_ir::CheckedPackageEvidence`] alongside the bytes: this
//! reader does not itself know which locked source, definition or domain
//! package bytes are current.
//!
//! # Ceilings
//!
//! [`V2ReadLimits`] carries every ceiling of this read: `artifact_bytes`
//! and `depth`, which this reader checks itself and also hands to IR as
//! `bytes`/`depth`, and IR's other five
//! [`quire_contract_ir::CheckedPackageReadLimits`] ceilings (`nodes`,
//! `edges`, `occurrences`, `diagnostics`, `work`), passed through unchanged.
//! Every one is the caller's, used as given; the defaults are IR's own
//! `bounded()` values plus this reader's 16 MiB byte default. It is not the
//! native-v1 `PackageLimits` (the root crate's `package`, FR-019, SEAM-1
//! until M-6): that type's `string_bytes` and `entries` exist only for the
//! native encode/intake path and have no IR counterpart. Every IR ceiling a
//! caller can hit is reported, verbatim, as [`V2ReadIncomplete::Limit`]'s
//! [`quire_contract_ir::CheckedPackageLimit`] -- except `depth` itself,
//! which two IR-side facts (both raised as IR-238, not papered over here)
//! keep from being a clean refused/
//! incomplete split at every depth:
//!
//! - **Above serde_json's parse-time recursion cap, refused, not
//!   `Incomplete`.** IR's `strict_json_value` parses with
//!   `serde_json::Deserializer` and never calls `disable_recursion_limit`,
//!   so serde_json's own fixed 128-container recursion cap fires *before*
//!   IR's own `json_depth` resource meter ever runs. A wire nested past
//!   128 containers is reported `Refused(Envelope(MalformedWire))`, not
//!   `Incomplete(Limit(Depth))`, regardless of [`V2ReadLimits::depth`]
//!   (IR-238 item 1; pinned by this module's own
//!   `depth_far_past_the_default_limit_is_refused_as_malformed_wire`
//!   test).
//! - **Fail-closed at the boundary, for a scalar-terminated path.** IR's
//!   `json_depth` counts a scalar leaf as depth 1 even at zero entered
//!   containers, while [`V2ReadLimits::depth`] counts only entered
//!   containers, so e.g. `{"a":1}` is depth 2 in IR's units but depth 1
//!   here. This reader passes `V2ReadLimits::depth` to IR *unchanged* --
//!   no `+ 1` conversion (QSL-6 M2, decided fail-closed): no wire deeper
//!   than `V2ReadLimits::depth` containers is ever admitted, for either
//!   kind of deepest path. The cost is one-sided: a wire whose deepest
//!   path ends in a scalar exactly at the configured boundary is refused
//!   `Incomplete` one level early (IR counts its terminal scalar as an
//!   extra unit), while a wire whose deepest path ends in an empty
//!   container is admitted exactly at the boundary (IR's count and this
//!   reader's agree there). A caller that needs exact-boundary admission
//!   for scalar-terminated documents must widen its own configured limit
//!   by one; this reader does not guess which kind of path is deepest.
//!
//! # Loci (FR-096)
//!
//! The read returns ADR-013 T-4's `Result<Staged<V2Read>,
//! StageFailure<V2ReadRefusal>>`. Every refusal and limit IR reports at a
//! value carries `Locus::Artifact`: the `raw-artifact-digest` record of the
//! supplied bytes (FR-201, O-18) and the RFC 6901 pointer IR reports. An IR
//! refusal at no value (malformed JSON, non-canonical bytes) and IR's byte
//! budget carry no locus, since the whole-document pointer would stand in
//! for a position it does not name. So does this reader's own artifact byte
//! ceiling, which refuses without hashing the oversized bytes. IR's refusal
//! of the contract version is `unknown_wire`/`unsupported-wire`, naming the
//! version IR read (ADR-013 O-22).
use std::collections::BTreeMap;

use quire_contract_ir::{
    read_checked_package, CheckedArtifactRef, CheckedOccurrenceRole, CheckedPackageDispatchResult,
    CheckedPackageEvidence, CheckedPackageIncomplete, CheckedPackageLimit,
    CheckedPackageReadLimits, CheckedPackageRefusal, CheckedPackageRefusalCode,
    CheckedSourceMapEntry, CheckedSourceRegion, CHECKED_PACKAGE_V2,
};

use qsl_foundation::diagnostic::{
    CatalogCode, CatalogCoded, Code, InternalFault, JsonPointer, LimitExceeded, LimitKind, Locus,
    RefusalRecord, StageFailure, Staged,
};
use qsl_foundation::digest::{DigestRecord, InvalidDigestRecord, WireNodeId};
use qsl_foundation::source::provenance::{
    InvalidProvenance, OccurrenceKey, PackageSourceMap, RawSourceRef, Revision, SourceRegion,
};
use qsl_semantics::library::{
    declared_exports, verify_binding, LibraryName, LibraryPackage, LibraryRefusal, PackageId,
    PinnedRequest, SupportedV2Wire, VerifiedPackage,
};
use qsl_semantics::value::IDENTITY_LIMITS;
use quire_exact::{Origin, Role};

#[cfg(test)]
mod tests;

/// A test's view of one [`read_checked_package_v2`] outcome, T-4's
/// `Result` unpacked so a test matches every arm by pattern.
#[cfg(test)]
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Read {
    /// The read admitted the bytes.
    Verified {
        /// See [`V2Read::package`].
        package: VerifiedPackage,
        /// See [`V2Read::source_map`].
        source_map: PackageSourceMap,
        /// See [`V2Read::effective_limits`].
        effective_limits: V2ReadLimits,
    },
    /// `StageFailure::Refused`.
    Refused(V2ReadRefusal),
    /// `StageFailure::Limit`.
    Limit(LimitExceeded),
}

/// [`read_checked_package_v2`], viewed as a [`Read`].
#[cfg(test)]
pub(crate) fn read_v2(
    bytes: &[u8],
    identity: LibraryName,
    version: String,
    limits: V2ReadLimits,
    evidence: &CheckedPackageEvidence,
    pinned: &PinnedRequest,
) -> Read {
    match read_checked_package_v2(bytes, identity, version, limits, evidence, pinned) {
        Ok(staged) => {
            let V2Read {
                package,
                source_map,
                effective_limits,
            } = staged.into_value();
            Read::Verified {
                package,
                source_map,
                effective_limits,
            }
        }
        Err(StageFailure::Refused(refusal)) => Read::Refused(refusal),
        Err(StageFailure::Limit(limit)) => Read::Limit(limit),
    }
}

/// serde_json's fixed container-recursion cap, which IR's
/// `strict_json_value` inherits because it never calls
/// `disable_recursion_limit`: a wire nested deeper is refused as malformed
/// before any depth ceiling is consulted, so no `depth` above this is
/// actually enforced. An IR-side defect tracked by IR-279, not a bound of
/// this reader; remove it once IR parses without the cap.
const SERDE_JSON_RECURSION_LIMIT: usize = 128;

/// Every ceiling of one [`read_checked_package_v2`] call (see the module
/// doc's "Ceilings" section). Each field is used exactly as the caller
/// supplies it, above or below [`Self::default`]: an implementation ceiling
/// is not a domain bound (NFR-001). The one ceiling a caller cannot raise
/// is `depth` past [`SERDE_JSON_RECURSION_LIMIT`];
/// [`V2ReadOutcome::Verified`]'s `effective_limits` records that.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct V2ReadLimits {
    /// Offered bytes, checked here and handed to IR as its `bytes`.
    /// Defaults to 16 MiB.
    pub(crate) artifact_bytes: usize,
    /// Entered JSON containers, handed to IR as its `depth`. Defaults to
    /// 128; enforced only up to [`SERDE_JSON_RECURSION_LIMIT`].
    pub(crate) depth: usize,
    /// IR's semantic-graph node ceiling.
    pub(crate) nodes: u64,
    /// IR's graph dependency-edge ceiling.
    pub(crate) edges: u64,
    /// IR's combined semantic-occurrence and source-map-entry ceiling.
    pub(crate) occurrences: u64,
    /// IR's diagnostic-entry ceiling.
    pub(crate) diagnostics: u64,
    /// IR's semantic-term validation-visit ceiling.
    pub(crate) work: u64,
}

impl Default for V2ReadLimits {
    fn default() -> Self {
        let ir = CheckedPackageReadLimits::bounded();
        Self {
            artifact_bytes: 16_777_216,
            depth: 128,
            nodes: ir.nodes,
            edges: ir.edges,
            occurrences: ir.occurrences,
            diagnostics: ir.diagnostics,
            work: ir.work,
        }
    }
}

impl V2ReadLimits {
    /// The ceilings IR's reader receives: every field passed through
    /// unchanged. `depth` is fail-closed (QSL-6 M2, see the module doc's
    /// "Ceilings" section): never widened by one for IR's
    /// scalar-counts-as-depth-1 convention, so no wire deeper than `depth`
    /// containers is ever admitted.
    fn for_ir(self) -> CheckedPackageReadLimits {
        CheckedPackageReadLimits {
            bytes: u64::try_from(self.artifact_bytes).unwrap_or(u64::MAX),
            depth: u64::try_from(self.depth).unwrap_or(u64::MAX),
            nodes: self.nodes,
            edges: self.edges,
            occurrences: self.occurrences,
            diagnostics: self.diagnostics,
            work: self.work,
        }
    }

    /// The ceilings actually enforced: every field as given, except `depth`,
    /// which cannot exceed [`SERDE_JSON_RECURSION_LIMIT`].
    fn enforced(self) -> Self {
        Self {
            depth: self.depth.min(SERDE_JSON_RECURSION_LIMIT),
            ..self
        }
    }
}

/// ADR-013 O-22: IR refused the artifact's contract version.
/// `unknown_wire`/`unsupported-wire`, naming the version IR read and the one
/// this reader admits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct UnsupportedWire {
    /// The `contract_version` IR read.
    pub(crate) actual: Box<str>,
}

impl CatalogCoded for UnsupportedWire {
    fn catalog_code(&self) -> CatalogCode {
        CatalogCode::new("unknown_wire", "unsupported-wire")
    }
}

impl UnsupportedWire {
    /// The catalog row's payload: the actual selected wire and the expected
    /// one (FR-096).
    pub(crate) fn catalog_fields(&self) -> BTreeMap<&'static str, String> {
        BTreeMap::from([
            ("actual", self.actual.to_string()),
            ("expected", CHECKED_PACKAGE_V2.to_owned()),
        ])
    }
}

/// Why the reader could not admit the offered bytes (ADR-011 §4). Distinct
/// from a [`LimitExceeded`]: a refusal names a defect in the input, never a
/// resource ceiling (FR-322-AC-9).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum V2ReadRefusal {
    /// IR refused the contract version (ADR-013 O-22), located at the
    /// artifact's `/contract_version`.
    #[error("unsupported contract version {:?}", wire.actual)]
    UnsupportedVersion {
        /// The version IR read.
        wire: UnsupportedWire,
        /// `Locus::Artifact` at IR's pointer, `/contract_version`.
        locus: Box<Locus>,
    },
    /// IR's I04 `checked_package` reader refused the wire, carrying its own
    /// closed refusal code, the pointer of the value it refused, and (where
    /// IR determined one) the paired cause tag and offending node key.
    /// Covers envelope-shape, digest-domain, canonical-form, semantic-graph
    /// and lock refusals -- every I04 refusal this slice does not classify
    /// more specifically than IR itself already does. `malformed_wire`,
    /// `duplicate_member`, `unknown_member`, `noncanonical_wire` and
    /// `digest_domain_mismatch` all name defects in the wire's own envelope
    /// shape and share `Code::InvalidPackage`: this crate does not mirror
    /// IR's own closed refusal-code vocabulary with a duplicate set of
    /// variants (team decision, no-copy rule).
    #[error("checked-package/v2 wire refused ({refusal:?})")]
    Envelope {
        /// IR's refusal. Boxed: IR's refusal is the largest payload, and
        /// every read's `Result` carries this type.
        refusal: Box<CheckedPackageRefusal>,
        /// `Locus::Artifact` at IR's pointer; `None` when IR refused the
        /// bytes at no value (malformed JSON, non-canonical bytes).
        locus: Option<Box<Locus>>,
    },
    /// `library`'s verified-binding entry point refused the candidate
    /// derived from an admitted wire (its identity preimage's declared
    /// names, never its already-IR-checked `package_id`).
    /// Boxed: `LibraryRefusal` is the largest refusal, and every read's
    /// `Result` carries this type.
    #[error(transparent)]
    Structural(Box<LibraryRefusal>),
    /// `quire-canonical` refused to encode an admitted wire's identity
    /// preimage. Practically unreachable: IR has already decoded the
    /// preimage into typed Rust structs it itself just re-serialized without
    /// error (`admit_value`'s own lossless-decode check), and those bytes
    /// were canonical. Kept as its own honest QSL-side variant rather than a
    /// fabricated `Envelope(CheckedPackageRefusal{MalformedWire, ..})`: IR
    /// never actually reported this, and blaming IR's own wire vocabulary
    /// for this crate's own serialization defect would misattribute the
    /// fault to the wrong layer (QSL-6 L3).
    #[error("identity preimage re-serialization failed: {0}")]
    Preimage(String),
    /// An admitted wire's `source_map` did not convert into the package
    /// source map (ADR-013 O-12). IR's reader has already validated the
    /// map, so this names a disagreement between IR's admission and this
    /// crate's own provenance types, not a second admission decision.
    #[error("source map conversion failed: {0}")]
    SourceMap(#[from] SourceMapDefect),
    /// A broken invariant between IR's reader and this one (ADR-013 T-4):
    /// IR reported a pointer that is not RFC 6901 text.
    #[error("internal fault: {} broke {}", .0.stage(), .0.invariant())]
    Fault(InternalFault),
}

/// Why an admitted wire's `source_map` did not convert into a
/// [`PackageSourceMap`].
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum SourceMapDefect {
    /// An entry's `node_id` digest is not 64 lowercase hexadecimal digits.
    #[error("source-map node id {0:?} is not a wire node id")]
    NodeId(Box<str>),
    /// A region's source digest did not read as a digest record.
    #[error(transparent)]
    Digest(#[from] InvalidDigestRecord),
    /// A region, its source reference or the map itself refused.
    #[error(transparent)]
    Provenance(#[from] InvalidProvenance),
}

/// ADR-013 O-07: an occurrence role's FR-322 wire spelling, which the
/// kernel `Role` carries.
fn role_spelling(role: &CheckedOccurrenceRole) -> &'static str {
    match role {
        CheckedOccurrenceRole::Declaration => "declaration",
        CheckedOccurrenceRole::Type => "type",
        CheckedOccurrenceRole::Expression => "expression",
        CheckedOccurrenceRole::Anchor => "anchor",
        CheckedOccurrenceRole::Claim => "claim",
        CheckedOccurrenceRole::Generated => "generated",
    }
}

/// Every FR-322 occurrence role.
const OCCURRENCE_ROLES: [CheckedOccurrenceRole; 6] = [
    CheckedOccurrenceRole::Declaration,
    CheckedOccurrenceRole::Type,
    CheckedOccurrenceRole::Expression,
    CheckedOccurrenceRole::Anchor,
    CheckedOccurrenceRole::Claim,
    CheckedOccurrenceRole::Generated,
];

/// The occurrence role `spelling` names, the inverse of [`role_spelling`].
pub(crate) fn occurrence_role(spelling: &str) -> Option<CheckedOccurrenceRole> {
    OCCURRENCE_ROLES
        .into_iter()
        .find(|role| role_spelling(role) == spelling)
}

fn raw_source_ref(source: &CheckedArtifactRef) -> Result<RawSourceRef, SourceMapDefect> {
    let digest = DigestRecord::from_wire(Some(&source.digest_domain), &source.digest)?;
    let revision = Revision::new(&*source.revision.namespace, &*source.revision.value)?;
    Ok(RawSourceRef::new(
        &*source.authority,
        &*source.identity,
        revision,
        digest,
    )?)
}

fn source_region(region: &CheckedSourceRegion) -> Result<SourceRegion, SourceMapDefect> {
    Ok(SourceRegion::new(
        raw_source_ref(&region.source)?,
        region.start,
        region.end,
    )?)
}

fn occurrence(
    entry: &CheckedSourceMapEntry,
) -> Result<(OccurrenceKey, Vec<SourceRegion>), SourceMapDefect> {
    let node = WireNodeId::from_hex(&entry.node_id.digest)
        .ok_or_else(|| SourceMapDefect::NodeId(entry.node_id.digest.clone()))?;
    let origin = Origin::new(Role::new(role_spelling(&entry.role)), entry.ordinal);
    let regions = entry
        .regions
        .iter()
        .map(source_region)
        .collect::<Result<_, _>>()?;
    Ok((OccurrenceKey::new(node, origin), regions))
}

/// ADR-013 O-12, C-14: the package source map of an admitted wire's
/// `source_map`, keyed by the O-07 occurrence key.
fn package_source_map(
    entries: &[CheckedSourceMapEntry],
) -> Result<PackageSourceMap, SourceMapDefect> {
    let entries = entries
        .iter()
        .map(occurrence)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PackageSourceMap::from_entries(entries)?)
}

impl V2ReadRefusal {
    /// The FR-010 stable code.
    #[allow(
        dead_code,
        reason = "no production caller yet: ADR-011 §4's round trip (QSL-6 slice S3) wires this reader in; until then only this module's own tests call it"
    )]
    pub(crate) fn code(&self) -> Code {
        match self {
            Self::UnsupportedVersion { .. } => Code::UnknownWire,
            Self::Envelope { refusal, .. } => map_refusal_code(refusal.code),
            Self::Structural(refusal) => refusal.code(),
            Self::Preimage(_) => Code::InvalidPackage,
            Self::SourceMap(_) => Code::InvalidSourceMap,
            Self::Fault(_) => Code::RuntimeInvariant,
        }
    }

    /// Where in the artifact the refusal was raised (FR-096). `None` for an
    /// IR refusal at no value, and for the refusals this reader raises
    /// against the caller's pins or its own re-encoding, which name no
    /// position in the bytes.
    #[allow(
        dead_code,
        reason = "no production caller yet: ADR-011 §4's round trip (QSL-6 slice S3) wires this reader in; until then only this module's own tests call it"
    )]
    pub(crate) fn locus(&self) -> Option<&Locus> {
        match self {
            Self::UnsupportedVersion { locus, .. } => Some(locus),
            Self::Envelope { locus, .. } => locus.as_deref(),
            Self::Structural(_) | Self::Preimage(_) | Self::SourceMap(_) | Self::Fault(_) => None,
        }
    }

    /// The O-17 record of an unsupported contract version: code
    /// `unknown_wire`/`unsupported-wire`, the `actual` and `expected`
    /// versions, and its locus. `None` for every other refusal, whose
    /// catalog mapping is IR conformance work (ADR-013 O-17).
    #[allow(
        dead_code,
        reason = "no production caller yet: ADR-011 §4's round trip (QSL-6 slice S3) wires this reader in; until then only this module's own tests call it"
    )]
    pub(crate) fn unsupported_wire_record(&self) -> Option<RefusalRecord> {
        match self {
            Self::UnsupportedVersion { wire, locus } => Some(RefusalRecord::new(
                wire.catalog_code(),
                wire.catalog_fields(),
                Some((**locus).clone()),
            )),
            _ => None,
        }
    }
}

/// Maps IR's closed I04 refusal-code vocabulary onto this crate's own FR-010
/// codes. Exhaustive on purpose: a new IR variant must be triaged here
/// rather than silently falling into a catch-all bucket. A contract-version
/// refusal is `unknown_wire` (ADR-013 O-22). Every envelope-shape code
/// (malformed/noncanonical wire, duplicate or unknown member, digest-domain
/// mismatch) collapses to the crate's own
/// pre-existing `Code::InvalidPackage`, exactly as `UnsupportedNodeTag`
/// already does -- no per-IR-variant `Code` is minted to mirror IR's own
/// closed vocabulary (team decision, no-copy rule). IR's full
/// `CheckedPackageRefusal` (code, path, cause, locus) is still carried
/// whole in [`V2ReadRefusal::Envelope`] for callers that want more than
/// this coarse `Code`.
///
/// One known-wrong mapping is kept as-is rather than routed around: a
/// `package_id` mismatch is reported by IR as `StaleDependency`, not a
/// digest-mismatch code of its own. That is IR's own classification to
/// fix (raised as an IR ticket), not this reader's to reinterpret.
#[allow(
    dead_code,
    reason = "no production caller yet: only reached through V2ReadRefusal::code, itself uncalled until QSL-6 slice S3"
)]
fn map_refusal_code(code: CheckedPackageRefusalCode) -> Code {
    match code {
        CheckedPackageRefusalCode::UnknownContractVersion => Code::UnknownWire,
        CheckedPackageRefusalCode::MalformedWire
        | CheckedPackageRefusalCode::DuplicateMember
        | CheckedPackageRefusalCode::UnknownMember
        | CheckedPackageRefusalCode::NoncanonicalWire
        | CheckedPackageRefusalCode::DigestDomainMismatch
        | CheckedPackageRefusalCode::UnsupportedNodeTag
        | CheckedPackageRefusalCode::InvalidSemanticGraph
        | CheckedPackageRefusalCode::InvalidPackage => Code::InvalidPackage,
        CheckedPackageRefusalCode::StaleDependency => Code::StaleDependency,
        CheckedPackageRefusalCode::UnknownRequiredCapability => Code::UnknownRequiredFeature,
        CheckedPackageRefusalCode::InvalidSourceMap => Code::InvalidSourceMap,
        CheckedPackageRefusalCode::MissingDeclaration => Code::MissingDeclaration,
        CheckedPackageRefusalCode::AmbiguousDeclaration => Code::AmbiguousDeclaration,
        CheckedPackageRefusalCode::InvalidModelBinding => Code::InvalidModelBinding,
        CheckedPackageRefusalCode::IllTyped => Code::IllTyped,
        CheckedPackageRefusalCode::MissingImport => Code::MissingImport,
    }
}

/// The T-4 [`LimitKind`] each of IR's reader limits carries (catalog
/// revision `1-draft.7`'s `stage_limit_exceeded` row).
fn limit_kind(kind: CheckedPackageLimit) -> LimitKind {
    match kind {
        CheckedPackageLimit::Bytes => LimitKind::InputBytes,
        CheckedPackageLimit::Depth => LimitKind::NestingDepth,
        CheckedPackageLimit::Nodes => LimitKind::NodeCount,
        CheckedPackageLimit::Edges => LimitKind::EdgeCount,
        CheckedPackageLimit::Occurrences => LimitKind::OccurrenceCount,
        CheckedPackageLimit::Diagnostics => LimitKind::DiagnosticCount,
        CheckedPackageLimit::Work => LimitKind::WorkBudget,
    }
}

/// IR reported a pointer that is not RFC 6901 text.
const IR_POINTER_FAULT: InternalFault = InternalFault::new("I2", "ir-pointer-is-rfc-6901");

/// The supplied bytes' `raw-artifact-digest` record, and the loci it names.
struct Artifact(DigestRecord);

impl Artifact {
    fn of(bytes: &[u8]) -> Self {
        Self(DigestRecord::raw_artifact(bytes))
    }

    /// `Locus::Artifact` at `pointer`.
    fn at(&self, pointer: JsonPointer) -> Locus {
        Locus::Artifact {
            digest: self.0,
            pointer,
        }
    }

    /// `Locus::Artifact` at the pointer IR reported, or `None` when IR
    /// reported none. IR builds every pointer as RFC 6901 text, the grammar
    /// [`JsonPointer`] parses, so a pointer that does not parse is a broken
    /// invariant between IR and this reader, never a property of the bytes.
    fn at_ir(
        &self,
        pointer: Option<&quire_contract_ir::JsonPointer>,
    ) -> Result<Option<Locus>, InternalFault> {
        let Some(pointer) = pointer else {
            return Ok(None);
        };
        match pointer.as_str().parse() {
            Ok(pointer) => Ok(Some(self.at(pointer))),
            Err(invalid) => {
                debug_assert!(false, "IR reported a non-RFC-6901 pointer: {invalid}");
                Err(IR_POINTER_FAULT)
            }
        }
    }
}

/// An I2 read that admitted the bytes: the three ADR-011 §4
/// verified-binding conditions held (FR-087-AC-1, AC-3): a supported schema
/// version, a recomputed `package_id` equal to the declared one, and an
/// identity listed in the caller's `pinned` library lock or pinned request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct V2Read {
    /// The verified package.
    pub(crate) package: VerifiedPackage,
    /// The package source map (ADR-013 O-12), read from the wire's
    /// `source_map`.
    pub(crate) source_map: PackageSourceMap,
    /// The ceilings this read actually enforced: the caller's
    /// [`V2ReadLimits`] as given, with `depth` reported as at most
    /// [`SERDE_JSON_RECURSION_LIMIT`] (see [`V2ReadLimits::enforced`]).
    pub(crate) effective_limits: V2ReadLimits,
}

/// I2's own outcome, ADR-013 T-4's stage result (FR-322-AC-9): a verified
/// package, a refusal of the input, or a reached limit with no partial
/// package.
pub(crate) type V2ReadOutcome = Result<Staged<V2Read>, StageFailure<V2ReadRefusal>>;

/// A refused read.
fn refused(refusal: V2ReadRefusal) -> V2ReadOutcome {
    Err(StageFailure::Refused(refusal))
}

/// `preimage`'s RFC 8785 bytes, encoded by `quire-canonical` (ADR-013 §2,
/// ADR-013:113) under a byte ceiling of the reader's own `artifact_bytes`.
/// Refuses with the encoder's reason.
fn canonical_preimage(
    preimage: &quire_contract_ir::CheckedPackageIdentityPreimageV2,
    artifact_bytes: usize,
) -> Result<Vec<u8>, String> {
    let limits = quire_canonical::Limits::new(
        u64::try_from(artifact_bytes).unwrap_or(u64::MAX),
        IDENTITY_LIMITS.max_depth(),
    )
    .map_err(|error| error.to_string())?;
    quire_canonical::to_vec(preimage, limits).map_err(|error| error.to_string())
}

/// Read and verify `bytes` as a `quire.checked-package/v2` artifact claiming
/// library identity `identity`/`version` (ADR-011 §4 I2, all three verified-
/// binding conditions). A wire artifact carries no library identity or
/// version of its own -- those are FR-307 source-level facts the caller
/// already knows (the import declaration, or the compiled unit's own
/// manifest) -- so they are supplied here rather than read from the bytes.
/// `evidence` proves the wire's locked sources, definitions and domain
/// packages are current; the caller owns it (this reader does not itself
/// know which bytes are current). `pinned` is condition 3's own input: the
/// consumer's library lock (`PinnedRequest::from(&LibraryLock)`) or pinned
/// request, one selection per library identity.
#[allow(
    dead_code,
    reason = "no production caller yet: ADR-011 §4's round trip (QSL-6 slice S3) wires this reader in; until then only this module's own tests call it"
)]
pub(crate) fn read_checked_package_v2(
    bytes: &[u8],
    identity: LibraryName,
    version: String,
    limits: V2ReadLimits,
    evidence: &CheckedPackageEvidence,
    pinned: &PinnedRequest,
) -> V2ReadOutcome {
    if bytes.len() > limits.artifact_bytes {
        // No locus: the ceiling refuses without hashing the oversized bytes,
        // and a digest over them is the only name the artifact has (FR-096).
        return Err(StageFailure::Limit(LimitExceeded::new(
            LimitKind::InputBytes,
            u64::try_from(limits.artifact_bytes).unwrap_or(u64::MAX),
            u128::try_from(bytes.len()).unwrap_or(u128::MAX),
        )));
    }
    match read_checked_package(bytes, limits.for_ir(), evidence) {
        CheckedPackageDispatchResult::AdmittedV2(package) => {
            // The preimage's RFC 8785 bytes, from `quire-canonical` (ADR-013
            // §2, ADR-013:113: the one RFC 8785 implementation), encoded
            // straight from IR's typed preimage: the encoder orders members
            // itself. They are a canonical substring of the wire IR just
            // admitted, so they fit its byte ceiling.
            let preimage_bytes =
                match canonical_preimage(package.identity_preimage(), limits.artifact_bytes) {
                    Ok(bytes) => bytes,
                    Err(reason) => return refused(V2ReadRefusal::Preimage(reason)),
                };
            let package_id = PackageId::of_preimage(&preimage_bytes);
            // L2: cross-check this reader's own recompute against IR's
            // already-verified `package_id.digest` (see the module doc).
            // `verify_package` below re-recomputes from the same
            // `preimage_bytes` this `package_id` was itself minted from, so
            // it cannot by itself catch this reader's canonicalization ever
            // diverging from IR's own -- only this comparison, against a
            // value IR derived independently, can.
            let ir_digest = package.package_id().digest.as_ref();
            if package_id.hex() != ir_digest {
                return refused(V2ReadRefusal::Structural(Box::new(
                    LibraryRefusal::IdentityDivergedFromIr {
                        library: identity.clone(),
                        ir_digest: ir_digest.into(),
                        recomputed_hex: package_id.hex(),
                    },
                )));
            }
            // `PreimageDefect::AmbiguousDeclaration` cannot reach here: IR's
            // `validate_declaration_names` refuses a repeated `qualified_name`
            // (`ambiguous_declaration`) and its segments are identifiers, so
            // the `::` join compares the same names.
            let exports = match declared_exports(&preimage_bytes) {
                Ok(exports) => exports,
                Err(defect) => {
                    return refused(V2ReadRefusal::Structural(Box::new(
                        LibraryRefusal::InvalidPreimage {
                            library: identity,
                            defect,
                        },
                    )));
                }
            };
            let source_map = match package_source_map(package.source_map()) {
                Ok(source_map) => source_map,
                Err(defect) => return refused(defect.into()),
            };
            let candidate = LibraryPackage {
                library: identity,
                version,
                package_id,
                identity_preimage: preimage_bytes.into_boxed_slice(),
                imports: Vec::new(),
                exports,
            };
            // Condition 1: this call, in IR's `AdmittedV2` arm, is the
            // witness's only production mint; `tests/it/
            // verified_binding_witness.rs` fails on any other.
            let admitted = SupportedV2Wire::attest_ir_admitted_v2();
            match verify_binding(admitted, candidate, pinned) {
                Ok(verified) => Ok(Staged::new(V2Read {
                    package: verified,
                    source_map,
                    effective_limits: limits.enforced(),
                })),
                Err(refusal) => refused(V2ReadRefusal::Structural(Box::new(refusal))),
            }
        }
        CheckedPackageDispatchResult::Refused(refusal) => {
            let artifact = Artifact::of(bytes);
            let locus = match artifact.at_ir(refusal.path.as_ref()) {
                Ok(locus) => locus.map(Box::new),
                Err(fault) => return refused(V2ReadRefusal::Fault(fault)),
            };
            refused(match (refusal.code, &refusal.contract_version, locus) {
                (CheckedPackageRefusalCode::UnknownContractVersion, Some(actual), Some(locus)) => {
                    V2ReadRefusal::UnsupportedVersion {
                        wire: UnsupportedWire {
                            actual: actual.clone(),
                        },
                        locus,
                    }
                }
                (_, _, locus) => V2ReadRefusal::Envelope {
                    refusal: Box::new(refusal),
                    locus,
                },
            })
        }
        CheckedPackageDispatchResult::Incomplete(CheckedPackageIncomplete {
            limit_kind: kind,
            limit,
            consumed,
            path,
        }) => match Artifact::of(bytes).at_ir(path.as_ref()) {
            Ok(locus) => Err(StageFailure::Limit(
                LimitExceeded::new(limit_kind(kind), limit, u128::from(consumed)).at(locus),
            )),
            Err(fault) => refused(V2ReadRefusal::Fault(fault)),
        },
    }
}
