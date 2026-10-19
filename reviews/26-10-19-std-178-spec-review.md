---
id: SR-12899
title: "Base review of PR #992: FR-310 S2 refinement form aligned to QSpec FR-375 and FR-432"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@91bf1ee1d; FR-310 Inputs, Outputs, Behavior, AC-1 to AC-4; TC-889 scope, steps 1 to 4 and Expected Results"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-310
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-889
    type: reviews
---

## Summary

Ticket: STD-178. PR quire-spec-language#992 (head 91bf1ee1d, diff
`origin/main...HEAD`, base 2e6c836). `History.initial` becomes
`HistoryLiteralForm`, `Internal` and `Visible` are removed, `Step` gains
`StepSourceForm` and `StepTargetForm::Node`, the fairness kind is built as
written, AC-4 is new and AC-2 is rewritten; TC-889 gains step 3 (old step 3
is step 4). This base review covers ID format, AC to TC coverage and link
integrity. `quire validate` over both changed files exits 0. No id collides
with open PRs #990 (FR-392, TC-1077) or #991 (FR-393, TC-1078); this PR
allocates only FR-310-AC-4.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The rule "build each `assume` and `ensure` row's kind ... and granularity as written, with no granularity left absent for S3 to read as `whole`" can be read two ways: S2 keeps a missing granularity absent, or S2 fills it in so that none is left absent. AC-4 ("granularity absent") takes the first reading. The rule should say it directly: an unwritten granularity is built as absent, and S3 reads it as `Whole` (FR-137). | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md:100-102,116 |

## Verdict

The base checklist finds no defect beyond FND-001. AC-4 and TC-889 step 3
trace one to one, and the step renumbering is consistent. The PR is not
mergeable, because of the two highs in the integrity and scope-boundary
reviews (SR-12900 FND-001, SR-12901 FND-001).

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | aea8326f7 rewrites the rule: "an unwritten granularity is built as absent, and S3 reads absent as `whole` (FR-137)". It now agrees with AC-4. |
