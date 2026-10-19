---
id: SR-13269
title: "Spec review of PR #1038: EARS conformance"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@651a86903; git diff origin/main...HEAD (PR #1038)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-139
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
---

## Summary

Ticket: STD-206. EARS conformance of the new requirement statements at 651a86903. Clean: FR-139's sum and only-not-one bullets, FR-135's two new bullets (modal SHALL, single response), FR-140's nested-lifting, optional-field (Where ... SHALL) and relationship bullets.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-140 UnmappedType reference bullet | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140 UnmappedType object bullet | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140 nested reference lifting bullet | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140 optional field bullet | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140 relationship extent bullet | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-139 sum bound bullet | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | examined |
| FR-139 only-not-one bullet | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | examined |
| FR-135 complete-value typing bullet | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-135 Int-Integer equality bullet | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The two UnmappedType bullets use lowercase 'shall' ('then map_state shall make', 'shall return', 'shall not drop'), unlike the rest of FR-140; the reference bullet names no MappingRowRef. Use SHALL. In the reference case, say the failure names the row being evaluated (field row, argument or history update) and the object. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:122-124 |

## Verdict

Findings: 1 low.
