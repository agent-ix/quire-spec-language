---
id: SR-599001
title: "code-review and rust-review of QSL-599 (FR-299 TC-784, FR-285-AC-6 tags, FR-276-AC-4)"
type: SpecReview
analysis: code-review
date: 2026-11-08
scope: "agent-ix/quire-spec-language; branch qsl-599-lc11 git diff ff0b7551d..1477a62db: qsl-replay/tests/it/value_family_end_to_end.rs, qsl-replay/tests/it/main.rs, qsl-replay/src/spine/call/execution_tests.rs, qsl-analyze/src/analyze.rs (tests)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-299
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: reviews
---
# SR-599001: code-review and rust-review of QSL-599 (FR-299 TC-784, FR-285-AC-6 tags, FR-276-AC-4)

## Summary

Ticket: QSL-599. Scope is only the two QSL-599 commits stacked on QSL-591's draft (#2576); nothing below that base was reviewed. The diff is tests and trace tags only. Rust style was loaded (dev-tools:rust-style). No cargo build was run: one shared build lane, and the coder reports confined tests plus clippy clean.

TC-784 (`value_family_end_to_end.rs`) goes through the public `qsl_replay::spine` API: `parse`, `select`, `check`, `package` and `run`, then `OutcomeDocument::{from_check, from_package, from_call}`. These tests can fail. The package-bytes assertion (`value_family_end_to_end.rs:150`) compares the hand-composed `spine::package(spine::check(..))` with `compile_package`. Both run the same spine underneath (`compile.rs:319` `run_spine_with_temporal_selections_and_execution_points`), so the comparison is not two independent computations. It is a composition-equivalence check: it fails if the library operations' defaults (limits, models, package name, dependency handling) diverge from the FR-278 composition, but not on a defect both paths share. That is exactly what AC-1 asks for ("the bytes the FR-278 composition writes"), so it is not a defect. AC-3 fails if the supplied library is ignored, because `check` of `IMPORTING` would refuse and the `expect` would panic. AC-4 maps the exit through `OutcomeDocument::from_check(..).category().exit_code()`, the frontend path.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The FR-276-AC-4 test inverts the AC instead of weakening it. The blocking happens inside item 0's work (`analyze.rs:2063-2070`), so item 0 is the item still open when the handle is cancelled. The test then asserts that item 0 keeps its engine record (`analyze.rs:2089`), which is the opposite of FR-276 Outputs and Behavior ("each item still open when the handle is cancelled settles incomplete"). Item 1 settles incomplete only because `run` checks `cancel.cause()` before starting work (`analyze.rs:1131-1137`). What the test backs is "an item not started before the cancel settles incomplete, and runs no work". The literal AC, where item 1's work blocks until the cancel, is not tested, and with the current `run` it would fail (see SR-599002 FND-001). The doc comment does not say this is a variant. Either fix `run` and test the literal AC, or at minimum drop the `FR-276-AC-4` tag and state the weaker claim in the doc. Keeping `records[0] == uncancelled[0]` pins the behaviour the spec forbids. | qsl-analyze/src/analyze.rs:2017-2098 |
| FND-002 | medium | The FR-285-AC-6 tag on `a_prepare_limit_and_a_prepare_fault_reach_the_call_failure` backs the `CallFailure::Limit` leg only with `prepare.code_bytes` (`Setting::PrepareCodeBytes`). AC-6 names `prepare.compile_work_units`. The `PrepareCompileWorkUnits => PrepareSetting::CompileWorkUnits` arm of `call_failure` (`qsl-eval/src/execution.rs:477`) is exercised by no exit-mapping test. The NotSupplied (`execution_tests.rs:1213`) and Untranslatable (`execution_tests.rs:1270`) legs do back AC-6 as written. | qsl-replay/src/spine/call/execution_tests.rs:384-418 |
| FND-003 | low | The FR-276-AC-4 test's rendezvous is a spin loop, `while cancel.cause().is_none() { yield_now() }`, with no bound. If the canceller thread panics or never sends, the test hangs instead of failing. Use a second channel, or a bounded wait that panics. Also, item 1 is built through `AnalyzeItem::new` directly while item 0 uses the local `item` helper: one helper taking `impl FnOnce + 'a` would serve both. | qsl-analyze/src/analyze.rs:2049-2084 |

## Verdict

**NOT MERGEABLE as tagged.** The FR-299 TC-784 tests are sound, go through the public API, and can fail. AC-1 to AC-4 are backed as written. The `execute` leg goes through `spine::run`, which compiles and then executes on the interpreter through the execution seam. That is the `quire run` path, and `from_run(Ok) == from_call` (`outcome.rs:1793`). The FR-285-AC-6 tags are two thirds correct (FND-002). FND-001 is blocking: the FR-276-AC-4 test asserts the opposite of the AC's open-item rule.

## Dispositions

Round 1, against fix commit 14ebd238a (head). The branch was rebased onto 3fd46b41a; `git range-diff` shows be2d3d5a2 and 6d649c8d5 equal (`=`) to 5f54991eb and 1477a62db.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |

Round 2, against a97c9d447 (head; 14ebd238a is its parent). No finding was still open, so no rows were added. I reviewed the new parts and they are clean: the `tripped()` post-work read and the `poll()`-charging test engine in `qsl-analyze/src/analyze.rs`; `tc_792_reference_completions_serialize_the_returned_reference` in `qsl-replay/src/spine/call/tests.rs`; and the deletion of `xtask/tests/drift_gate_wiring.rs`. Nothing else references `DRIFT_GATES`, `NON_DRIFT_LANES` or the file, apart from ADR-032 §3 on this branch, which spec PR #2582 (b700b6c75, on main) removes.

Round 3 (final), against 1e8e19e09. No findings were open. The two new tests (`a_cancel_after_the_last_charge_leaves_the_verdict_standing`, `tc_792_a_reference_completion_is_not_a_clause_truth`) reviewed clean. `.unwrap()` in `spine/clause/tests.rs` follows that file's existing convention.
