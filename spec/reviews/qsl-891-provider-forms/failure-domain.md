---
id: SR-17209
title: "Failure-domain review of QSL state-graph provider form negotiation"
type: SpecReview
analysis: failure-domain
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

# Failure-domain review of QSL state-graph provider form negotiation

## Summary

Reviewed absent, partial and malformed provider declarations; named-backend
mismatch; item kind isolation; and descriptor identity conflicts.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found. Missing declarations yield no state-graph candidates; malformed declarations refuse registration; an unadvertised form yields an empty candidate set; unrelated item kinds do not read the form set. | FR-075-AC-21/22, TC-193 |

## Verdict

PASS. The failure outcomes are separated at registration and candidate
computation boundaries.
