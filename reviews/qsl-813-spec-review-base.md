---
id: SR-12037
title: "QSL-813 PR #919 spec review: FR-365 multi-valued reaches destination"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@607a2285856cf5330fee3a2b5c91df459d39f9b3; spec/functional/FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md, spec/test-cases/TC-1046-reaches-over-a-relationship-end-types-boolean-and-refuses-by-name-operand-direction-and-destination.md, spec/test-cases/TC-1047-a-checked-reaches-over-a-relationship-end-lowers-to-quire-op-model-reaches.md"
review_set: subset
---

## Summary

Ticket: QSL-813. PR quire-spec-language#919. FR-365 "Checking" steps 4 and 5, the order paragraph, FR-365-AC-1 and FR-365-AC-3, TC-1046 (fixture rows `Order/litter`, `Order/manyOwners`; vectors C10, C17, C18) and TC-1047 row 8 were read against QSpec FR-322-AC-57 and the QSpec relationship navigation cases RN-18 to RN-27 (QSpec main, read-only).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS. Step 5 admits a finite multi-valued destination end of the near end's own type, unique or not, and refuses a foreign destination single- or multi-valued; this matches FR-322-AC-57 and RN-18, RN-20, RN-21, RN-22, RN-25. The stated order (operands, direction, multiplicity, type) matches FR-322-AC-57's order (receiver endpoint for both operands, direction, destination multiplicity, type), and the two stated refusal-winner consequences agree with RN-22/RN-26/RN-27 and with C12, C16 and C18. FR-365 "Evaluation" already handles several neighbours and repeats, so admitting multi-valued ends needs no evaluation change. FR-363 navigation is unchanged. AC-1 and AC-3 are each backed by TC-1046 vectors (C17, C10, C18), and TC-1047 row 8 backs AC-5 for the multi-valued case.
