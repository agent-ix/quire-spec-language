---
id: SR-2026110216378
title: "Code review — QSL-1525 per-use profile admission"
type: SpecReview
analysis: code-review
scope: "QSL-1525 branch diff from 5f53c2c83: qsl-analyze, qsl-eval, qsl-package, qsl-replay, qsl-semantics source and tests; FR-508 and TC-1303 acceptance"
review_set: subset
---

## Summary

Reviewed the per-use profile admission checker, refusal mapping, fixture migrations, and FR-508 coverage against the Rust conventions. The earlier QSL-1569 blocker is resolved on current main; the rebased protocol-role suite passes, while the final workspace gates remain pending.

## Verdict

**CONDITIONAL** — no implementation defect was found; final default/all-features Clippy and workspace tests remain to run on the reviewed head.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | low | The new exhaustive FR-508 semantics tests have not executed: `qsl-semantics` test compilation stops on two non-exhaustive `StageFailure` matches tracked by QSL-1569 / PR #2182. Re-run them after that fix lands. | qsl-semantics/src/check/family.rs:3476; QSL-1569 |

## Verification

- Passed: `cargo fmt --all -- --check`, `git diff --check`, and `cargo check -p qsl-semantics --lib` through `locked-build.sh`.
- Passed: public replay wrong-kind test, public two-alias AC-6 package test, QSL package compensation filter (3 tests), and earlier focused package protocol controls (28), requirements (24), and references (13) filters.
- Prior pre-#2182 run: the focused semantics coverage tests did not compile because `family.rs` lacked `StageFailure::LimitWithPayload` and `StageFailure::RefusedWithPayload`; current main contains the fix.
- Passed after rebase: `cargo test -p qsl-analyze --test it protocol_roles::` (26 passed).
- Passed static coverage for the ticket: all FR-508-AC-1..6 are tagged by live tests. The standalone strict matrix exits 1 for repository-wide gaps (1,202 untagged and 8 ignored-only criteria).
- Quoin matrix: all six FR-508 criteria are tagged; no run evidence is bound for the repository's 4,048 criteria.
- Pending: final default/all-features Clippy, workspace test targets, and `cargo deny check --disable-fetch` on the reviewed head.

## Dispositions

| FND | Outcome | Reason |
| --- | ------- | ------ |
| FND-001 | deferred: QSL-1569 | Re-run the focused semantics test after PR #2182 lands; the blocker is outside QSL-1525 scope. |
