---
id: SR-13326
title: "Spec review of PR #1050: failure-domain analysis"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@c5407fabd; git diff origin/main...HEAD (PR #1050)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-146
    type: reviews
---

## Summary

Ticket: STD-212. Identity confusion, unstated cases and edge topology in the new ordering and record-identity text.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-136 Order of the checked rows bullet | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-146 Occurrence key bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Row order bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 hypotheses bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| QSpec FR-375 Checked-package node sort sentence | quire-specification:spec/functional/analysis/FR-375-declare-model-refinement.md | context_only |
| ADR-013 O-09 | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The role is defined as 'the role of the refinement record's own occurrence key (FR-144)', but FR-144 names no role, and ordinals 1..n+1 reuse the refinement node's own source-occurrence space. FR-322's `source_map` is the 'semantic-occurrence to original source-region mapping', with role one of `declaration`, `type`, `expression`, `anchor`, `claim` or `generated`. O-09 treats a declaration's occurrence as 'that node's one `declaration` occurrence'. Under this text the `Initial` key maps to the same regions as ordinal 0, so a source-map reader sees the declaration occur twice or more, and a real second occurrence of the node would collide with `Initial`. Name the role. FR-322 already has `generated`, which fits a derived record better than reusing the declaration's role. Either way it is a QSpec FR-322 ask. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:132-139 |
| FND-002 | medium | Over a protocol subject the order is undefined. A `MemoryStep(MemoryStepKind)` row names no declaration, so 'first named declaration' gives it no position, and `duplicate`/`lose` (or `spawn`/`retire`) rows share one declaration and need a member-name tie-break that a step kind does not have. FR-136's `StepClass` has these arms, and QSpec FR-375's sort sentence is equally silent (FR-432 owns protocol rows), so `StepRowRef` is undefined for those refinements. Raise a QSpec ask (FR-375/FR-432) and state the protocol-row order in FR-136, or limit the bullet to model subjects until QSpec answers. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:77-81 |

## Verdict

Findings: 2 medium. The hypotheses rule is clean on the shared-type edge: 'abstract model only' keeps an invariant whose type both models admit.

## Dispositions

Round 1, head 7cfb5530b (fix commit 7cfb5530b over merge 2b96e20e4).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
