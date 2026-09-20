// SPDX-License-Identifier: AGPL-3.0-or-later
//! PR #262 review, finding F18: `#[string_edge(anything)]` must fail to
//! compile. An earlier version of `qsl_attrs::string_edge` silently
//! dropped its `attribute` argument instead, so a typo'd or misremembered
//! argument compiled cleanly and did nothing -- contradicting the
//! attribute's own doc, which states none are admitted. This fixture is
//! the compile-fail case `trybuild` (`tests/compile_fail.rs`) runs.

#[qsl_attrs::string_edge(anything)]
fn edge(_kind: &str) -> bool {
    true
}

fn main() {}
