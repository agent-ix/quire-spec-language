// SPDX-License-Identifier: AGPL-3.0-or-later
//! Layer-4 `package` (ADR-011 §6.1), the future `qsl-package` crate
//! (QSL-182, ADR-011 §7.3 X-7): the S4 in-process
//! [`CheckedPackage`]/[`EmittedPackage`] typestate (ADR-013 T-1, FR-087,
//! QSL-158 S-3a, this module's own `checked` submodule), the
//! `quire.checked-package/v2` I2 byte reader (`checked_v2`, ADR-011 §4) and
//! the v2 emitter (`emit`, ADR-011 T-8, M-4).
//!
//! Split out of `src/package.rs` (QSL-182 prep): this module tree is meant
//! to become `qsl-package`'s own crate root unchanged, so its own imports
//! are exactly what that future crate may depend on -- layers F, K, 1, I3,
//! 2 and 3 (`check`, `library`, `diagnostic`, and external crates such as
//! `quire_contract_ir`) -- and never SEAM-1 (`checking`, `linking`,
//! `native_model`, or `package`'s own staying submodules). The sibling
//! `package` module (`NativePackage` and the native-linked-package/1
//! submodules, SEAM-1 until M-6 deletes them) is free to depend on this
//! module -- and does, for [`PackageLimits`], the ceiling type both the
//! native encode path and this module's own I2 reader honor -- but nothing
//! here may depend back on `package`.

mod checked;
mod checked_v2;
mod emit;

pub use checked::{CheckedPackage, EmittedPackage};

// ADR-011 §4 I2: `read_checked_package_v2`, its outcome type
// (`V2ReadOutcome`) and its refusal/incomplete types (`V2ReadRefusal`,
// `V2ReadIncomplete`) are all `pub(crate)` on `checked_v2` itself (QSL-6
// review) and not re-exported here: its candidate is not yet a
// checked-package-crossing type any caller outside this crate should see
// (`resolve_libraries` check 3 has not run, and FR-087's `VerifiedPackage`
// does not exist yet), and this module has no caller yet (S3, ADR-011 §4's
// round trip, has not landed) beyond `checked_v2`'s own tests.

/// Inclusive per-pass ceilings; elevated options clamp to the defaults.
///
/// Shared by both package formats (QSL-182 prep): the sibling `package`
/// module's native v1 encode path (`package::encoding`, `package::intake`)
/// honors all four fields; the `quire.checked-package/v2` byte reader
/// (`checked_v2`, ADR-011 §4 I2) honors only `artifact_bytes` and `depth` --
/// IR's own I04 reader has no decode-time meter for `string_bytes` or
/// aggregate `entries` at all (IR-238 item 2), so a caller of that reader
/// who sets either of those two fields gets no enforcement of them.
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
    /// `pub(crate)`: called from both this module's own `checked_v2` and
    /// from the sibling `package` module's `NativePackage::new` (QSL-182
    /// prep widened this from private-to-module, since the two are now
    /// separate modules; both call sites are real, pre-existing callers,
    /// not a speculative widening).
    pub(crate) fn bounded(self) -> Self {
        let hard = Self::default();
        Self {
            artifact_bytes: self.artifact_bytes.min(hard.artifact_bytes),
            string_bytes: self.string_bytes.min(hard.string_bytes),
            entries: self.entries.min(hard.entries),
            depth: self.depth.min(hard.depth),
        }
    }
}
