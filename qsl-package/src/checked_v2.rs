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
//! - refusing a lock that names `dependency_selections` this reader cannot
//!   yet derive imports from (QC-10, ADR-013 TK-08, not yet mapped);
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
use quire_contract_ir::{
    read_checked_package, CheckedArtifactRef, CheckedOccurrenceRole, CheckedPackageDispatchResult,
    CheckedPackageEvidence, CheckedPackageIncomplete, CheckedPackageReadLimits,
    CheckedPackageRefusal, CheckedPackageRefusalCode, CheckedSourceMapEntry, CheckedSourceRegion,
};

use qsl_foundation::diagnostic::Code;
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

/// Why the reader could not admit the offered bytes (ADR-011 §4). Distinct
/// from [`V2ReadIncomplete`]: a refusal names a defect in the input, never a
/// resource ceiling (FR-322-AC-9).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum V2ReadRefusal {
    /// IR's I04 `checked_package` reader refused the wire, carrying its own
    /// closed refusal code, the closed-schema path admission failed at, and
    /// (where IR determined one) the paired cause tag and offending node
    /// key. Covers contract-version, envelope-shape, digest-domain,
    /// canonical-form and (once node arms land) semantic-graph and lock
    /// refusals -- every I04 refusal this slice does not classify more
    /// specifically than IR itself already does. `unknown_contract_version`,
    /// `malformed_wire`, `duplicate_member`, `unknown_member`,
    /// `noncanonical_wire` and `digest_domain_mismatch` all name defects in
    /// the wire's own envelope shape and share `Code::InvalidPackage`: this
    /// crate does not mirror IR's own closed refusal-code vocabulary with a
    /// duplicate set of variants (team decision, no-copy rule).
    #[error("checked-package/v2 wire refused ({0:?})")]
    Envelope(CheckedPackageRefusal),
    /// The admitted wire's lock names one or more `dependency_selections`;
    /// this reader does not yet derive imports from a lock (QC-10, ADR-013
    /// TK-08).
    #[error("dependency_selections present; import derivation is not yet implemented")]
    UnsupportedDependencySelections,
    /// `library`'s verified-binding entry point refused the candidate
    /// derived from an admitted wire (its identity preimage's declared
    /// names, never its already-IR-checked `package_id`).
    #[error(transparent)]
    Structural(#[from] LibraryRefusal),
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
            Self::Envelope(refusal) => map_refusal_code(refusal.code),
            Self::UnsupportedDependencySelections => Code::UnsupportedDependencySelections,
            Self::Structural(refusal) => refusal.code(),
            Self::Preimage(_) => Code::InvalidPackage,
            Self::SourceMap(_) => Code::InvalidSourceMap,
        }
    }
}

/// Maps IR's closed I04 refusal-code vocabulary onto this crate's own FR-010
/// codes. Exhaustive on purpose: a new IR variant must be triaged here
/// rather than silently falling into a catch-all bucket. Every envelope-
/// shape code (contract-version, malformed/noncanonical wire, duplicate or
/// unknown member, digest-domain mismatch) collapses to the crate's own
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
        CheckedPackageRefusalCode::UnknownContractVersion
        | CheckedPackageRefusalCode::MalformedWire
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
    }
}

/// The reader ran out of a selected resource ceiling before it could decide
/// admission (FR-322-AC-6/AC-9): no partial package, distinct from a
/// refusal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum V2ReadIncomplete {
    /// The offered bytes exceed [`V2ReadLimits::artifact_bytes`], checked
    /// before the bytes are handed to IR's v2 reader.
    Bytes {
        /// The configured ceiling.
        limit: usize,
        /// The bytes actually offered.
        actual: usize,
    },
    /// IR's v2 reader stopped at a named resource ceiling before reaching a
    /// conclusion. No catalog cause yet for `Edges`/`Occurrences`/
    /// `Diagnostics` (STD-95); this reader has no production caller yet
    /// either (ADR-011 §4's round trip, QSL-6), so none of `kind`'s values
    /// are moved onto `stage_limit_exceeded` here (QSL-236).
    Limit {
        /// The exhausted resource.
        kind: quire_contract_ir::CheckedPackageLimit,
        /// The configured ceiling.
        limit: u64,
        /// The counter value at the failed charge.
        consumed: u64,
    },
}

/// I2's own outcome: exactly a verified package, refused or incomplete
/// (FR-322-AC-9). `pub(crate)`: ADR-011 §4's round trip (QSL-6 slice S3, A7)
/// has no production caller yet; until then only this module's own tests
/// construct one.
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(
    dead_code,
    reason = "no production caller yet: ADR-011 §4's round trip (QSL-6 slice S3) wires this reader in; until then only this module's own tests construct one"
)]
pub(crate) enum V2ReadOutcome {
    /// The bytes held all three ADR-011 §4 verified-binding conditions
    /// (FR-087-AC-1, AC-3): a supported schema version, a recomputed
    /// `package_id` equal to the declared one, and an identity listed in
    /// the caller's `pinned` library lock or pinned request.
    Verified {
        /// The verified package.
        package: VerifiedPackage,
        /// The package source map (ADR-013 O-12), read from the wire's
        /// `source_map`.
        source_map: PackageSourceMap,
        /// The ceilings this read actually enforced: the caller's
        /// [`V2ReadLimits`] as given, with `depth` reported as at most
        /// [`SERDE_JSON_RECURSION_LIMIT`] (see [`V2ReadLimits::enforced`]).
        effective_limits: V2ReadLimits,
    },
    /// A named defect in the input.
    Refused(V2ReadRefusal),
    /// A resource ceiling stopped the reader first.
    Incomplete(V2ReadIncomplete),
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
        return V2ReadOutcome::Incomplete(V2ReadIncomplete::Bytes {
            limit: limits.artifact_bytes,
            actual: bytes.len(),
        });
    }
    match read_checked_package(bytes, limits.for_ir(), evidence) {
        CheckedPackageDispatchResult::AdmittedV2(package) => {
            if !package.lock().dependency_selections.is_empty() {
                return V2ReadOutcome::Refused(V2ReadRefusal::UnsupportedDependencySelections);
            }
            // The preimage's RFC 8785 bytes, from `quire-canonical` (ADR-013
            // §2, ADR-013:113: the one RFC 8785 implementation), encoded
            // straight from IR's typed preimage: the encoder orders members
            // itself. They are a canonical substring of the wire IR just
            // admitted, so they fit its byte ceiling.
            let preimage_bytes =
                match canonical_preimage(package.identity_preimage(), limits.artifact_bytes) {
                    Ok(bytes) => bytes,
                    Err(reason) => return V2ReadOutcome::Refused(V2ReadRefusal::Preimage(reason)),
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
                return V2ReadOutcome::Refused(V2ReadRefusal::Structural(
                    LibraryRefusal::IdentityDivergedFromIr {
                        library: identity.clone(),
                        ir_digest: ir_digest.into(),
                        recomputed_hex: package_id.hex(),
                    },
                ));
            }
            // `PreimageDefect::AmbiguousDeclaration` cannot reach here: IR's
            // `validate_declaration_names` refuses a repeated `qualified_name`
            // (`ambiguous_declaration`) and its segments are identifiers, so
            // the `::` join compares the same names.
            let exports = match declared_exports(&preimage_bytes) {
                Ok(exports) => exports,
                Err(defect) => {
                    return V2ReadOutcome::Refused(V2ReadRefusal::Structural(
                        LibraryRefusal::InvalidPreimage {
                            library: identity,
                            defect,
                        },
                    ))
                }
            };
            let source_map = match package_source_map(package.source_map()) {
                Ok(source_map) => source_map,
                Err(defect) => return V2ReadOutcome::Refused(defect.into()),
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
                Ok(verified) => V2ReadOutcome::Verified {
                    package: verified,
                    source_map,
                    effective_limits: limits.enforced(),
                },
                Err(refusal) => V2ReadOutcome::Refused(V2ReadRefusal::Structural(refusal)),
            }
        }
        CheckedPackageDispatchResult::Refused(refusal) => {
            V2ReadOutcome::Refused(V2ReadRefusal::Envelope(refusal))
        }
        CheckedPackageDispatchResult::Incomplete(CheckedPackageIncomplete {
            limit_kind,
            limit,
            consumed,
            path: _,
        }) => V2ReadOutcome::Incomplete(V2ReadIncomplete::Limit {
            kind: limit_kind,
            limit,
            consumed,
        }),
    }
}
