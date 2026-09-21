---
id: FR-035
title: "Retain explicit ecosystem scope identities in assessment requests"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
---
## Description

When binding an ecosystem assessment request, the binder SHALL retain the declared workflow, role, relationship and population-scope identities needed by that request and refuse identity substitutions that change its subject.

## Inputs

An admitted clause, D-owned model/configuration identities, B-owned protocol instance bindings, F-owned observation bindings and explicit finite snapshot or window scope references.

## Outputs

A request bound to the exact declared subjects and completeness dependencies, or a located identity/correlation refusal or explicitly incomplete binding result.

## Behavior

A trace ID, timestamp, endpoint or matching textual name alone does not establish a business relationship. Workflow instances, role instances, message deliveries, attempts and effects remain distinct subjects. Scope references preserve the selected snapshot/window, membership declaration and observation/progress dependencies; static package identity remains separate. Unknown required membership cannot establish aggregate success. D/B/F define the corresponding producer contracts; this requirement introduces no model store, correlation heuristic or alternate schema.

The [package contract](../../proposals/quire-v1/package-contract.md) defines the
typed role mapping. Configuration and binding contracts must agree with the
linked requirements. Concrete runtime bindings are separately identified inputs;
they cannot override static selections. Required role mappings and records needed
by a currently activated obligation are distinct prerequisites.

Runtime binding checks the package contract's stage-specific prerequisites and
producer/native correspondence against the already linked selections. A prior
successful type-check cannot supply missing required runtime scope or authority.
The selected producer canonical domain and native raw-byte domain remain
separate; a changed relation cannot override the linked model/profile.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-035-AC-1 | Two concurrent orders with shared infrastructure retain distinct request subjects and explicitly related payment/shipment/refund identities. | Test (TC-035) |
| FR-035-AC-2 | A delivery ID or telemetry trace ID substituted for the required effect or workflow identity refuses binding. | Test (TC-035) |
| FR-035-AC-3 | A population request retains the exact snapshot/window and membership/completeness dependencies; changing that scope changes the runtime request subject. | Test (TC-035) |
| FR-035-AC-4 | Unknown required membership yields an explicitly incomplete assessment and cannot establish a successful aggregate. | Test (TC-035) |
| FR-035-AC-5 | A missing, repeated or incompatible required role mapping refuses the dependent request; an explicit compatible binding shared by two distinct roles is admitted. | Test (TC-035) |
| FR-035-AC-6 | A runtime configuration selecting another model/profile/binding-contract meaning refuses against the existing linked subject rather than overriding it. | Test (TC-035) |
| FR-035-AC-7 | Changing only the runtime resource limit changes the retained configuration and assessment input identity while preserving the static subject. | Test (TC-035) |
| FR-035-AC-8 | A valid mapping for an unactivated compensation does not require a recovery record; once required, an unavailable observation remains incomplete rather than becoming optional-none or false. | Test (TC-035) |
| FR-035-AC-9 | A previously linked template still refuses a missing required runtime scope selection and reports unavailable required observations as incomplete; a foreign producer/native correspondence cannot override its linked model/profile selection. | Test (TC-035) |

## Dependencies

- [Shared drafting foundation](../../proposals/quire-v1/shared-foundation.md).
- [Composed architecture](../assurance/AD-001-composed-native-language.md).
- [Planned acceptance matrix](../composed-foundation/tests.md).

Proposed requirement for the composed profile; it does not amend historical
accepted definitions or establish an implemented capability.
