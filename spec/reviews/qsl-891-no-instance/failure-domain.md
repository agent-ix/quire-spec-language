---
id: SR-17205
title: "QSL-891 NoInstance failure-domain review"
type: SpecReview
analysis: failure-domain
scope: "FR-169-AC-16, TC-594 step 6, qsl-analyze/src/model_check/state_graph.rs, qsl-replay/src/model_check/graph_settle.rs"
review_set: subset
review_date: "2026-10-25"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: references
---

## Summary

The NoInstance path is distinct from NoInitialState and contributes no predicate labels or evidence. Shared exploration remains available for other instantiated items and requested deadlock freedom.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Failure-domain analysis

The run returns an item-level marker when the item has no instances. If all requested claims have no instances and no deadlock item is requested, it returns before sampling or graph exploration. In a mixed request, empty-instancing items retain the marker while instantiated items proceed; a no-initial-state subject still yields NoInitialState for actual instances. A requested deadlock-freedom decision continues through the shared graph path. Settlement maps the marker directly and does not invoke evidence replay or certificate checking.

## Verdict

PASS for the failure cases in FR-169-AC-16 and its shared-exploration boundary.
