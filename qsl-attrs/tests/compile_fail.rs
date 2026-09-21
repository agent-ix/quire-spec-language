// SPDX-License-Identifier: AGPL-3.0-or-later
//! PR #262 review, finding F18 (round 3, item 9): `#[string_edge]` takes no
//! arguments, and the attribute's own doc says so
//! (`src/lib.rs`'s doc on `string_edge`) -- but until this test, nothing
//! checked-in actually attempted to compile
//! `#[string_edge(anything)]` and confirmed it fails. `trybuild` compiles
//! the real fixture in `tests/ui/` as a separate crate and asserts the
//! build fails, closing that gap.

#[test]
fn ui() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/string_edge_with_args.rs");
}
