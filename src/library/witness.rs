// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §4 condition 1's witness, in its own module so that its field is
//! private to this module alone: no other code, including the rest of
//! `library`, can build one except through
//! [`SupportedV2Wire::attest_ir_admitted_v2`].
//!
//! That function has to be callable from `checked_package::checked_v2`, the
//! layer-4 v2 reader. Rust restricts visibility only to an ancestor module
//! (`pub(in path)`, E0742), and `checked_v2` is not an ancestor of
//! `library`, so the compiler cannot confine the call to that one module.
//! `tests/it/verified_binding_witness.rs` does: it fails unless the only
//! reference outside this module and `library`'s own tests is one call
//! inside `read_checked_package_v2`'s `AdmittedV2` arm, including any
//! reference written inside a macro.

/// IR's I04 reader admitted the bytes as a supported
/// `quire.checked-package/v2` wire. `library` is layer 3 and may not name
/// IR's admitted package type (ADR-011 §6.1: only layer-4 `package` depends
/// on `quire-contract-model`), so the layer-4 reader attests it with this
/// token instead.
#[derive(Debug)]
pub(crate) struct SupportedV2Wire(());

impl SupportedV2Wire {
    /// Attest that IR's v2 reader has just admitted the bytes the candidate
    /// is derived from. Called only from `read_checked_package_v2`'s
    /// `AdmittedV2` arm.
    pub(crate) fn attest_ir_admitted_v2() -> Self {
        Self(())
    }
}
