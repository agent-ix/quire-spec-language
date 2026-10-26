---
id: SR-17208
title: "Base review of QSL state-graph provider form negotiation"
type: SpecReview
analysis: base
scope: "QSL FR-075-AC-21/22, FR-167-AC-4, ADR-014 §6, ADR-022 QS-7, TC-193, TC-592"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-167
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-193
    type: references
---

# Base review of QSL state-graph provider form negotiation

## Summary

Reviewed the QSL descriptor, request item and candidate rules against QSpec
FR-331/FR-390 and ADR-022 SG-4/QS-7. No applicable AssuranceProfile was found
in the QSL specification tree. The additions are grounded in those sources;
each new acceptance criterion has a linked focused test case.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No blocking issues found. The form set and request form are explicit, exact membership controls state-graph candidacy, and descriptor changes participate in registration equality. | FR-075-AC-21/22, FR-167-AC-4, TC-193, TC-592 |

## Verdict

PASS. Form omission, malformed declarations, exact matching and cross-kind
isolation have observable outcomes.
