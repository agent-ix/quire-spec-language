---
id: SR-1780002
title: "gap analysis of PR #1780 (QSL-1005d): timed refinement liveness on zones"
type: SpecReview
analysis: gap-analysis
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1780 at 206a6646e against FR-430-AC-5, FR-143, FR-144, FR-237 Lasso"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1780002: gap analysis of PR #1780 (QSL-1005d)

## Summary

`timed_refinement_liveness.rs` (TC-1170, FR-430-AC-5) covers the outcome level:
- a positive-delay loop refutes;
- a zero-delay loop does not refute, and its lasso is refused at replay;
- `F_C` fairness, abstract unfairness, stem rollback, an idle tail and a tick source;
- `Unsupported`, drift and the liveness product's own counts.

It also covers the item level in `the_item_settles_from_its_liveness_half_once_safety_is_proved` (refuted with the lasso; with the invariant, `Proved{Exhaustive, Uncertified}`).

**FND-001.** Two settlement rows are untested:
- a liveness half that is `Unsupported` after a proved safety half settles V-8 `unsupported`;
- a rejected certificate with a `Violated` liveness half settles the safety half's record, not `refuted`.

Both are correct by inspection. They are the rows that decide whether liveness can override safety.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No settlement test of `Unsupported` liveness after proved safety (V-8), or of a rejected certificate with a `Violated` liveness half (safety record kept) | FR-144, ADR-020 AX-5 |

## Verdict

Mergeable. The finding is low.
