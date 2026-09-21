---
id: FR-056
title: "Register compensation only after a successful effect"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-006
    type: implements
  - target: ix://agent-ix/quire-specification/FR-051
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-055
    type: depends_on
---
## Description

When an authored forward effect is established, the conformance evaluator SHALL register exactly the compensation obligations paired with that effect and bind each registration to its role, eligibility conditions, retry policy, commit boundary and recovery relation.

## Inputs

An admitted compensation definition and a successfully established forward
effect with exact workflow/effect identity.

## Outputs

Registered compensation obligations or typed violations/refusals; the evaluator
does not invoke the compensating operation.

## Behavior

The evaluator SHALL NOT register compensation from a send, receive, delivery or
attempt that lacks the required successful effect. Repeated observation of one
effect SHALL NOT create additional registrations. Each registration SHALL retain
the source definition and effect that created it.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-056-AC-1 | One successful payment effect registers one matching refund obligation after the effect. | Test (TC-056) |
| FR-056-AC-2 | Failed payment attempts and duplicate deliveries register no refund obligation. | Test (TC-056) |
| FR-056-AC-3 | Replaying the same effect identity does not create a second registration. | Test (TC-056) |
| FR-056-AC-4 | Compensation observed before its forward effect produces an ordering violation and cannot retroactively validate registration. | Test (TC-056) |

## Dependencies

- [FR-051](./FR-051-preserve-communication-identities.md).
- [FR-055](./FR-055-activate-protocol-obligations.md).
