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
//!   preimage via [`crate::library::PackageId::of_preimage`] -- the only
//!   constructor;
//! - deriving the FR-307 export node keys from that preimage
//!   (`crate::library::declared_exports`/`verify_package`, layer-3);
//! - refusing a lock that names `dependency_selections` this reader cannot
//!   yet derive imports from (QC-10, ADR-013 TK-08, not yet mapped);
//! - the ADR-011 §4 condition 3 deferral (listed in the consumer's lock or
//!   pinned request) -- `crate::library::resolve_libraries`'s job, not this
//!   reader's.
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
//! [`PackageLimits::artifact_bytes`] and [`PackageLimits::depth`] are the
//! only two of [`PackageLimits`]' four fields this reader honors:
//! `string_bytes` and `entries` have no IR counterpart -- IR's own I04
//! reader does not meter decoded string bytes or aggregate entries at all,
//! only [`quire_contract_ir::CheckedPackageReadLimits`]'s seven ceilings
//! (`bytes`, `depth`, `nodes`, `edges`, `occurrences`, `diagnostics`,
//! `work`). This reader passes `artifact_bytes`/`depth` through unchanged
//! (see below) and leaves the other five at IR's own bounded defaults;
//! every one of the seven a caller can hit is reported, verbatim, as
//! [`V2ReadIncomplete::Limit`]'s [`quire_contract_ir::CheckedPackageLimit`]
//! -- except `depth` itself, which two IR-side facts (both raised as
//! IR-238, not papered over here) keep from being a clean refused/
//! incomplete split at every depth:
//!
//! - **Above serde_json's parse-time recursion cap, refused, not
//!   `Incomplete`.** IR's `strict_json_value` parses with
//!   `serde_json::Deserializer` and never calls `disable_recursion_limit`,
//!   so serde_json's own fixed 128-container recursion cap fires *before*
//!   IR's own `json_depth` resource meter ever runs. A wire nested past
//!   128 containers is reported `Refused(Envelope(MalformedWire))`, not
//!   `Incomplete(Limit(Depth))`, regardless of `PackageLimits::depth`
//!   (IR-238 item 1; pinned by this module's own
//!   `depth_far_past_the_default_limit_is_refused_as_malformed_wire`
//!   test).
//! - **Fail-closed at the boundary, for a scalar-terminated path.** IR's
//!   `json_depth` counts a scalar leaf as depth 1 even at zero entered
//!   containers, while [`PackageLimits::depth`] counts only entered
//!   containers, so e.g. `{"a":1}` is depth 2 in IR's units but depth 1
//!   here. This reader passes `PackageLimits::depth` to IR *unchanged* --
//!   no `+ 1` conversion (QSL-6 M2, decided fail-closed): no wire deeper
//!   than `PackageLimits::depth` containers is ever admitted, for either
//!   kind of deepest path. The cost is one-sided: a wire whose deepest
//!   path ends in a scalar exactly at the configured boundary is refused
//!   `Incomplete` one level early (IR counts its terminal scalar as an
//!   extra unit), while a wire whose deepest path ends in an empty
//!   container is admitted exactly at the boundary (IR's count and this
//!   reader's agree there). A caller that needs exact-boundary admission
//!   for scalar-terminated documents must widen its own configured limit
//!   by one; this reader does not guess which kind of path is deepest.
use quire_contract_ir::{
    read_checked_package, CheckedPackageDispatchResult, CheckedPackageEvidence,
    CheckedPackageIncomplete, CheckedPackageReadLimits, CheckedPackageRefusal,
    CheckedPackageRefusalCode,
};

use super::PackageLimits;
use crate::library::{
    declared_exports, verify_package, LibraryName, LibraryPackage, LibraryRefusal, PackageId,
};
use qsl_foundation::diagnostic::Code;

#[cfg(test)]
mod tests;

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
    /// This reader's own re-serialization of an admitted wire's identity
    /// preimage failed. Practically unreachable: IR has already decoded
    /// the preimage into typed Rust structs it itself just re-serialized
    /// without error one line earlier (`admit_value`'s own lossless-decode
    /// check). Kept as its own honest QSL-side variant rather than a
    /// fabricated `Envelope(CheckedPackageRefusal{MalformedWire, ..})`: IR
    /// never actually reported this, and blaming IR's own wire vocabulary
    /// for this crate's own serialization defect would misattribute the
    /// fault to the wrong layer (QSL-6 L3).
    #[error("identity preimage re-serialization failed: {0}")]
    Preimage(String),
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
        CheckedPackageRefusalCode::InvalidModelBinding => Code::InvalidModelBinding,
        CheckedPackageRefusalCode::IllTyped => Code::IllTyped,
    }
}

/// The reader ran out of a selected resource ceiling before it could decide
/// admission (FR-322-AC-6/AC-9): no partial package, distinct from a
/// refusal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum V2ReadIncomplete {
    /// The offered bytes exceed [`PackageLimits::artifact_bytes`], checked
    /// before the bytes are handed to IR's v2 reader.
    Bytes {
        /// The configured ceiling.
        limit: usize,
        /// The bytes actually offered.
        actual: usize,
    },
    /// IR's v2 reader stopped at a named resource ceiling before reaching a
    /// conclusion.
    Limit {
        /// The exhausted resource.
        kind: quire_contract_ir::CheckedPackageLimit,
        /// The configured ceiling.
        limit: u64,
        /// The counter value at the failed charge.
        consumed: u64,
    },
}

/// I2's own outcome: exactly a candidate, refused or incomplete
/// (FR-322-AC-9). `pub(crate)`: this reader hands its candidate to
/// [`crate::library::resolve_libraries`] before anything downstream sees it as
/// a package (finding #4, ADR-011 §4 check 3 has not run yet).
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(
    dead_code,
    reason = "no production caller yet: ADR-011 §4's round trip (QSL-6 slice S3) wires this reader in; until then only this module's own tests construct one"
)]
pub(crate) enum V2ReadOutcome {
    /// The bytes verified against checks 1-2: `package` is ready for
    /// [`crate::library::resolve_libraries`] to apply check 3. Not yet a
    /// `VerifiedPackage` (FR-087): that type does not exist yet.
    Candidate(LibraryPackage),
    /// A named defect in the input.
    Refused(V2ReadRefusal),
    /// A resource ceiling stopped the reader first.
    Incomplete(V2ReadIncomplete),
}

/// Read and verify `bytes` as a `quire.checked-package/v2` artifact claiming
/// library identity `identity`/`version` (ADR-011 §4 I2, checks 1-2). A wire
/// artifact carries no library identity or version of its own -- those are
/// FR-307 source-level facts the caller already knows (the import
/// declaration, or the compiled unit's own manifest) -- so they are supplied
/// here rather than read from the bytes. `evidence` proves the wire's locked
/// sources, definitions and domain packages are current; the caller owns it
/// (this reader does not itself know which bytes are current).
#[allow(
    dead_code,
    reason = "no production caller yet: ADR-011 §4's round trip (QSL-6 slice S3) wires this reader in; until then only this module's own tests call it"
)]
pub(crate) fn read_checked_package_v2(
    bytes: &[u8],
    identity: LibraryName,
    version: String,
    limits: PackageLimits,
    evidence: &CheckedPackageEvidence,
) -> V2ReadOutcome {
    let limits = limits.bounded();
    if bytes.len() > limits.artifact_bytes {
        return V2ReadOutcome::Incomplete(V2ReadIncomplete::Bytes {
            limit: limits.artifact_bytes,
            actual: bytes.len(),
        });
    }
    let ir_limits = CheckedPackageReadLimits {
        bytes: u64::try_from(limits.artifact_bytes).unwrap_or(u64::MAX),
        // Fail-closed (QSL-6 M2, see the module doc's "Ceilings" section):
        // passed through unchanged, never widened by one for IR's
        // scalar-counts-as-depth-1 convention, so no wire deeper than
        // `PackageLimits::depth` containers is ever admitted.
        depth: u64::try_from(limits.depth).unwrap_or(u64::MAX),
        ..CheckedPackageReadLimits::bounded()
    };
    match read_checked_package(bytes, ir_limits, evidence) {
        CheckedPackageDispatchResult::AdmittedV2(package) => {
            if !package.lock().dependency_selections.is_empty() {
                return V2ReadOutcome::Refused(V2ReadRefusal::UnsupportedDependencySelections);
            }
            let preimage_value = match serde_json::to_value(package.identity_preimage()) {
                Ok(value) => value,
                Err(error) => {
                    return V2ReadOutcome::Refused(V2ReadRefusal::Preimage(error.to_string()))
                }
            };
            // `preimage_value` is a `serde_json::Value` decoded from bytes
            // IR already verified are the whole wire's exact canonical form
            // (its own `NoncanonicalWire` check re-serializes the entire
            // document and compares it byte-for-byte against the input).
            // Canonical form is compositional: re-serializing this already-
            // parsed sub-value with `serde_json`'s own (key-sorted, no
            // inserted whitespace) `Value` representation reproduces the
            // same substring of canonical bytes IR's own `package_id`
            // recompute already hashed -- not a fresh, independent claim
            // about RFC 8785 in general, only about bytes IR itself already
            // certified.
            let preimage_bytes = match serde_json::to_vec(&preimage_value) {
                Ok(bytes) => bytes,
                Err(error) => {
                    return V2ReadOutcome::Refused(V2ReadRefusal::Preimage(error.to_string()))
                }
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
            let candidate = LibraryPackage {
                library: identity,
                version,
                package_id,
                identity_preimage: preimage_bytes.into_boxed_slice(),
                imports: Vec::new(),
                exports,
            };
            match verify_package(&candidate) {
                Ok(_) => V2ReadOutcome::Candidate(candidate),
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
        }) => V2ReadOutcome::Incomplete(V2ReadIncomplete::Limit {
            kind: limit_kind,
            limit,
            consumed,
        }),
    }
}
