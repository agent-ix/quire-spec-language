// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-019/021: immutable native checked artifacts and source-bound static
//! identity, plus ADR-013 T-1 (FR-087, QSL-158 S-3a)'s canonical, layer-4
//! [`CheckedPackage`]/[`EmittedPackage`] typestate (defined in this module's
//! private `checked` submodule and re-exported below). The two are unrelated:
//! this file's own top-level `NativePackage` wraps the lane-private
//! `checking::CheckedPackage<'a>` (ADR-013 §6), referenced here by its full
//! path rather than a bare `use` import, precisely so that name stays
//! distinct from [`CheckedPackage`] in this module's own item namespace (both
//! are reachable as
//! `crate::package::*` items once `checked`'s canonical type is re-exported
//! below) -- `src/package/features.rs` and `src/package/view.rs` keep their
//! own pre-existing, unrelated `use crate::checking::CheckedPackage;`
//! imports unchanged (FR-087-AC-10/TC-247).

mod checked;
mod checked_v2;
mod emit;
mod encoding;
mod features;
mod intake;
mod reading;
#[cfg(test)]
mod tests;
mod view;
mod wire;

pub use checked::{CheckedPackage, EmittedPackage};
pub use reading::{PackageReadLimits, PackageSupport};

// ADR-011 §4 I2: `read_checked_package_v2`, its outcome type
// (`V2ReadOutcome`) and its refusal/incomplete types (`V2ReadRefusal`,
// `V2ReadIncomplete`) are all `pub(crate)` on `checked_v2` itself (QSL-6
// review) and not re-exported here: its candidate is not yet a
// checked-package-crossing type any caller outside this crate should see
// (`resolve_libraries` check 3 has not run, and FR-087's `VerifiedPackage`
// does not exist yet), and this module has no caller yet (S3, ADR-011 §4's
// round trip, has not landed) beyond `checked_v2`'s own tests.

use std::fmt;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{ByteDigest, Code, Diagnostic};

/// Inclusive per-pass ceilings; elevated options clamp to the defaults.
///
/// Not every reader in this module enforces every field. The
/// `quire.checked-package/v2` byte reader (`checked_v2`, ADR-011 §4 I2)
/// honors only `artifact_bytes` and `depth`: IR's own I04 reader has no
/// decode-time meter for `string_bytes` or aggregate `entries` at all
/// (IR-238 item 2), so a caller of that reader who sets either of those two
/// fields gets no enforcement of them.
#[derive(Clone, Copy, Debug)]
pub struct PackageLimits {
    /// Offered or emitted bytes, at most 16 MiB.
    pub artifact_bytes: usize,
    /// Inspected decoded strings, including member names, at most 16 MiB.
    pub string_bytes: usize,
    /// Aggregate object members and array elements, at most 100,000.
    pub entries: usize,
    /// Entered JSON containers, at most 128.
    pub depth: usize,
}

impl Default for PackageLimits {
    fn default() -> Self {
        Self {
            artifact_bytes: 16_777_216,
            string_bytes: 16_777_216,
            entries: 100_000,
            depth: 128,
        }
    }
}

impl PackageLimits {
    fn bounded(self) -> Self {
        let hard = Self::default();
        Self {
            artifact_bytes: self.artifact_bytes.min(hard.artifact_bytes),
            string_bytes: self.string_bytes.min(hard.string_bytes),
            entries: self.entries.min(hard.entries),
            depth: self.depth.min(hard.depth),
        }
    }
}

/// Actual admitted work in one package pass; inapplicable fields are zero.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PackagePassUsage {
    /// Output bytes retained before successful completion or refusal.
    pub output_bytes: usize,
    /// Decoded UTF-8 string bytes inspected, including member names.
    pub string_bytes: usize,
    /// Object members and array elements admitted before traversal.
    pub entries: usize,
    /// Greatest admitted container depth.
    pub max_depth: usize,
}

/// Request-local measured passes; an unentered pass remains absent.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackageUsage {
    /// Entire raw input length admitted before hashing; zero for construction.
    pub admitted_input_bytes: usize,
    /// Generic raw JSON recognition.
    pub recognition: Option<PackagePassUsage>,
    /// Closed version-specific typed decoding.
    pub decode: Option<PackagePassUsage>,
    /// Complete manifest inspection before output.
    pub derive: Option<PackagePassUsage>,
    /// Native static canonical content encoding.
    pub canonical: Option<PackagePassUsage>,
    /// Complete artifact encoding.
    pub encode: Option<PackagePassUsage>,
    /// Regenerated claim comparison.
    pub compare: Option<PackagePassUsage>,
    /// Actual successful frontend checking work, retained even if comparison fails.
    /// Parser/linker APIs and failed checks expose no counters to manufacture here.
    pub checking: Option<crate::checking::CheckUsage>,
}

/// Boundary that refused a package; native causes preserve their own phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageStage {
    /// Constructing immutable bytes.
    Encode,
    /// Recognizing or decoding selected package bytes.
    Decode,
    /// Checking external dependencies through the actual compiler.
    Rebind,
    /// Comparing reconstructed claims.
    Compare,
}

impl fmt::Display for PackageStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Encode => "encode",
            Self::Decode => "decode",
            Self::Rebind => "rebind",
            Self::Compare => "compare",
        })
    }
}

/// Located package member or array occurrence, without an invented native span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackagePathSegment {
    /// A decoded object member.
    Field(String),
    /// A zero-based array element.
    Index(usize),
}

/// Original structured cause; message text is never parsed to recover a code.
#[derive(Debug, thiserror::Error)]
pub enum PackageCause {
    /// Original Serde error, retaining its real line and column when decoding.
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    /// Original native compiler refusal with no linking or checking context.
    #[error("{0}")]
    Native(#[from] Box<Diagnostic>),
    /// Original formal-environment linking refusal (FR-020: retains its
    /// original related declarations and upstream formal diagnostic).
    #[error("{0}")]
    Linking(#[from] Box<crate::linking::LinkingError>),
    /// Original native constraint/proof checking refusal (FR-020: retains
    /// its original upstream IR proof refusal).
    #[error("{0}")]
    Checking(#[from] Box<crate::checking::CheckingError>),
}

/// Atomic package refusal with stable code, actual path and per-pass accounting.
#[derive(Debug, thiserror::Error)]
#[error("{code} at {stage}: {message}")]
pub struct PackageError {
    /// Stable crate diagnostic code.
    pub code: Code,
    /// Boundary at which this request stopped.
    pub stage: PackageStage,
    /// Original field/index path available at the stopped operation.
    pub path: Vec<PackagePathSegment>,
    /// Actual completed and partial pass measurements.
    pub usage: PackageUsage,
    /// Human-readable explanation; callers select behavior by code.
    pub message: &'static str,
    /// Original JSON or native error where one exists.
    #[source]
    pub cause: Option<PackageCause>,
}

impl PackageError {
    /// Whether the retained native classification records incomplete work.
    pub fn is_incomplete(&self) -> bool {
        self.code.is_incomplete()
    }
}

/// Exact byte selector under the fixed native-linked-package/1 format.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativePackageRef {
    digest: ByteDigest,
}

impl NativePackageRef {
    /// Select bytes without claiming they exist or have passed checking.
    pub fn new(digest: ByteDigest) -> Self {
        Self { digest }
    }
    /// Selected complete artifact hash.
    pub fn digest(&self) -> ByteDigest {
        self.digest
    }
    /// Exact wire format selected by this reference type.
    pub fn format(&self) -> &'static str {
        view::FORMAT
    }
}

/// Source-bound static identity in quire.native.bound-package/v1, using SHA-256.
///
/// TC-091: a native static identity cannot select raw package bytes:
/// ```compile_fail,E0308
/// use quire_spec_language::package::{NativePackageIdentity, NativePackageRef};
/// fn select(identity: NativePackageIdentity) {
///     let _ = NativePackageRef::new(identity);
/// }
/// ```
/// Nor can it substitute for the independent Contract IR canonical role:
/// ```compile_fail,E0308
/// use quire_spec_language::package::NativePackageIdentity;
/// fn substitute(identity: NativePackageIdentity) {
///     let _: quire_contract_ir::CanonicalDigest = identity;
/// }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativePackageIdentity([u8; 32]);

impl NativePackageIdentity {
    fn of(canonical: &[u8]) -> Self {
        let mut hash = Sha256::new();
        hash.update(b"quire-spec-language\0quire.native.bound-package/v1\0linked-package\0");
        hash.update(canonical);
        Self(hash.finalize().into())
    }
}

impl fmt::Display for NativePackageIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Immutable checked source, complete artifact bytes and separately typed identities.
#[derive(Debug)]
pub struct NativePackage<'model> {
    checked: crate::checking::CheckedPackage<'model>,
    bytes: Vec<u8>,
    digest: ByteDigest,
    canonical_identity: NativePackageIdentity,
    usage: PackageUsage,
}

impl<'model> NativePackage<'model> {
    /// Reconstruct through the actual compiler under explicit external authority.
    ///
    /// # Errors
    /// Refuses malformed or unsupported wire, foreign dependencies, native
    /// compiler failures, forged derived claims and exhausted selected limits.
    pub fn read_verified(
        bytes: &[u8],
        expected: NativePackageRef,
        bindings: crate::checking::CheckBindings,
        models: &'model [crate::native_model::NativeModel],
        support: &PackageSupport,
        limits: PackageReadLimits,
    ) -> Result<Self, Box<PackageError>> {
        reading::read(bytes, expected, bindings, models, support, limits)
    }
    /// Encode a real checked package with no runtime inputs or capability overrides.
    ///
    /// # Errors
    /// Returns the actual bounded pass refusal, without exposing a partial package.
    pub fn new(
        checked: crate::checking::CheckedPackage<'model>,
        limits: PackageLimits,
    ) -> Result<Self, Box<PackageError>> {
        let limits = limits.bounded();
        let mut usage = PackageUsage::default();
        let features = features::derive(&checked)?;
        let manifest = view::Manifest::new(&checked, &features, true, None);
        run(&manifest, limits, Pass::Derive, &mut usage)?;
        let canonical = run(
            &view::Manifest::new(&checked, &features, false, None),
            limits,
            Pass::Canonical,
            &mut usage,
        )?;
        let canonical_identity = NativePackageIdentity::of(&canonical);
        drop(canonical);
        let bytes = run(
            &view::Manifest::new(&checked, &features, true, Some(canonical_identity)),
            limits,
            Pass::Encode,
            &mut usage,
        )?;
        Ok(Self {
            digest: ByteDigest::of(&bytes),
            checked,
            bytes,
            canonical_identity,
            usage,
        })
    }
    /// Original checked source/model/runtime-obligation correspondence.
    pub fn checked(&self) -> &crate::checking::CheckedPackage<'model> {
        &self.checked
    }
    /// Complete immutable accepted artifact bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Exact complete artifact hash, separate from the static native identity.
    pub fn digest(&self) -> ByteDigest {
        self.digest
    }
    /// Native static identity derived under the named domain and byte profile.
    pub fn canonical_identity(&self) -> NativePackageIdentity {
        self.canonical_identity
    }
    /// Exact byte selector for this artifact.
    pub fn reference(&self) -> NativePackageRef {
        NativePackageRef::new(self.digest)
    }
    /// Actual request-local measurements.
    pub fn usage(&self) -> &PackageUsage {
        &self.usage
    }
}

#[derive(Clone, Copy)]
enum Pass {
    Derive,
    Canonical,
    Encode,
}

fn run(
    value: &impl Serialize,
    limits: PackageLimits,
    pass: Pass,
    usage: &mut PackageUsage,
) -> Result<Vec<u8>, Box<PackageError>> {
    let result = encoding::run(value, limits, !matches!(pass, Pass::Derive));
    let measured = match &result {
        Ok((_, measured)) => *measured,
        Err(failure) => failure.usage,
    };
    match pass {
        Pass::Derive => usage.derive = Some(measured),
        Pass::Canonical => usage.canonical = Some(measured),
        Pass::Encode => usage.encode = Some(measured),
    }
    result.map(|(bytes, _)| bytes).map_err(|failure| {
        Box::new(PackageError {
            code: failure.code,
            stage: PackageStage::Encode,
            path: failure.path,
            usage: usage.clone(),
            message: "native package encoding did not complete",
            cause: Some(PackageCause::Json(failure.cause)),
        })
    })
}
