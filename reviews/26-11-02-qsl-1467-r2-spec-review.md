---
id: SR-2026110215393
title: "Spec review round 2 — QSL-1467: FR-220-AC-16 and TC-665 step 15"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; PR #2118 fix round 0b1d9029b..08b58f4f9; spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md; spec/test-cases/TC-665-access-orderings-and-classification.md"
review_set: subset
---

## Summary

Ticket: QSL-1467. PR: quire-spec-language#2118, head 08b58f4f9. Method: spec-review (integrity and EARS lenses) over the spec edit the fix round added. The edit adds a new SHALL bullet in FR-220's pre-check list, FR-220-AC-16, TC-665 step 15 with its expected result, and widens TC-665's scope to AC-16 and steps 10 to 15. `make check-spec-validation` exits 0 at 08b58f4f9, with no diagnostic on FR-220 or TC-665.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-220 new SHALL: EN-1 instantiation is an FR-120 expansion under `max_candidates` and the meter, and a stop on either budget SHALL settle FR-126's `Stopped{ResourceExhausted, limit: Some(..)}` before any state is explored | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:177-182 | examined |
| FR-220-AC-16 | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:284 | examined |
| TC-665 scope line and step 15, with its expected result | spec/test-cases/TC-665-access-orderings-and-classification.md:15, 55-57, 81 | examined |
| FR-126 expansion-stop SHALL | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:306-313 | context_only |
| FR-120 "Post-states" `max_candidates` stop | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:592-643 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean.

- The SHALL bullet is EARS unwanted-behaviour shaped ("If it stops on either budget, the item SHALL settle ..."). It names its subject, and it defers the limit-value rule to FR-126 instead of restating it. It is consistent with FR-126 lines 306-313: the same `Stopped{cause: ResourceExhausted, limit: Some({MaxCandidates | EvaluationMeter { counter }, value})}` shape. It is also consistent with FR-120 "Post-states", which makes the `max_candidates` count stop the expansion with `resource_exhausted`/`insufficient-next-charge`.
- AC-16 states a testable outcome with concrete inputs. The inputs are `SB` under `tso` and `ra`, `max_candidates` 0, 6 and 12, and ten absent `Cell` keys. The outcomes are exact: `Stopped{ResourceExhausted, {MaxCandidates, n}}`, no product state, and an `incomplete`/`limit-reached` record naming `expansion.max_candidates`. It has a negative control: the variant without the absent keys builds at 6, and the absent-key variant builds at 12. A mutation can fail it (gap-analysis SR-2026110215392 checked this).
- Numbering is correct. AC-16 follows AC-15, step 15 follows step 14, and the scope line now reads AC-1 to AC-16 and steps 10 to 15. The Verification cell `Test (EN-1 pre-check; TC-665 step 15)` follows the convention of AC-11 to AC-15.
- AC-16 tests only the `MaxCandidates` limit, while the SHALL bullet also covers the evaluation-meter stop. The meter arm is the shared `expansion_stopped` function, which FR-126-AC-19 (TC-521) already tests at the explorer. A meter-specific AC here would restate it, so this is no gap.
