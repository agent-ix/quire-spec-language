// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-175 (#306): shared helpers for `tests/it/*`, reached as
//! `crate::support::<name>` from every module in the single `it` integration
//! test binary. Formerly each test file re-included the file it needed via
//! its own private `#[path = "support/<name>"] mod <local-name>;`,
//! duplicating the compiled module once per test binary (one binary per
//! `tests/*.rs` file); now each of these compiles once, here, and callers
//! reach it with `use crate::support::<name> as <local-name>;`, keeping
//! their previous unqualified name.
//!
//! **`runtime_setup` and `native_rule_model` are re-exports, not their own
//! `mod`.** `runtime_setup.rs` is *also* compiled directly into the library
//! crate (`src/lib.rs`'s `#[cfg(test)] mod runtime_test_setup;`) and nested
//! inside `standalone_setup.rs`, which is itself *also* compiled directly
//! into `examples/standalone_fixtures.rs` (both pre-dating and outside this
//! ticket's scope). Both keep their own internal `#[path]` declarations
//! file-relative rather than `crate::support::...`, since that path would
//! not exist in those other hosts. So this module re-exports
//! `standalone_setup`'s already-nested `runtime_setup` copy (and, through
//! it, the `native_rule_model` copy `runtime_setup.rs` itself nests)
//! instead of declaring either again, which would load the same file twice
//! within this one crate (`clippy::duplicate_mod`, `-D warnings`).
//!
//! `runtime_evaluation_invariants.rs` is not declared below: its only
//! `#[path]` caller is `src/runtime/validation/api.rs`'s own
//! `#[cfg(test)]` unit test module (pre-dating and outside this ticket's
//! scope, not `tests/it/*`), so it stays out of this module's tree
//! entirely, exactly as before.
//!
//! `config_version` is `examples/config-version/fixtures.rs`, outside
//! `tests/` entirely; it moves here for the same reason as everything else
//! below -- three now-`it` modules each used to include it as their own
//! private copy (`clippy::duplicate_mod` denies that once they share a
//! crate).

pub mod composed_types;
#[path = "../../examples/config-version/fixtures.rs"]
pub mod config_version;
pub mod located_json;
pub mod native_protocol;
pub mod package_vector_setup;
pub mod protocol_artifact;
pub mod standalone_setup;
pub mod temporal;
pub mod type_form;

// See this module's own doc above: both are re-exports of copies nested
// inside `standalone_setup`, not fresh `mod` declarations.
pub(crate) use runtime_setup::native_rule_model;
pub(crate) use standalone_setup::runtime as runtime_setup;
