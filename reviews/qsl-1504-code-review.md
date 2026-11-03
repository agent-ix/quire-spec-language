---
id: SR-2026110216376
title: "Code and Rust review of the FR-527 mapping classifier (QSL PR #2170)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-package/src/checked_views/mapping.rs, qsl-package/src/checked_views/subject.rs, qsl-package/src/emit/tests/mapping_support.rs, tests/it/mapping_support.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-527
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1322
    type: reviews
---
# SR-2026110216376: Code and Rust review of the FR-527 mapping classifier (QSL PR #2170)

## Summary

Ticket: QSL-1504. PR #2170 at 6f037fe08 against main 2b2f4fa3d. Code-review with the Rust lane folded in.

The classifier matches FR-045's table row by row, including the order of the accumulating rows, the past-only row under both closures, `total_sample_valuation` only on the fixed-sample future row, and the timed profile. `AuthenticatedSelection` has private fields and no constructor, and two compile-fail doctests prove it (both go red when a field is made `pub` or a `pub fn new` is added). The `#[string_edge]` marker sits on the one function that compares an operation identity to literals, as in `qsl-cst`; `xtask string-edge` reports nothing in `mapping.rs`. The legacy `src/temporal/mapping.rs` is untouched. The classifier reads `profile()`, `formula()[..].operation`, `declaration()`, `activation()` and `package().name()`; it reads no formula bytes, backend report, or TL version.

Mutation results (each run of `cargo test -p qsl-package --lib mapping_support` went red): direction swap, `since` dropped from past, `_ => Future` for unknown operations, no accumulation, closure swap, closure ignored, timed row removed, valuation set for past-only, valuation set without a future operator, selection package/definition/clause replaced, `NoProfileSelection` removed. Compile-fail doctests: making the fields `pub` fails the E0451 doctest, adding `pub fn new` fails the E0599 doctest, passing `Closure::Open` fails the AC-11 doctest (so its imports and call shape are sound).

Gates on the PR head: `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, the same with `--no-default-features`, root `--features checked-handoff` clippy, and `arch-lint qualified-core` all pass. `cargo test -p qsl-package`: 539 passed, 30 failed, 4 ignored; the 30 failing names are identical to a control run on clean origin/main (525 passed, 30 failed, 3 ignored). `make string-edge` reports 76 unmarked occurrences on both main and the PR, none in `mapping.rs`. The integration test `mapping_support` under `checked-handoff` passes. The ignored AC-24 test calls `unimplemented!` and cannot run except under `--include-ignored`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The AC-21 test asserts only that `read_wire` returns `None` for the stripped and renamed `profile_selections` rows. Any other refusal (digest, canonical encoding, limits) would pass it. The reader's cause today is `OperationLawUnselected` at the clause's `laws/0/definition` (measured with a probe), so the test is correct now but does not pin it. | qsl-package/src/emit/tests/mapping_support.rs:524-560 |
| FND-002 | low | AC-22 and AC-23 present the timed profile by swapping the view's profile row (`with_profile`) on an infinite-trace package. No package whose reader admitted a timed clause is classified, so the timed row is tested on a row the lock does not hold for that clause. The doc comment states this, and the classifier reads only the identity, so the result is the same; but the AC reads "a clause selected under `quire.temporal.timed/v1`". | qsl-package/src/emit/tests/mapping_support.rs:620-690 |
| FND-003 | low | AC-13's "holds no clock parameter map" is checked as a Debug-string search for `clock` or `parameter`. A field named `params` or `map` would pass. The struct has three private fields, so the property holds structurally; the test adds little. | qsl-package/src/emit/tests/mapping_support.rs:441-478 |

## Verdict

Mergeable on the code-review and Rust lanes. No high or medium finding. The three low findings are test-strength notes and may be taken or declined.
