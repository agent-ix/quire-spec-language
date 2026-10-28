---
id: SR-17186
title: "QSL-1072 PR #1523 gap analysis: matrix rows of FR-487 and FR-315-AC-43"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@<sha removed>; PR #1523 (branch qsl-1072-lowering-spec); spec/functional/FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md, spec/functional/FR-315-encode-the-smt-lib-transition-relation.md, spec/test-cases/TC-1264-hyper-atoms-and-match-conjuncts-are-lowered-and-evaluated.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-487
    type: references
---

## Summary

Ticket: QSL-1072. PR: quire-spec-language#1523. Planless run; no plan was supplied. The diff is spec-only (three spec files, no source or test change), so no reverse-gap or stub check applies to it.

`quire matrix --scope . --format json` at the head gives `status: untagged` for each of the nine new criteria: FR-487-AC-1 to FR-487-AC-8 and FR-315-AC-43. The criteria the new text builds on are also untagged on main: FR-172-AC-5 (label constants), FR-315-AC-7 and FR-315-AC-10, the last two owned by QSL-1010. That is expected for a spec PR. The tests are owed by the QSL-1072 coding PR, tagged `#[trace("TC-1264", "FR-487-AC-n")]` and `#[trace("TC-1264", "FR-315-AC-43")]` as TC-1264 prescribes.

## Verdict

FAIL by the mechanical rule, because nine criteria are untagged. This is not a merge blocker for a spec-only PR, and the same rows must read `tagged` before the QSL-1072 code PR merges.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-487-AC-1 to FR-487-AC-8 and FR-315-AC-43 have no tagged test (matrix `untagged`). FR-172-AC-5, which FR-487-AC-2 depends on, is also untagged on main. All are owed by the QSL-1072 code PR. FR-172-AC-5 is owed unless it is moved to its own ticket (SR-17180 FND-004). | FR-487-AC-1, FR-315-AC-43, FR-172-AC-5 |

## Coverage

- Criteria in scope: 9; tagged: 0; untagged: 9.
- Reverse gap: not applicable (no code in the diff).
- Stubs and coverage inflation: not applicable.
- Semantic review: skipped (spec-only diff, nothing to compare).
- Plan completion: not assessed

## Dispositions

Round 1, head <sha removed>. `quire matrix --scope . --format json` at the head gives `untagged` for FR-487-AC-1 to FR-487-AC-12, FR-315-AC-43, FR-172-AC-7 and FR-172-AC-5, 15 rows in all.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Spec-only PR. The tests are owed by the QSL-1072 code PR and must read `tagged` before it merges. The set grew with FR-487-AC-9 to AC-12 and FR-172-AC-7 (TC-1264 steps 10 to 13 and 12), and still includes FR-172-AC-5 (TC-597), which QSL-1072 now delivers. |
