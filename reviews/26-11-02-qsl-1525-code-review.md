---
id: SR-2026110216378
title: "Code review — QSL-1525 per-use profile admission"
type: SpecReview
analysis: code-review
scope: "QSL-1525 branch diff from 5f53c2c83: qsl-analyze, qsl-eval, qsl-package, qsl-replay, qsl-semantics source and tests; FR-508 and TC-1303 acceptance"
review_set: subset
---

## Summary

Reviewed the per-use profile admission checker, refusal mapping, fixture migrations, and FR-508 coverage against the Rust conventions. No implementation defect was found in the inspected changes; the remaining gate limitation is the unrelated QSL-1569 test compilation failure.

## Verdict

**CONDITIONAL** — no implementation defect was found, but the new semantics test matrix and strict traceability matrix could not be completed while QSL-1569 remains red.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | low | The new exhaustive FR-508 semantics tests have not executed: `qsl-semantics` test compilation stops on two non-exhaustive `StageFailure` matches tracked by QSL-1569 / PR #2182. Re-run them after that fix lands. | qsl-semantics/src/check/family.rs:3476; QSL-1569 |

## Verification

- Passed: `cargo fmt --all -- --check`, `git diff --check`, and `cargo check -p qsl-semantics --lib` through `locked-build.sh`.
- Passed: public replay wrong-kind test, public two-alias AC-6 package test, QSL package compensation filter (3 tests), and earlier focused package protocol controls (28), requirements (24), and references (13) filters.
- Blocked: `cargo test -p qsl-semantics --lib protocol_clause::tests::coverage::` fails before executing tests because `family.rs` does not cover `StageFailure::LimitWithPayload` and `StageFailure::RefusedWithPayload`; these are assigned to QSL-1569 / PR #2182 and were not changed here.
- Blocked: Clippy stops at the unrelated `qsl-cst/src/parser.rs:241` `double_ended_iterator_last` lint.
- Unavailable: `cargo deny check` cannot lock its advisory database under the read-only Cargo home.
- Incomplete: `quire matrix --scope . --strict` did not finish within the available run window; the 90-second TSV retry timed out without producing matrix rows.

## Dispositions

| FND | Outcome | Reason |
| --- | ------- | ------ |
| FND-001 | deferred: QSL-1569 | Re-run the focused semantics test after PR #2182 lands; the blocker is outside QSL-1525 scope. |
