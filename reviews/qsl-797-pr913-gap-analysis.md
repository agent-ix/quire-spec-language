---
id: SR-11757
title: "Gap analysis of quire-spec-language PR #913: FR-116-AC-7 replay and FR-115-AC-9/AC-10 tags"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@749b893a54c358601d912ab99aae6601654feeac; qsl-replay/src/spine/clause/tests/link_replay.rs, qsl-semantics/tests/it/relationship_snapshots.rs, spec/functional/FR-116-replay-a-frame-counterexample.md, spec/functional/FR-115-run-an-operation-frame-over-an-invocation.md, spec/test-cases/TC-1037-a-frame-run-reports-a-changed-relationship-link-as-a-link-witness.md"
review_set: subset
---
# Gap analysis of quire-spec-language PR #913

## Summary

Ticket: QSL-797 (closing slice). Planless gap analysis over the PR diff.
Plan completion: not assessed.

Examined: FR-116-AC-7, TC-1037 row 15, FR-115-AC-9, FR-115-AC-10, the
FR-116 Description sentence, and the six test-to-AC bindings the diff adds.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS, no major findings.

- FR-116-AC-7 / TC-1037 row 15: all three clauses are asserted, one test each
  (removed claim reproduces holding claim and replay `Link`; added claim
  reproduces holding both; housing-modifying frame settles `inconclusive`,
  `Verdicts`). Bindings correct.
- FR-115-AC-9: housing-before-composition (row 6), lower source first whatever
  the difference (row 7), lower target within one source (row 2, `p1`-`t1`
  before `p1`-`t2`), step 1-3 finding first (row 8, `Created`), step 4
  `population_delta_mismatch`/`delta-disagreement` (row 9), and equal results
  twice (row 12). The report-level tail (exit 20, report equality) is asserted
  on the `FrameVerdict`; verdict-to-report routing is covered by existing
  generic run tests (frame.rs exit 0 and exit 20 cases). Bindings correct.
- FR-115-AC-10: deleted-object links (row 10), one-snapshot and incomplete
  relationship (row 11) in the row 10/11 test, and the relationship-the-frame-
  modifies clause (row 1, `Holds`) in the rows 1-3 test, which justifies its
  added AC-10 tag. Bindings correct.
- FR-116 Description now lists retype and link add/remove, matching the
  `FrameChange` variants (`FieldWrite`, `Created`, `Deleted`, `Retyped`,
  `Link`). Correct.
