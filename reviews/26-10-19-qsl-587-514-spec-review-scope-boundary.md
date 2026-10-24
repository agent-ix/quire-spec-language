---
id: SR-13064
title: "Scope-boundary review of PR #1020: is FR-307-AC-6 QSL export behaviour or driver behaviour"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language@66ef5a3e3; FR-307 Behavior (Caller and position, Per-item results), FR-307-AC-6, FR-307 Dependencies, ADR-017 AR-4 Implementation export, TC-1082; context: spec/reviews/qsl-382-spec-review.md FND-012/FND-013"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-307
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: reviews
---

## Summary

Tickets: QSL-587, QSL-514. PR quire-spec-language#1020. The earlier AC-6
(deleted by QSL-382 FND-012) asserted what the driver's QSpec FR-331 request
holds and pointed at quire-integration. The new AC-6 asserts only what the
layer-4 export returns: one refusal per item with no bindings, and a
successful export with no bound item. Both are observable at the QSL export
API and are owned by this repo. TC-1082 tests the export result itself; it
does no driver-side filtering (the QSL-382 FND-013 defect). The Dependencies
addition and the ADR-017 AR-4 pointer name the driver as the consumer and say
FR-307 owns the partition, not the driver's request building. No boundary
defect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean: FR-307-AC-6 describes QSL-owned export behaviour.
