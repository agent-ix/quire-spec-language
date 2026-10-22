---
id: SR-15950
title: "spec review of STD-289 refinement property form (QSL PR #1259)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@a11192291930168bf43025ec9ffbecc5c776c6fa; spec/functional/FR-144-request-and-settle-a-refinement-item.md, spec/test-cases/TC-1167-a-refinement-record-carries-the-shared-refinement-property-form.md"
review_set: subset
evaluated_revision: "a11192291930168bf43025ec9ffbecc5c776c6fa"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-144
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1167
    type: reviews
---
# SR-15950: spec review of STD-289 refinement property form (QSL PR #1259)

## Summary

Ticket: STD-289 (unblocks QSL-544 FR-144-AC-1). PR quire-spec-language#1259, head a11192291 (matches `git ls-remote`), reviewed as a merge with origin/main 39823fe73 (24912ced2). Two files, spec Markdown only. FR-144 names `quire_semantic_value::fairness::PropertyForm::Refinement { liveness }` as a fifth QSV FR-121 variant, spelled `{"refinement":{"liveness":<bool>}}` and ordered after `Liveness`, keeps `RefinementFailure` in QSL, and adds FR-144-AC-8 with TC-1167.

These claims hold:

- The bytes are right: RFC 8785 of a one-member object holding a one-member object with a boolean is exactly `{"refinement":{"liveness":true}}` and `{"refinement":{"liveness":false}}`. The order `Liveness < Refinement{false} < Refinement{true}` is the derived `Ord` of a fifth, struct-like variant listed last, which QSV FR-121 Behavior 3 ("enum variants order as listed") gives.
- `liveness` is true exactly when `F_A` is non-empty, and `CasRefinesCounter`'s `ensure` row is its abstract fairness: FR-144-AC-7 settles it `Certified` (safety half only) with the row removed, and TC-549 already reads `Refinement{liveness: true}` with it. So both AC-8 vectors are grounded.
- FR-123 is correctly left unedited: its four-way classification is the classification of a temporal clause (TP-1 to TP-4), and a refinement record is FR-144's TP-5.
- STD-326 (Backlog) exists for the QSV change. The PR body lists the QSV edits it needs. STD-326 must also drop Behavior 1's "corresponding to QSL FR-123's classifications", since `Refinement` is FR-144's and not an FR-123 classification.
- Ids: TC-1167 and FR-144-AC-8 appear on no other open PR head. #1241 also edits FR-144; a trial merge of both is clean. No status wording or compat layer. The gate on the merge exits 0 with no diagnostic on a touched file.

The `code-review` method was skipped, because the diff contains no code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | minor: FR-144 now states a fifth variant of QSV FR-121's `PropertyForm`, but QSV FR-121 Behavior 1 requires "exactly" four, and FR-144 records no dependency on QSV FR-121. It has no `ix://agent-ix/quire-semantic-value/FR-121` relationship (FR-123 has one), no Dependencies entry and no reference to STD-326. A reader of FR-144 cannot see that AC-8 waits on another repository. Fix: add the `depends_on` edge, a Dependencies line for QSV FR-121, and a References line naming Linear STD-326. | spec/functional/FR-144-request-and-settle-a-refinement-item.md:1-27,62-70,152-163 |
| FND-002 | low | minor: AC-8's last clause, that no `quire_semantic_value` item names `RefinementFailure`, is verified by "Inspect the public items" (TC-1167 step 3) under a `Test` method. A QSL test cannot enumerate another crate's public items, so the clause never fails in a test run. Fix: move it out of the Test AC (it is an Inspection of QSV), or drop it, since FR-144 already places `RefinementFailure` in `qsl-replay`. | spec/functional/FR-144-request-and-settle-a-refinement-item.md:149; spec/test-cases/TC-1167-a-refinement-record-carries-the-shared-refinement-property-form.md:27-28,38 |

## Verdict

Mergeable after FND-001 (frontmatter and Dependencies lines). FND-002 is minor. FR-144-AC-8 cannot pass until STD-326 lands in QSV and QSL's pin moves, which QSL-544 owns. The bytes, the order, the `liveness` grounding, FR-123 being left unedited, the trial merge with #1241, id uniqueness and the gate are verified. Owner or planner decisions: (1) merge order. Either the QSV FR-121 amendment (STD-326) merges first, or this QSL spec merges ahead with FND-001's dependency recorded. (2) Acknowledge the three lead defaults: the `{"refinement":{"liveness":<bool>}}` spelling as the only non-string `PropertyForm`, the order after `Liveness`, and `RefinementFailure` staying in QSL against the QSL-918 planner note.
