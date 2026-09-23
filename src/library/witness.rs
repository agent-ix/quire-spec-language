// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §4 condition 1's witness, in its own module so that its field is
//! private to this module alone: no other code, including the rest of
//! `library`, can build one except through
//! [`SupportedV2Wire::attest_ir_admitted_v2`].
//!
//! That function has to be callable from `checked_package::checked_v2`, the
//! layer-4 v2 reader, which is a separate crate once QSL-181 extracts
//! `qsl-semantics` (and X-7 `qsl-package`). Rust has no visibility that
//! names one module of another crate, so the minter is `pub`, and two
//! checks confine its callers instead of the compiler: arch-lint rule T12-E
//! (`tools/arch-lint/api_surface.rs`, run on this tree by
//! `tc_arch_lint_api_surface_024` in `cargo test --workspace`) fails on any
//! shipped reference outside `checked_package::checked_v2`, and
//! `tests/it/verified_binding_witness.rs` fails unless the only reference
//! outside this module and `library`'s own tests is one call inside
//! `read_checked_package_v2`'s `AdmittedV2` arm, including any reference
//! written inside a macro. This is ADR-013 O-04's pattern for the `pub`
//! kernel `NodeKey` constructor (T12-B). The field stays private, so the
//! minter is the only way to build one:
//!
//! ```compile_fail
//! let forged = quire_spec_language::library::SupportedV2Wire(());
//! ```

/// IR's I04 reader admitted the bytes as a supported
/// `quire.checked-package/v2` wire. `library` is layer 3 and may not name
/// IR's admitted package type (ADR-011 §6.1: only layer-4 `package` depends
/// on `quire-contract-model`), so the layer-4 reader attests it with this
/// token instead.
#[derive(Debug)]
pub struct SupportedV2Wire(());

impl SupportedV2Wire {
    /// Attest that IR's v2 reader has just admitted the bytes the candidate
    /// is derived from. Called only from `read_checked_package_v2`'s
    /// `AdmittedV2` arm.
    ///
    /// `pub` only for the QSL-181 crate boundary: arch-lint rule T12-E fails
    /// on a shipped call from any module but `checked_package::checked_v2`
    /// (see this module's doc).
    pub fn attest_ir_admitted_v2() -> Self {
        Self(())
    }
}
