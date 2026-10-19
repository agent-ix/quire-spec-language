---
id: SR-13325
title: "Spec review of PR #1050: EARS conformance"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@c5407fabd; git diff origin/main...HEAD (PR #1050)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-146
    type: reviews
---

## Summary

Ticket: STD-212. EARS grammar of the new and edited SHALL statements. The FR-136 order bullet, the hypotheses bullet, the obligation-identity bullet and the rejected-certificate bullet each carry one SHALL with a named subject, and are clean.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-136 Order of the checked rows bullet | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-146 hypotheses bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Row order bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Occurrence key bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Obligation identity bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 rejected-certificate bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 settlement bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The occurrence-key bullet holds four SHALLs (node id and role, the `Initial` ordinal, the `Step` ordinal, the source-map regions). Split it into one statement for the key, with both ordinals, and one for the source map, so each can be cited and tested alone. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:132-139 |
| FND-002 | low | The row-order bullet holds a definition SHALL and an invariance SHALL (reordering changes no `StepRowRef`, occurrence key or obligation identity) that is about three things. Keep the definition here, and put the reordering invariance with the occurrence key and obligation identity it constrains. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:125-131 |

## Verdict

Findings: 2 low.
