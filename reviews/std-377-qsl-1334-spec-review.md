---
id: SR-16323
title: "spec-review of FR-194 optional min_count (quire-spec-language PR 1334)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@995191af; spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md; spec/decisions/ADR-024-statistical-and-probabilistic-properties.md; spec/test-cases/TC-629-window-aggregates-in-monitors.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-194
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-024
    type: references
---

# Spec review of FR-194 optional min_count (quire-spec-language PR 1334)

## Summary

Ticket: STD-377. One spec-review pass over `git diff origin/main...995191af`, checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (key vectors derived by hand), id uniqueness across all open PR heads, cross-repo names, mergeability and gates.

Gate: `QSPEC_DIR=<worktree of QSpec PR 397 head> make check-spec-validation`: exit 0, 19 warnings (none in the touched files).

Examined, clean:

- FR-194-AC-9 by hand: window `past[0, 9]` at position 0 holds position 0 only, so `count over past[0, 9] >= 1` is 1 and true; over `x` 1, 2 at position 1 the filtered `count where holds(e.x > 5)` is 0 and below `min_count` 1, so `InsufficientData{count: 0, min_count: 1}`; `min_count 2` gives `InsufficientData{1, 2}` at position 0 and true at position 1. All hold.
- FR-194-AC-10 by hand: with `min_count 0` the filtered `count` of 0 is defined, `0 <= 2` true and `0 >= 1` false, and the `sum` over no value is 0, so `= 0` is true. The `min`, `max`, `fraction` and `quantile` terms with `min_count 0` stay refused, as on main.
- The statements match QSpec PR 397 head (FR-414 "min_count" paragraph, FR-414-AC-11 and AC-12, shared grammar `[ 'min_count', uint ]`). AC-4 and AC-7 now refuse `min_count 0` on a `max` term only, and the quantile-plus-`min_count 0` two-fault sentence of AC-7 still holds.
- No deferral or narrowing language. The `min_count 0` refusal on four term kinds is main behaviour, not new. Ids: FR-194-AC-9 and AC-10 appear on no other open PR head; AC-4 and AC-7 edits are shared with no other head.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

Mergeable: no defect that blocks the PR.

Conflicts with main: none. Clean merge-tree with open #1340 (FR-194) and #1373 (ADR-024). Merge #1334 before #1340: #1340's AC-7 vectors write aggregate atoms without `min_count`.
