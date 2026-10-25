---
id: SR-17206
title: "QSL-891 NoInstance integrity review"
type: SpecReview
analysis: integrity
scope: "FR-169 NoInstance behavior and AC-16, TC-594 step 6, state-graph outcome and settlement map"
review_set: subset
review_date: "2026-10-25"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: references
---

## Summary

The requirement, implementation marker, settlement row, and test agree on the NoInstance result. The change adds no alternative source of truth for the outcome.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Integrity analysis

The new enum arm is handled exhaustively by `settle_state_graph`. Its row is `Inconclusive(NoInstance)`, matching FR-169's V-6 row and the FR-127 cause. The state-graph run carries one explicitly documented item-level marker only where instancing is empty; regular instances and no-initial-state results remain separate. The tagged test checks the actual return and settlement paths.

## Verdict

PASS for the scoped NoInstance state and verdict mapping.
