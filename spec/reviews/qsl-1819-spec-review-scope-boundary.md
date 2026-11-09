---
id: SR-20261109172912000004
title: "Scope-boundary review of the L5 engine-gap spec batch"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language; QSL-1819 (with QSL-1820); branch spec/l5-engine-gaps @ 9514a0e41 vs origin/main db995d817; ADR-029; FR-195; FR-201; FR-280; FR-281; FR-282; FR-286; FR-564; FR-565; FR-567; TC-761; TC-762; TC-763; TC-770; TC-1391; TC-1393; TC-1394"
review_set: subset
---

## Summary

Ticket: QSL-1819 (with QSL-1820). Allocation of responsibilities between FR-280 (provider entry), FR-281 (analyze), FR-201/FR-282 (checkers), FR-286 (serialization) and FR-564/565 (matrix) in this batch.

## Verdict

Mergeable on boundary grounds. The allocation is sound: analyze owns request construction and the provision; provider::run delegates to analyze; serialization of the counterexample stays in FR-286; the checker's subject acquisition is in FR-201. One low overlap.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-281 now holds matrix and provider-entry content that belongs to FR-564 and FR-280. The 'Engine ownership of the state-budget cell' section assigns FR-564 cells, and its 'The matrix's unsupported cell is EN-1's RequiresBound refusal' is ambiguous, because the analyze unsupported cell is FR-281-AC-3's item and only the prove cell is AC-11's. AC-11's new tail tests provider::run, which FR-280-AC-9 already covers. Move the cell assignment to FR-564, say 'the prove unsupported cell', and drop AC-11's provider::run clause in favour of FR-280-AC-9. | FR-281:187-196, 526 |

## Coverage

- FR-280 (examined)
- FR-281 (examined)
- FR-282 (examined)
- FR-286 (examined)
- FR-564 (examined)
- FR-281-AC-11 (examined)
