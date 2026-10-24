---
id: SR-16322
title: "Spec review of quire-spec-language PR #1392: FR-393-AC-5 and TC-1078"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@9809b06b8cbeb1a8a02025b9a73ac02c323e1bc9; spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md FR-393-AC-5; spec/test-cases/TC-1078-s2-builds-time-declaration-quantity-and-timed-interval-forms.md step 5; FR-447 interval rule as context"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-393
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1078
    type: reviews
---
# Spec review of quire-spec-language PR #1392

## Summary

Ticket: QSL-1017. Reviewed the revised FR-393-AC-5 and TC-1078 step 5
against FR-447's `[q, *]` admission rule. The change removes the direct
contradiction between FR-393 and FR-447. Both edited files passed targeted
`quire validate --scope .`. No applicable required AssuranceProfile was found.
The EARS lens does not apply because the diff changes an AC and test case,
not an FR Description, Behavior or Constraint statement.

## Verdict

**CONDITIONAL**: the test case no longer covers the positive sentence that
the amended criterion adds.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-393-AC-5 now requires `[q, *]` to parse, while TC-1078 step 5 and its expected results mention only the two rejection cases. The criterion's named verification case cannot establish its new positive assertion. Add a positive fixture and expected S1/S2 result to TC-1078, or move that assertion entirely to FR-447/TC-1220. | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:171; spec/test-cases/TC-1078-s2-builds-time-declaration-quantity-and-timed-interval-forms.md:25,37 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | TC-1078 step 5 now includes `always [2 s, *] holds(p)` and an expected S2 timed interval with an open upper end. |
