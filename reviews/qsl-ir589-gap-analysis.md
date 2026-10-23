---
id: SR-12343
title: "Gap analysis of PR #1233 (IR-589 S5: qsl-package checked-IR path no_std) against NFR-015 / TC-1111"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@30e831e6d2ebf9f0186c8ad28cb40c9da13f0d71; qsl-package/**; spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md, spec/test-cases/TC-1111-qsl-package-gates-the-checked-package-wire-behind-std.md, spec/test-cases/TC-1110-qsl-foundation-builds-no-std-on-the-governed-target.md"
review_set: subset
---

## Summary

Ticket: IR-589. This is a planless gap analysis of NFR-015's `qsl-package`
crate-tier row, which TC-1111 covers (AC-4 to AC-7 and AC-9). Plan
completion: not assessed.

`quire matrix --scope . --format json` was still running after more than
10 minutes with no output, so it was stopped. The matching review of S3
(PR #1230) saw the same thing. Trace coverage was checked instead by
searching the Rust sources for `TC-1111` and `NFR-015` tags. TC-1110 (S3)
was read for context only.

Each acceptance criterion, measured at the reviewed head:

- **AC-4.** Partly met. `default = ["std"]`, `std` enables
  `dep:quire-contract-model`, and `cargo check --locked -p qsl-package
  --no-default-features` passes with the crate root `no_std`. The
  inverse `cargo tree` finds no path to `quire-contract-model`. The
  missing `qsl-foundation/std` token is the known gap that waits on S3
  (FND-005). No test is tagged with AC-4.
- **AC-5.** Met. `tests/checked_ir_surface.rs` names every export in the
  "With `std` disabled" column, and all 3 tests pass with and without
  default features. `lib.rs` has one `compile_fail` case for each of the
  26 "Behind `std`" exports, and all pass under `cargo test
  --no-default-features`, which ran 37 doc tests. No gate runs the std-off
  half (FND-002).
- **AC-6.** Met by measurement. The 6 conformance tests in
  `cargo test -p qsl-package --lib conformance` pass against
  quire-specification main 5ee0071f. The full default suite passes
  (230/3/12), including the FR-091 round trip
  (`emit/tests/admission_corpus.rs`). These tests are not tagged with
  NFR-015-AC-6 (FND-003).
- **AC-7.** Met by inspection. With `#[cfg(test)]` items excluded, no
  shipped file names `serde_json`, `HashMap` or `HashSet`, and
  `make arch-lint-canonical-encoder` passes. No test is tagged with it.
- **AC-9.** Untested (FND-001).
- **Reverse gaps.** None. Every new gate maps to a row of the NFR-015
  surface table, and nothing outside that table was gated or added.
- **Stubs.** No `todo!`/`unimplemented!`. One test name overclaims what the
  test checks (FND-004).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | NFR-015-AC-9 has no test. The AC: with default features, after `emit_checked` records an emission, `bind_abstraction` with valid bindings leaves `emitted_package_id` as `None`. No test in the crate calls `emitted_package_id` at all, and this PR moves exactly that reset (`self.emission = OnceLock::new()`) into a `#[cfg(feature = "std")]` block. If the block were mis-gated or deleted, every test would still pass. Add a default-feature unit test tagged `#[trace("TC-1111", "NFR-015-AC-9")]`. It should emit with `emit_checked`, assert `emitted_package_id().is_some()`, call `bind_abstraction(&[])`, and assert `None`. Optionally, also assert that `bind_abstraction_at` keeps the id, as NFR-015 states | qsl-package/src/checked.rs:477-488, qsl-package/src/checked.rs:532-537 |
| FND-002 | medium | TC-1111's std-off evidence runs in no gate. The 3 surface tests built with `std` off, and the 26 `compile_fail` doc tests (which compile only under `cfg(not(feature = "std"))`), run only under `cargo test -p qsl-package --no-default-features`. `make ci` never runs that command. NFR-015's `build-no-std` recipe, as specified, runs only `cargo check` and `cargo clippy` without `--all-targets`, so even after the S3 follow-up neither test kind is compiled. Concretely: delete the `#[cfg(feature = "std")]` on `pub use emit::{...}` and its module, and `make ci` stays green. The follow-up `build-no-std` row for `qsl-package` should add `cargo test --locked -p qsl-package --no-default-features` beside the check and clippy lines, and NFR-015's Gate and AC-8 should name it. Note that the xtask aggregate tests record the recipe's commands (SR-12336 FND-001), so they need updating with it | qsl-package/src/lib.rs:59-172; qsl-package/tests/checked_ir_surface.rs; spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md |
| FND-003 | low | No test is tagged with NFR-015-AC-4, AC-6 or AC-7, although NFR-015's Verification says Rust tests carry `#[trace("TC-...", "NFR-015-AC-...")]`. AC-6 is backed by real tests (the `checked_v2` conformance tests and the FR-091 round trip) that carry only their FR-322/FR-091 tags. Add `"TC-1111", "NFR-015-AC-6"` to them. AC-4's tree check and AC-7's inspection are by hand; tag the canonical-encoder arch-lint test, or record in TC-1111 that these are manual checks | qsl-package/src/checked_v2/tests.rs; qsl-package/src/emit/tests/admission_corpus.rs |
| FND-004 | low | `export_bindings_answers_each_requested_item_in_order` calls `export_bindings` only with an empty slice, so it checks neither "each item" nor "in order". `let _: Option<UnboundElement> = None;` only proves the type resolves. That is enough for AC-5, which is about resolution, but the test name claims behaviour the test does not check. Rename it, for example `export_surface_resolves_without_std`, or have it request one unbound item and assert the refusal | qsl-package/tests/checked_ir_surface.rs:54-61 |
| FND-005 | low | Known and accepted, waiting on S3 (PR #1230, open): the `std` feature does not yet enable `qsl-foundation/std` (AC-4, first clause), and `build-no-std` has no `qsl-package` row (AC-8). After S3 merges, the follow-up is one Cargo token (`"qsl-foundation/std"` in `std`) plus the Makefile row: `cargo check` and `cargo clippy --no-default-features -D warnings` for `-p qsl-package` in `target/no-std`, and the `cargo test` line from FND-002 | qsl-package/Cargo.toml:15 |

## Coverage

- Plan completion: not assessed
- Criteria in scope: NFR-015-AC-4, AC-5, AC-6, AC-7, AC-9 (TC-1111).
  Tagged: AC-5 (3 tests). Untagged: AC-4, AC-6, AC-7, AC-9.
- Matrix: `quire matrix` did not finish, so a grep index was used.
- Semantic review: not run as a separate pass. The intent, test and code
  of AC-5 and AC-9 were read directly.
- Gates run at the reviewed head: `cargo check` / `cargo clippy --all-targets
  -D warnings` / `cargo test` with `--no-default-features`, all passing;
  `cargo test -p qsl-package` with default features, passing (needs a
  current `QSPEC_DIR`); `cargo clippy --all-features --all-targets -D
  warnings`, passing; `make arch-lint-canonical-encoder
  arch-lint-api-surface-qsl arch-lint-qualified-core`, passing.

## Verdict

FAIL under the gap-analysis rule, because criteria are untagged. The
implementation meets AC-4 (minus the S3 token), AC-5, AC-6 and AC-7 by
measurement. What blocks is evidence, not code:

- AC-9 has no test at all, although this PR touches exactly the line it
  guards (FND-001, medium).
- The std-off half of TC-1111 is real (26 compile-fail cases and 3
  surface tests) but runs in no gate (FND-002, medium). This can ride the
  S3 follow-up, provided that row includes the `cargo test` line.

To merge, add the AC-9 test and the AC-6 tags in this PR (both small), and
record FND-002 against the follow-up. FND-004 and FND-005 do not block.
