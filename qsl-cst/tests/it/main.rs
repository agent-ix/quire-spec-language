// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-178 PR2: single integration-test binary for `qsl-cst`, following the
//! same `it` pattern the root crate uses (`tests/it/main.rs`, QSL-175/#306) --
//! one link step per `cargo test` invocation instead of one per file.

mod complete_grammar;
