// SPDX-License-Identifier: AGPL-3.0-or-later
//! Workspace `xtask`: `cargo xtask seam-probe`, `cargo xtask string-edge`
//! and `cargo xtask route-lint`, the gates `make ci` runs standalone (the
//! `Makefile`'s `seam-probe`/`string-edge`/`route-lint` targets).
//! `import_graph`, `definition_scan` and `typestate_scan` are plain library
//! modules backing their own `#[cfg(test)]` suites.
#![forbid(unsafe_code)]

pub mod definition_scan;
pub mod error;
pub mod import_graph;
pub mod route_lint;
pub mod seam_probe;
pub mod string_edge;
pub mod typestate_scan;

pub use error::{Error, Result};

/// `Self`'s type name for an `impl` block, for the common case `seam_probe`
/// and `string_edge` both need (`impl PlainType { ... }`) -- shared here
/// (PR #262 review, F8: the same one-fact-one-place instinct that keyed
/// `seam_probe`'s own checked-in locations on the enclosing item's name
/// instead of a line number) rather than kept as two near-identical private
/// copies.
///
/// **Known limitation (rust-review pre-handoff pass, carried over
/// unchanged).** A `Self` type neither tool uses for a real seam --
/// `impl<T> Foo<T>`, `impl Foo<Bar>`, `impl &Foo`, a tuple or reference type
/// -- falls into the `"<impl>"` catch-all below, which would silently
/// collapse two *different* impl blocks' methods of the same name into one
/// indistinguishable key if two such impls ever both held a checked seam or
/// string-edge occurrence in the same file. Widening this to handle every
/// `syn::Type` shape distinctly is speculative against today's real call
/// sites; broaden it if a future one actually needs a generic or otherwise
/// non-`Type::Path` `Self`.
pub(crate) fn impl_self_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_else(|| "<impl>".to_owned()),
        _ => "<impl>".to_owned(),
    }
}
