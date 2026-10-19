---
id: SR-13271
title: "Spec review of PR #1038: scope-boundary analysis"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language@651a86903; git diff origin/main...HEAD (PR #1038)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: reviews
---

## Summary

Ticket: STD-206. Allocation of the new rules to requirements and of the new ACs to tickets at 651a86903. The author's ownership split is in the STD-206 Linear comment (the spec carries no ownership). It was checked against QSL-543 ('Implements: FR-135, FR-136, FR-137', Done), QSL-544 ('Implements: FR-140, FR-141, FR-142, FR-144 (safety), FR-145 ... nothing beyond them'), QSL-545 (FR-138) and QSL-546 ('Implements: FR-139, FR-140 (AC-2/3)'). Clean: FR-139-AC-6/7 (TC-544) to QSL-546, FR-140-AC-7 and AC-8 (CasRefinesCounter) to QSL-544.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| Linear STD-206 ownership comment | Linear STD-206 comment 06e48fbd | examined |
| FR-135 complete-value typing bullet | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-135 Description scope sentence | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | context_only |
| FR-140-AC-8 | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140-AC-6 | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140-AC-9 | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140-AC-10 | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140-AC-11 | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-135-AC-6 | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-141-AC-7 | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The comment gives FR-140-AC-6, 9, 10, 11 and FR-135-AC-6 to QSL-544, but they need the RingIsQueue fixture and FR-139 forms (the reason AC-5 moved to QSL-546), and FR-135 belongs to QSL-543 (Done); the sentence also lists FR-139-AC-6/7 under both tickets. AC-6, AC-10 and AC-11 are RingIsQueue variants, and AC-9's `only` is FR-139's. QSL-544 does not wait on QSL-546, so by the comment's own rule these go to QSL-546 (or QSL-544 gains a blocker edge on QSL-546). FR-135-AC-6 is S3 checking, outside QSL-544's 'nothing beyond' list, and its owner QSL-543 is Done, so it needs a ticket. Also: AC-11's optional-field rule reverses PR #1032's fix round 1 (which refuses optional fields), so QSL-544 needs that change re-planned. | Linear STD-206 comment 06e48fbd:- |
| FND-002 | medium | FR-135's Description says 'Step rows are FR-136, ... history rows FR-138', yet the new bullet rules on step-row arguments and history updates. Move the step-argument and history-update typing to FR-136 and FR-138 (each with its AC), and keep FR-135's bullet to field rows. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:134-142 |
| FND-003 | low | The ModelSystem::over_model refusal is FR-120's constructor contract (FR-120 owns ModelSystem), but FR-140 states it and AC-8 tests it; map_state is never called in AC-8's first half. State the over_model refusal in FR-120 with its own AC, and keep FR-140-AC-8 to what map_state does with two valid systems. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:171 |

## Verdict

Findings: 2 medium, 1 low.
