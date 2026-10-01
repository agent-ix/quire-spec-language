// SPDX-License-Identifier: AGPL-3.0-or-later
//! Single integration-test binary for `qsl-cst`, following the
//! same `it` pattern the root crate uses (`tests/it/main.rs`) --
//! one link step per `cargo test` invocation instead of one per file.

mod complete_cst;
mod complete_grammar;
mod cst_identity;
mod limits;
mod nesting_levels;
mod selection;
