---
id: SR-117435
title: "QSL-1232 integrity analysis"
type: SpecReview
analysis: integrity
scope: "FR-167/168/169, ADR-022 and TC-592/593/594"
review_set: all
---

## Summary

The result vocabulary, exploration rules, settlement map and test procedures now agree on when a reduced proof is returned and how it is labeled. A draft inconsistency between the POR test procedure and acceptance criterion was found and corrected before this review artifact was finalized.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved contradiction, missing condition or non-atomic outcome rule found in the reviewed change. | FR-167; FR-168; FR-169; ADR-022; TC-592; TC-593; TC-594 |

## Integrity checks

- `Holds` carries a certificate and is reserved for exhaustive runs; `HoldsReduced` carries applied reductions and no certificate.
- The same split is used by ADR-022's GV-1/GV-4 rules and FR-169's V-1 settlement rows.
- FR-168-AC-14 and TC-593 now agree that both symmetry claims and POR `Possible` produce `HoldsReduced`.
- FR-169-AC-17 and TC-594 agree that reduced proofs settle `Proved{Reduced, Uncertified}` without certificate checking or replay.
