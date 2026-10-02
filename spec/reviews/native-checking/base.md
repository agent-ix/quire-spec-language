---
id: SR-066
title: "Native model and checker base review"
type: SpecReview
analysis: base
scope: "FR-015/016, FR-006 judgments, docs/native-model-checking.md, IT-005, TC-025–029/040–053 and TM-003"
review_set: all
evaluated_revision: "ceccabb564b61742597ad356eb1019ab0c8d1544"
review_date: "2026-09-08"
---

## Summary

The specified model and checker interfaces are ready for implementation. All 15 new criteria and the five inherited static-judgment criteria have planned cases; none is reported as executed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Resolved: initial matrix IDs TC-40–52 were not the actual TC-040–052 identities; commit 8957c8c corrected them. | TM-003; TC-040–052 |
| FND-002 | low | Resolved at the evaluated revision: TC-025–029 now name both owning criterion sets, and historical linker coverage is explicitly dated to PR8. | TM-003; FR-006; FR-016 |

## Checklist and coverage

Read FR-015/016, their concrete API document, IT-005, TC-025–029 and TC-040–053, and the affected TM-003 rows. FRs implement US-002 and trace to StR-001; NFR-005 applies to production and qualification. Full IDs are unique in this addition and local document targets resolve. Earlier requirement and source-bridge acceptance remains separate.

The six coverage rules are addressed: each criterion has a case; scalar/role/context combinations have positive and adverse controls; zero/exact/one-over bounds and invalid coordinates are explicit; missing/ambiguous/stale/foreign/type/undefined/unsupported/resource failures remain distinguishable; pre/post and lexical scope transitions are paired; unused declarations, contradictory guards, common-fact joins, repeated aliases and success after refusal cover edge cases. Runtime population transitions belong to FR-007, not static checker acceptance.

There is one selected model profile with explicit roles, not configurable fallback semantics. Inputs, immutable outputs, refusal codes, budgets and assumptions are concrete in docs/native-model-checking.md. TM-003 contains 15 existing executed and 19 planned cases. No test-status advancement is authorized by this review.

## Verdict and provenance

PASS for implementation of this specified scope. Agent A applied the actual
QUOIN base and all seven analysis skills serially, following the owner's
selected all review set. No subagents or builds were started for this review.
No applicable required AssuranceProfile was found. Catalog schema and existing
public interfaces were inspected. Implementation/test completion is not claimed.
The owner-declined optional gap-analysis semantic comparison remains excluded.

