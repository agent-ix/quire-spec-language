---
id: SR-314
title: "Scope boundaries of exact protocol numbers"
type: SpecReview
analysis: scope-boundary
scope: "FR-038; TC-117; public numeric API and indexing"
review_set: all
---
## Summary

PASS. FR-038 owns exact numeric admission; README and TC-117 preserve the separate
source/model, enclosing artifact and protocol-consumer responsibilities.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No responsibility overlap or false whole-artifact claim found. | FR-038; TC-117; src/protocol_artifact/mod.rs |

| Responsibility | Owner/class | Dependency boundary |
| --- | --- | --- |
| FR-038 exact kind/components and refusal | protocol_artifact numeric API; core | Serde/object-only adapter behavior guaranteed locally by TC-117 controls; no consumer integration inferred. |
| Source normalization and model bounds | Existing frontend/model authorities | Assumed upstream under standard FR-039/044; wire 2/4 still refuses. |
| Canonical artifact, budgets and consumer admission | Enclosing compiler #40 and B | Assumed separate contracts. B FR-001 permits exact string encoding; its safe-number allowance does not require bare integers. |
