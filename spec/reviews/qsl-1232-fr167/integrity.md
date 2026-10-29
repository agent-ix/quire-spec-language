---
id: SR-117435
title: "QSL-1232 integrity analysis"
type: SpecReview
analysis: integrity
scope: "FR-167/168/169, ADR-022 and TC-592/593/594"
review_set: all
---

## Summary

The result vocabulary, exploration rules, settlement map and test procedures now agree on when a reduced proof is returned and how it is labeled. Independent review found an ambiguity about whether SG-1 witness paths survive in `HoldsReduced`; the spec now retains those concrete paths and requires FR-170 replay before settlement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Resolved: SG-1 `HoldsReduced` carries one path per initial state; SG-2 carries none, and FR-169 defines replay and failure settlement for carried paths. | FR-168; FR-169; ADR-022; TC-593; TC-594 |

## Integrity checks

- `Holds` carries a certificate and is reserved for exhaustive runs; `HoldsReduced` carries applied reductions and no certificate.
- SG-1's reduced `Possible` carries concrete witness paths and replays them; SG-2's reduced `AlwaysPossible` carries no witness paths.
- POR counts in FR-168-AC-14 make the ample-set reduction observable and non-vacuous.
- FR-169-AC-17 and TC-594 distinguish replayed SG-1 witnesses from SG-2's checker-free, path-free reduced proof.
