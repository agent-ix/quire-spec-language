// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(feature = "std"), no_std)]
//! ADR-011 §6.1 layer 4, `package` (ADR-011 §7.3 X-7): the S4
//! in-process [`CheckedPackage`]/[`EmittedPackage`] typestate (ADR-013 T-1,
//! FR-087, the `checked` module), the
//! `quire.checked-package/v2` I2 byte reader (`checked_v2`, ADR-011 §4) and
//! the v2 emitter (`emit`, ADR-011 T-8, M-4).
//!
//! This crate depends on layer 3 (`qsl-semantics`), F (`qsl-foundation`), K
//! (`quire-exact`) and `quire-contract-model` for the v2 wire vocabulary. It never names the root crate or a
//! SEAM-1 module (`checking`, `linking`, `native_model`, or the root crate's
//! native-v1 `package`): Cargo refuses the first, and the root crate is the
//! only home of the second. The layer-5 evaluator and every later layer
//! depend on this crate, never the other way.

// NFR-015: with `std` disabled the crate is `no_std` + `alloc` and keeps the
// in-memory checked-IR path; the `quire.checked-package/v2` emitter and reader
// (`emit`, `checked_v2`) and the dependency-bearing `link_with` need
// `quire-contract-model` and are behind `std`.
extern crate alloc;

mod checked;
#[cfg(feature = "std")]
mod checked_v2;
#[cfg(feature = "std")]
mod emit;
mod export;

pub use checked::{CheckedPackage, ResolvedDependency};
#[cfg(feature = "std")]
pub use checked::{EmittedPackage, Import, LinkRefusal};
#[cfg(feature = "std")]
pub use checked_v2::{
    clause_roots, read_import_view, resolve_clause_root, supply_closure, AdmittedPackages,
    ClauseRoot, ClauseRootUnresolved, ImportViewRefusal, SourceMapDefect, UnsupportedWire,
    V2ReadLimits, V2ReadRefusal,
};
#[cfg(feature = "std")]
pub use emit::{
    checked_node_id, checked_occurrence, emit_checked, emit_checked_with_cancel, emit_package,
    Emission, EmitRefusal, OmissionCause, OmittedNode,
};
pub use export::{export_bindings, BoundItem, ExportRefusal, UnboundElement};

// The FR-105-AC-6 fault-injection seam. Public only under
// `test-support` (see `emit.rs`'s doc comments and this crate's
// `Cargo.toml`), so a dependent crate's own tests can reach it while it
// compiles into no shipped build.
#[cfg(all(feature = "std", feature = "test-support"))]
pub use emit::{checked_node_id_of, emit_checked_with_fault};

// ADR-011 §4 I2: `read_checked_package_v2`, its outcome type
// (`V2ReadOutcome`) and its refusal/incomplete types (`V2ReadRefusal`,
// `V2ReadIncomplete`) are all `pub(crate)` on `checked_v2` itself and not re-exported here: `checked_v2` returns a real
// `qsl_semantics::library::VerifiedPackage` (FR-087-AC-1, AC-3), but it
// still has no caller of its own (S3, ADR-011 §4's round trip, has not
// landed) beyond `checked_v2`'s own tests.

/// NFR-015-AC-5, TC-1111: with `std` disabled, the exports of the `quire.checked-package/v2`
/// wire do not resolve. One case per export.
///
/// ```compile_fail,E0599
/// fn named(graph: qsl_semantics::check::CheckedGraph) {
///     let _ = qsl_package::CheckedPackage::link_with(graph, Vec::new());
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn named(package: &qsl_package::CheckedPackage) {
///     let _ = package.emitted_package_id();
/// }
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::Import;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::LinkRefusal;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::EmittedPackage;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::emit_checked;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::emit_checked_with_cancel;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::emit_package;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::checked_node_id;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::checked_occurrence;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::Emission;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::EmitRefusal;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::OmissionCause;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::OmittedNode;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::read_import_view;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::supply_closure;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::clause_roots;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::resolve_clause_root;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::AdmittedPackages;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::ClauseRoot;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::ClauseRootUnresolved;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::ImportViewRefusal;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::SourceMapDefect;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::UnsupportedWire;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::V2ReadLimits;
/// ```
///
/// ```compile_fail,E0432
/// use qsl_package::V2ReadRefusal;
/// ```
///
#[cfg(not(feature = "std"))]
#[allow(dead_code)] // carries the compile-fail docs above; nothing calls it
mod behind_std {}
