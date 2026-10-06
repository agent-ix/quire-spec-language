// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §2.1 I2, §4 verified binding: the layer-4 `package` byte reader
//! for `quire.checked-package/v2` wire artifacts (QSpec FR-322).
//!
//! Envelope parsing, contract-version dispatch and the FR-322 `package_id`
//! recompute are delegated to `quire_contract_model`'s I04 `checked_package`
//! reader ([`quire_contract_model::read_checked_package`]): this module defines
//! no wire member set, digest domain or contract-version spelling of its
//! own, and never mints a `NodeKey` or `PackageId` from wire text (R-10,
//! ADR-013 O-02). ADR-011 §4 condition 2 (`package_id` recompute-and-match)
//! is primarily IR's own job, enforced before it ever admits a wire; this
//! reader additionally mints its own typed `PackageId` (never trusting the
//! wire's raw hex) from the same already-admitted preimage and asserts that
//! mint agrees with IR's own already-verified `package_id.digest`, refusing
//! if it does not (M-4 L2: defense against this reader's own
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
//!   layer-3): once IR admits the wire, this reader mints
//!   the condition-1 witness (`qsl_semantics::library::SupportedV2Wire`, which only
//!   this module constructs) and hands it, its digest-checked candidate and
//!   the caller's `pinned` request to `library`, which applies conditions 2
//!   and 3 and constructs the `VerifiedPackage` (FR-087-AC-1, AC-3) this
//!   reader returns.
//!
//! IR's v2 reader validates the whole I04 contract unconditionally (the
//! lock, the complete semantic graph, source-map and diagnostics) -- there
//! is no narrower "envelope and `package_id` only" entry point. A caller
//! therefore supplies [`quire_contract_model::CheckedPackageEvidence`]
//! alongside the bytes: the domain package documents, admitted dependency
//! packages and supported features this reader does not itself hold.
//!
//! # Ceilings
//!
//! [`V2ReadLimits`] carries every ceiling of this read: `artifact_bytes`,
//! which this reader checks itself and also hands to IR as `bytes`, and IR's
//! other five [`quire_contract_model::CheckedPackageReadLimits`] ceilings
//! (`nodes`, `edges`, `occurrences`, `diagnostics`, `work`), passed through
//! unchanged. Every one is the caller's, used as given; the defaults are IR's
//! own `bounded()` values plus this reader's 16 MiB byte default. There is no
//! depth ceiling (FR-264): IR's flat v2 wire has a JSON depth fixed by its
//! closed schema whatever the package's node count, so the read recurses to
//! that fixed depth and no further. It is not the native-v1 `PackageLimits`
//! (the root crate's `package`, FR-019, SEAM-1 until M-6): that type's
//! `string_bytes` and `entries` exist only for the native encode/intake path
//! and have no IR counterpart. Every IR ceiling a caller can hit is reported,
//! verbatim, as [`V2ReadIncomplete::Limit`]'s
//! [`quire_contract_model::CheckedPackageLimit`].
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
use std::sync::Arc;

use quire_contract_model::{
    read_checked_package, CheckedOccurrenceRole, CheckedPackageDispatchResult,
    CheckedPackageEvidence, CheckedPackageIncomplete, CheckedPackageLimit,
    CheckedPackageReadLimits, CheckedPackageRefusal, CheckedPackageRefusalCode, CheckedPackageV2,
    CheckedSourceMapEntry, CheckedSourceRef, CheckedSourceRegion, CHECKED_PACKAGE_V2,
};

use qsl_foundation::diagnostic::{
    CatalogCode, CatalogCoded, Code, InternalFault, JsonPointer, LimitExceeded, Locus,
    RefusalRecord, StageFailure, Staged,
};
use qsl_foundation::digest::{DigestRecord, InvalidDigestRecord, WireNodeId};
use qsl_foundation::source::provenance::{
    InvalidProvenance, OccurrenceKey, PackageSourceMap, RawSourceRef, SourceRegion,
};
use qsl_foundation::{Setting, SettingLimits};
use qsl_semantics::library::{
    declared_exports, verify_binding, ImportView, LibraryName, LibraryPackage, LibraryRefusal,
    PackageId, PinnedRequest, SupportedV2Wire, VerifiedPackage,
};
use qsl_semantics::model::key::hex;

use crate::checked::CheckedPackage;
use crate::emit::{emit_checked, Emission, EmitRefusal};
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
    /// `StageFailure::Cancelled`.
    Cancelled(quire_exact::CancelCause),
    /// `StageFailure::Fault`.
    Fault(qsl_foundation::diagnostic::InternalFault),
}

/// [`read_checked_package_v2`], viewed as a [`Read`].
#[cfg(test)]
pub(crate) fn read_v2(
    bytes: &[u8],
    identity: LibraryName,
    limits: V2ReadLimits,
    evidence: &CheckedPackageEvidence,
    pinned: &PinnedRequest,
) -> Read {
    match read_checked_package_v2(bytes, identity, limits, evidence, pinned) {
        Ok(staged) => {
            let V2Read {
                package,
                source_map,
                effective_limits,
                ..
            } = staged.into_value();
            Read::Verified {
                package,
                source_map,
                effective_limits,
            }
        }
        Err(StageFailure::Refused(refusal)) => Read::Refused(refusal),
        Err(StageFailure::Limit(limit)) => Read::Limit(limit),
        Err(StageFailure::Cancelled(cause)) => Read::Cancelled(cause),
        Err(StageFailure::Fault(fault)) => Read::Fault(fault),
    }
}

/// Every ceiling of one `read_checked_package_v2` call (see the module
/// doc's "Ceilings" section). Each field is used exactly as the caller
/// supplies it, above or below [`Self::default`]: an implementation ceiling
/// is not a domain bound (NFR-001). There is no depth ceiling (FR-264). Each
/// field is named by its `i2.*` setting (FR-255).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct V2ReadLimits {
    /// Offered bytes, checked here and handed to IR as its `bytes`
    /// (`i2.input_bytes`). Defaults to 16 MiB.
    pub artifact_bytes: usize,
    /// IR's semantic-graph node ceiling (`i2.nodes`).
    pub nodes: u64,
    /// IR's graph dependency-edge ceiling (`i2.edges`).
    pub edges: u64,
    /// IR's combined semantic-occurrence and source-map-entry ceiling
    /// (`i2.occurrences`).
    pub occurrences: u64,
    /// IR's diagnostic-entry ceiling (`i2.diagnostics`).
    pub diagnostics: u64,
    /// IR's semantic-term validation-visit ceiling (`i2.work_units`).
    pub work: u64,
}

impl Default for V2ReadLimits {
    fn default() -> Self {
        let ir = CheckedPackageReadLimits::bounded();
        Self {
            artifact_bytes: 16_777_216,
            nodes: ir.nodes,
            edges: ir.edges,
            occurrences: ir.occurrences,
            diagnostics: ir.diagnostics,
            work: ir.work,
        }
    }
}

impl V2ReadLimits {
    /// These limits with `i2.input_bytes` set to `bound`.
    #[must_use]
    pub const fn with_artifact_bytes(mut self, bound: usize) -> Self {
        self.artifact_bytes = bound;
        self
    }

    /// These limits with `i2.nodes` set to `bound`.
    #[must_use]
    pub const fn with_nodes(mut self, bound: u64) -> Self {
        self.nodes = bound;
        self
    }

    /// These limits with `i2.edges` set to `bound`.
    #[must_use]
    pub const fn with_edges(mut self, bound: u64) -> Self {
        self.edges = bound;
        self
    }

    /// These limits with `i2.occurrences` set to `bound`.
    #[must_use]
    pub const fn with_occurrences(mut self, bound: u64) -> Self {
        self.occurrences = bound;
        self
    }

    /// These limits with `i2.diagnostics` set to `bound`.
    #[must_use]
    pub const fn with_diagnostics(mut self, bound: u64) -> Self {
        self.diagnostics = bound;
        self
    }

    /// These limits with `i2.work_units` set to `bound`.
    #[must_use]
    pub const fn with_work(mut self, bound: u64) -> Self {
        self.work = bound;
        self
    }

    /// The ceilings IR's reader receives: every field passed through
    /// unchanged.
    fn for_ir(self) -> CheckedPackageReadLimits {
        CheckedPackageReadLimits {
            bytes: u64::try_from(self.artifact_bytes).unwrap_or(u64::MAX),
            nodes: self.nodes,
            edges: self.edges,
            occurrences: self.occurrences,
            diagnostics: self.diagnostics,
            work: self.work,
        }
    }
}

/// ADR-013 O-22: IR refused the artifact's contract version.
/// `unknown_wire`/`unsupported-wire`, naming the version IR read and the one
/// this reader admits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedWire {
    /// The `contract_version` IR read.
    pub actual: Box<str>,
}

impl CatalogCoded for UnsupportedWire {
    fn catalog_code(&self) -> CatalogCode {
        CatalogCode::new("unknown_wire", "unsupported-wire")
    }

    /// The catalog row's payload: the actual selected wire and the expected
    /// one (FR-096).
    fn catalog_fields(&self) -> Option<BTreeMap<&'static str, String>> {
        Some(BTreeMap::from([
            ("actual", self.actual.to_string()),
            ("expected", CHECKED_PACKAGE_V2.to_owned()),
        ]))
    }
}

/// Why the reader could not admit the offered bytes (ADR-011 §4). Distinct
/// from a [`LimitExceeded`]: a refusal names a defect in the input, never a
/// resource ceiling (FR-322-AC-9).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum V2ReadRefusal {
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
    /// `duplicate_member`, `unknown_member` and `digest_domain_mismatch`
    /// all name defects in the wire's own envelope shape and share
    /// `Code::InvalidPackage`; `noncanonical_wire` is the native catalog's
    /// own code (QSpec FR-271) and maps to `Code::NoncanonicalWire`. This
    /// crate does not mirror IR's own closed refusal-code vocabulary with a
    /// duplicate set of variants (team decision, no-copy rule).
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
    /// fault to the wrong layer.
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
pub enum SourceMapDefect {
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

fn raw_source_ref(source: &CheckedSourceRef) -> Result<RawSourceRef, SourceMapDefect> {
    let digest = DigestRecord::from_wire(Some(&source.digest_domain), &source.digest)?;
    Ok(RawSourceRef::new(
        &*source.authority,
        &*source.identity,
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
    pub fn code(&self) -> Code {
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
    pub fn locus(&self) -> Option<&Locus> {
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
        reason = "no production caller yet: unused pending ADR-011 §4's round trip (QSL-347); until then only this module's own tests call it"
    )]
    pub(crate) fn unsupported_wire_record(&self) -> Option<RefusalRecord> {
        match self {
            Self::UnsupportedVersion { wire, locus } => {
                wire.refusal_record(Some((**locus).clone()))
            }
            _ => None,
        }
    }
}

/// Maps IR's closed I04 refusal-code vocabulary onto this crate's own FR-010
/// codes. Exhaustive on purpose: a new IR variant must be triaged here
/// rather than silently falling into a catch-all bucket. A contract-version
/// refusal is `unknown_wire` (ADR-013 O-22). Non-canonical bytes are the
/// native catalog's own `noncanonical_wire` (QSpec FR-271, FR-056), shared
/// with intake's refusal of a model document number with no exact RFC 8785
/// spelling. Every other envelope-shape code (malformed wire, duplicate or
/// unknown member, digest-domain mismatch) collapses to the crate's own
/// pre-existing `Code::InvalidPackage`, exactly as `UnsupportedNodeTag`
/// already does -- no per-IR-variant `Code` is minted to mirror IR's own
/// closed vocabulary (team decision, no-copy rule). IR's full
/// `CheckedPackageRefusal` (code, path, cause, locus) is still carried
/// whole in [`V2ReadRefusal::Envelope`] for callers that want more than
/// this coarse `Code`.
///
/// IR's `StaleDependency` covers content that does not match the identity
/// it names, so a `package_id` that does not recompute maps to
/// [`Code::StaleDependency`].
fn map_refusal_code(code: CheckedPackageRefusalCode) -> Code {
    match code {
        CheckedPackageRefusalCode::UnknownContractVersion => Code::UnknownWire,
        CheckedPackageRefusalCode::NoncanonicalWire => Code::NoncanonicalWire,
        CheckedPackageRefusalCode::MalformedWire
        | CheckedPackageRefusalCode::DuplicateMember
        | CheckedPackageRefusalCode::UnknownMember
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
        CheckedPackageRefusalCode::UnknownProfile => Code::UnknownProfile,
    }
}

/// FR-255: the one mapping from each field to its setting.
impl SettingLimits for V2ReadLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        vec![
            (
                Setting::I2InputBytes,
                u64::try_from(self.artifact_bytes).unwrap_or(u64::MAX),
            ),
            (Setting::I2Nodes, self.nodes),
            (Setting::I2Edges, self.edges),
            (Setting::I2Occurrences, self.occurrences),
            (Setting::I2Diagnostics, self.diagnostics),
            (Setting::I2WorkUnits, self.work),
        ]
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        match setting {
            Setting::I2InputBytes => {
                self.artifact_bytes = usize::try_from(bound).unwrap_or(usize::MAX);
            }
            Setting::I2Nodes => self.nodes = bound,
            Setting::I2Edges => self.edges = bound,
            Setting::I2Occurrences => self.occurrences = bound,
            Setting::I2Diagnostics => self.diagnostics = bound,
            Setting::I2WorkUnits => self.work = bound,
            _ => return false,
        }
        true
    }
}

/// The setting that raises each of IR's reader limits (FR-255's `i2.*`
/// rows).
const fn limit_setting(kind: CheckedPackageLimit) -> Setting {
    match kind {
        CheckedPackageLimit::Bytes => Setting::I2InputBytes,
        CheckedPackageLimit::Nodes => Setting::I2Nodes,
        CheckedPackageLimit::Edges => Setting::I2Edges,
        CheckedPackageLimit::Occurrences => Setting::I2Occurrences,
        CheckedPackageLimit::Diagnostics => Setting::I2Diagnostics,
        CheckedPackageLimit::Work => Setting::I2WorkUnits,
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
        pointer: Option<&quire_contract_model::JsonPointer>,
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
    /// The ceilings this read enforced: the caller's [`V2ReadLimits`] as
    /// given.
    pub(crate) effective_limits: V2ReadLimits,
    /// IR's admitted package, supplied to a later read whose
    /// `dependency_selections` name it.
    pub(crate) admitted: Arc<CheckedPackageV2>,
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
    preimage: &quire_contract_model::CheckedPackageIdentityPreimageV2,
    artifact_bytes: usize,
) -> Result<Vec<u8>, String> {
    let limits = quire_canonical::Limits::new(u64::try_from(artifact_bytes).unwrap_or(u64::MAX));
    quire_canonical::to_vec(preimage, limits).map_err(|error| error.to_string())
}

/// Read and verify `bytes` as a `quire.checked-package/v2` artifact claiming
/// library identity `identity` (ADR-011 §4 I2, all three verified-binding
/// conditions). A wire artifact carries no library identity of its own --
/// that is an FR-307 source-level fact the caller already knows (the import
/// declaration, or the compiled unit's own manifest) -- so it is supplied
/// here rather than read from the bytes.
/// `evidence` supplies the selected domain package documents, the admitted
/// dependency packages and the supported features; the caller owns it, as
/// this reader holds none of them itself. `pinned` is condition 3's own input: the
/// consumer's library lock (`PinnedRequest::from(&LibraryLock)`) or pinned
/// request, one selection per library identity.
pub(crate) fn read_checked_package_v2(
    bytes: &[u8],
    identity: LibraryName,
    limits: V2ReadLimits,
    evidence: &CheckedPackageEvidence,
    pinned: &PinnedRequest,
) -> V2ReadOutcome {
    if bytes.len() > limits.artifact_bytes {
        // No locus: the ceiling refuses without hashing the oversized bytes,
        // and a digest over them is the only name the artifact has (FR-096).
        return Err(StageFailure::Limit(LimitExceeded::new(
            Setting::I2InputBytes,
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
                    effective_limits: limits,
                    admitted: Arc::from(package),
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
                LimitExceeded::new(limit_setting(kind), limit, u128::from(consumed)).at(locus),
            )),
            Err(fault) => refused(V2ReadRefusal::Fault(fault)),
        },
    }
}

/// Why [`read_import_view`] built no import view (ADR-015 D-1 step 6).
#[derive(Debug, thiserror::Error)]
pub enum ImportViewRefusal {
    /// The package, or a package of its dependency closure, did not emit
    /// complete v2 bytes: the emitter refused (`Some`), or would omit nodes
    /// (`None`).
    #[error("the package {package} does not emit a complete checked package")]
    Emission {
        /// The library identity of the package that did not emit.
        package: LibraryName,
        /// The emitter's refusal, or `None` when the emission omits nodes.
        refusal: Option<EmitRefusal>,
    },
    /// A closure entry names a `package_id` no package of the closure has.
    /// A broken invariant of `CheckedPackage::link_with`, which records
    /// only recomputed ids of packages it holds.
    #[error("the dependency {identity} is not held by the closure")]
    UnheldDependency {
        /// The closure entry's identity.
        identity: LibraryName,
    },
    /// The I2 read of the package, or of a package of its closure, refused
    /// or reached a ceiling.
    #[error("the I2 read of {package} refused: {}", read_message(refusal))]
    Read {
        /// The library identity of the package whose read refused.
        package: LibraryName,
        /// The read's refusal or reached ceiling.
        refusal: StageFailure<V2ReadRefusal>,
    },
}

/// A readable account of an I2 read's refusal or reached ceiling.
fn read_message(refusal: &StageFailure<V2ReadRefusal>) -> String {
    match refusal {
        StageFailure::Refused(refusal) => refusal.to_string(),
        StageFailure::Limit(limit) => limit.to_string(),
        StageFailure::Cancelled(cause) => format!("cancelled ({cause:?})"),
        StageFailure::Fault(fault) => {
            format!("internal fault ({}, {})", fault.stage(), fault.invariant())
        }
    }
}

impl ImportViewRefusal {
    /// The catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::Emission {
                refusal: Some(refusal),
                ..
            } => refusal.code(),
            Self::Emission { refusal: None, .. } => Code::UnsupportedProjection,
            Self::UnheldDependency { .. } => Code::RuntimeInvariant,
            Self::Read {
                refusal: StageFailure::Refused(refusal),
                ..
            } => refusal.code(),
            Self::Read {
                refusal: StageFailure::Limit(_),
                ..
            } => Code::StageLimitExceeded,
            Self::Read {
                refusal: StageFailure::Cancelled(_),
                ..
            } => Code::Cancelled,
            Self::Read {
                refusal: StageFailure::Fault(_),
                ..
            } => Code::RuntimeInvariant,
        }
    }
}

/// ADR-015 D-1 step 6: read `package`'s `emission` through the I2
/// reader, pinned to the one selection `identity` and the recomputed
/// `package_id`, into the [`VerifiedPackage`]'s [`ImportView`]
/// (ADR-011 §4 verified binding). The lock is checked against the
/// emission's own evidence of the artifacts it compiled against, against
/// `domain_packages` (FR-056's package input, by `sha256-jcs` digest) for
/// its `model_selections`, and against the admitted package of every entry
/// of its `dependency_selections` (QSpec FR-322-AC-36): taken from
/// `admitted`, by `package_id`, where an earlier read admitted it, and
/// otherwise read the same way from the closure `package` holds. Every
/// package this read admits, `package` among them, is added to `admitted`
/// for the next read. Every read runs under `limits`, the caller's `i2.*`
/// ceilings.
///
/// The artifact evidence is vacuous by construction: it is the emission's
/// own record of what it compiled against, so it cannot disagree with the
/// lock. The authority for a library is its source compile (ADR-015 D-1);
/// this read only turns the emitted bytes into the view E3 resolves
/// against, through the one verified-binding path (ADR-011 §4).
pub fn read_import_view(
    package: &CheckedPackage,
    emission: &Emission,
    identity: LibraryName,
    domain_packages: &BTreeMap<[u8; 32], Vec<u8>>,
    admitted: &mut AdmittedPackages,
    limits: V2ReadLimits,
) -> Result<ImportView, ImportViewRefusal> {
    let mut held = BTreeMap::new();
    hold_closure(package, &mut held);
    let mut reader = ClosureReader {
        held,
        domain_packages,
        admitted: std::mem::take(&mut admitted.0),
        limits,
    };
    let read = reader.read_emitted(package, emission, identity);
    admitted.0 = reader.admitted;
    let read = read?;
    admitted
        .0
        .insert(emission.package().package_id(), Arc::clone(&read.admitted));
    Ok(read.package.into_import_view())
}

/// The packages [`read_import_view`] calls have admitted, by `package_id`:
/// one compile's reads share it, so each package of the closure is read
/// once.
#[derive(Debug, Default)]
pub struct AdmittedPackages(BTreeMap<PackageId, Arc<CheckedPackageV2>>);

/// Supply `evidence` with the admitted package of every entry of
/// `package`'s `dependency_selections`, each read as [`read_import_view`]
/// reads it: for a caller that reads `package`'s own bytes itself.
#[cfg(test)]
pub(crate) fn supply_closure(
    package: &CheckedPackage,
    evidence: &mut CheckedPackageEvidence,
) -> Result<(), ImportViewRefusal> {
    let domain_packages = BTreeMap::new();
    let mut held = BTreeMap::new();
    hold_closure(package, &mut held);
    ClosureReader {
        held,
        domain_packages: &domain_packages,
        admitted: BTreeMap::new(),
        limits: V2ReadLimits::default(),
    }
    .supply(package, evidence)
}

/// Every package of `package`'s dependency closure, by recomputed
/// `package_id`.
fn hold_closure<'p>(
    package: &'p CheckedPackage,
    held: &mut BTreeMap<PackageId, &'p CheckedPackage>,
) {
    let mut pending = vec![package];
    while let Some(package) = pending.pop() {
        for (id, dependency) in package.dependencies() {
            if held.insert(*id, dependency).is_none() {
                pending.push(dependency);
            }
        }
    }
}

/// One [`read_import_view`] call's reads: each package of the closure is
/// admitted at most once.
struct ClosureReader<'p> {
    held: BTreeMap<PackageId, &'p CheckedPackage>,
    domain_packages: &'p BTreeMap<[u8; 32], Vec<u8>>,
    admitted: BTreeMap<PackageId, Arc<CheckedPackageV2>>,
    limits: V2ReadLimits,
}

impl ClosureReader<'_> {
    /// `package`, emitted and read under `identity`, with its closure's
    /// admitted packages supplied first.
    fn read(
        &mut self,
        package: &CheckedPackage,
        identity: LibraryName,
    ) -> Result<V2Read, ImportViewRefusal> {
        let emission = emit_checked(package).map_err(|refusal| ImportViewRefusal::Emission {
            package: identity.clone(),
            refusal: Some(refusal),
        })?;
        self.read_emitted(package, &emission, identity)
    }

    /// `package`'s `emission`, read under `identity`, with its closure's
    /// admitted packages supplied first.
    fn read_emitted(
        &mut self,
        package: &CheckedPackage,
        emission: &Emission,
        identity: LibraryName,
    ) -> Result<V2Read, ImportViewRefusal> {
        if !emission.omitted().is_empty() {
            return Err(ImportViewRefusal::Emission {
                package: identity,
                refusal: None,
            });
        }
        let mut evidence = emission.evidence.clone();
        for (digest, document) in self.domain_packages {
            evidence.insert_domain_package_document(hex(digest), document.as_slice());
        }
        self.supply(package, &mut evidence)?;
        let pinned = PinnedRequest::single(identity.clone(), emission.package.package_id());
        read_checked_package_v2(
            emission.package.bytes(),
            identity.clone(),
            self.limits,
            &evidence,
            &pinned,
        )
        .map(Staged::into_value)
        .map_err(|refusal| ImportViewRefusal::Read {
            package: identity,
            refusal,
        })
    }

    /// Supply `evidence` with the admitted package of every entry of
    /// `package`'s `dependency_selections`.
    fn supply(
        &mut self,
        package: &CheckedPackage,
        evidence: &mut CheckedPackageEvidence,
    ) -> Result<(), ImportViewRefusal> {
        self.admit_closure(package)?;
        for (dependency, resolved) in package.dependency_selections() {
            let admitted = self.admitted.get(&resolved.package_id).ok_or_else(|| {
                ImportViewRefusal::UnheldDependency {
                    identity: dependency.clone(),
                }
            })?;
            evidence.insert_dependency_package(dependency.as_str(), Arc::clone(admitted));
        }
        Ok(())
    }

    /// Admits every package of `package`'s closure, each read once and only
    /// after the packages it depends on, over an explicit stack so a
    /// dependency chain of any length reads on a constant native stack
    /// (ADR-030 D-1).
    fn admit_closure(&mut self, package: &CheckedPackage) -> Result<(), ImportViewRefusal> {
        // (library identity, package id, whether its dependencies are queued)
        let mut pending: Vec<(LibraryName, PackageId, bool)> = package
            .dependency_selections()
            .iter()
            .rev()
            .map(|(identity, resolved)| (identity.clone(), resolved.package_id, false))
            .collect();
        while let Some((identity, id, expanded)) = pending.pop() {
            if self.admitted.contains_key(&id) {
                continue;
            }
            let held = *self
                .held
                .get(&id)
                .ok_or_else(|| ImportViewRefusal::UnheldDependency {
                    identity: identity.clone(),
                })?;
            if !expanded {
                pending.push((identity, id, true));
                for (dependency, resolved) in held.dependency_selections().iter().rev() {
                    if !self.admitted.contains_key(&resolved.package_id) {
                        pending.push((dependency.clone(), resolved.package_id, false));
                    }
                }
                continue;
            }
            let read = self.read(held, identity)?;
            self.admitted.insert(id, read.admitted);
        }
        Ok(())
    }
}
