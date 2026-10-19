---
id: SR-13268
title: "Spec review of PR #1038: integrity analysis"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@651a86903; git diff origin/main...HEAD (PR #1038)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-540
    type: reviews
---

## Summary

Ticket: STD-206. Consistency and completeness across the changed artifacts at 651a86903. Clean: the ADR-020 section 8 lost-update and divergence walkthroughs still hold with the guard (each commit precondition implies its guard, so the 28/36 reachable-state counts and the position-4 Postcondition failure are unchanged). FR-139-AC-6's bound [0, 4] matches its new bullet. TC-544 steps 6 and 7 match FR-139-AC-6 and AC-7.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-135 complete-value typing bullet | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-135-AC-6 | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| TC-540 step 6 | spec/test-cases/TC-540-s3-checks-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-140 Outputs EvalFailure and MappingRowRef prose | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140 refused/incomplete/OutOfType/Stopped bullet | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| ADR-020 section 8 guard rationale | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md | examined |
| FR-139-AC-6 | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | examined |
| TC-544 steps 6-7 | spec/test-cases/TC-544-population-valued-expressions-type-and-evaluate-inside-a-refinement.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | New refusal 'a unit that selects no complete-value profile refuses a refinement with unknown_profile/wrong-selection-role at the alias' is not in QSpec FR-375, and 'the alias' is ambiguous. QSpec FR-375 refuses unknown_profile/wrong-selection-role only for 'a profile alias that does not select quire.temporal.infinite-trace/v1'. The refinement's only alias (`using inf`) selects its role correctly, so this cause at that alias misreports the defect, and a unit with no complete-value selection has no complete-value alias to point at. Say which span is used and add the rule to the QSpec asks, or drop it. TC-540 step 6's 'selecting only the infinite-trace profile' unit also cannot hold RingIsQueue's Q/R contracts, which need `using v`. Say the variant has no contract clauses. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:134-142 |
| FND-002 | low | One bullet carries three triggers (refused/incomplete, out of type, stopped), and its OutOfType case contradicts the Outputs comment's scope. The Outputs block says OutOfType is 'the value leaves the declared type of its field', while the bullet says only a history update can hit it. Split the bullet into one per EvalFailure arm and state OutOfType as history-update-only in both places. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:142-149 |

## Verdict

Findings: 1 medium, 1 low. The base SR's FND-001 to FND-003 are also cross-artifact contradictions and are not repeated here.

## Dispositions

Round 1, head 72d04b1dc (fix commits d561b72e7, 72d04b1dc).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |

Disposition pass 1 (head 72d04b1dc): both fixed. No new findings.
