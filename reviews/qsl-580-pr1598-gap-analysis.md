---
id: SR-1598002
title: "gap analysis of PR #1598 (QSL-580c, FR-232 time-lock item, FR-236 delays)"
type: SpecReview
analysis: gap-analysis
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1598 git diff ddeff5dd6...cb95a247a; FR-232 AC-1 to AC-4, FR-236-AC-4; qsl-eval/tests/it/timed_subject.rs, qsl-analyze/tests/it/{time_lock_item.rs,lasso_replay.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-232
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-236
    type: reviews
---
# SR-1598002: gap analysis of PR #1598 (QSL-580c, FR-232 time-lock item, FR-236 delays)

## Summary

Ticket: QSL-580. Plan completion: not assessed. FR-232-AC-1, AC-2 and AC-4 are asserted as written. Two tags name ACs that are only partly asserted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `a_stall_that_only_pings_is_a_non_local_time_lock` carries FR-232-AC-3, but the AC's settlement half (`always holds(true)` settles `inconclusive`, `NoAdmittedBehaviour`) is unasserted. It is deferred to QSL-581 per the PR body. | qsl-eval/tests/it/timed_subject.rs:759 |
| FND-002 | medium | `an_untimed_counterexample_carries_no_delay` carries FR-236-AC-4 but does not assert that "its bytes equal FR-126's counterexample for the same item". QSL has no byte encoding of `TemporalCounterexample` to compare. Either the AC needs a named encoding (spec gap), or the test should compare against the FR-126 engine's payload value. Today it checks only `delay.is_none()` and `UntimedDelay`. | qsl-analyze/tests/it/lasso_replay.rs:724-751 |

## Verdict

**CONDITIONAL.** No untagged claimed AC, no stub, no unowned code. Fix the two partial tags.

## Coverage

Plan completion: not assessed. FR-236-AC-1 to AC-3 are declared out of scope (they need EN-6). Semantic review: done by hand for the claimed ACs.

## Dispositions

Round 1, against #1598 fix commit f2efd8da7 on the restacked head bd68b877a (is_timed fix carried in restacked 3880da60a; `git range-diff ddeff5dd6..cb95a247a origin/main..3880da60a`).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
