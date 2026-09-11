---
id: FR-050
title: "Bind protocol roles and instances to exact subjects"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-006
    type: implements
  - target: ix://agent-ix/quire-specification/FR-035
    type: depends_on
---
## Description

When admitting a native protocol definition, the protocol linker SHALL bind every role, role instance, workflow instance, message and channel reference to its exact typed declaration and configuration identity before conformance evaluation.

## Inputs

An admitted native package, protocol definition, A-owned source/profile identities,
D-owned model/configuration identities and explicit workflow-instance bindings.

## Outputs

A finite linked protocol representation with exact subject references, or a
located typed refusal for every unresolved, ambiguous or substituted subject.

## Behavior

The linker SHALL keep roles distinct from components, endpoints and channels.
The linker SHALL keep role instances and workflow instances distinct from their
static definitions. The linker SHALL refuse duplicate role authorities,
ambiguous instance bindings and name-only substitutions. Internal and external
participants use the same identity rules; an external role is not assumed to be
fully observable.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-050-AC-1 | A protocol with internal order, inventory and fulfillment roles plus an external payment role retains every definition and instance identity independently. | Test (TC-050) |
| FR-050-AC-2 | Substituting an endpoint or channel identity for a role, or a component identity for a role instance, refuses linking with the affected source location. | Test (TC-050) |
| FR-050-AC-3 | Two concurrent workflow instances sharing one provider remain distinct assessment subjects. | Test (TC-050) |
| FR-050-AC-4 | An unresolved, duplicate or ambiguous role/instance reference prevents dependent evaluation without removing independent valid definitions. | Test (TC-050) |

## Dependencies

- [US-006](../usecase/US-006-assess-distributed-protocols.md).
- [Shared foundation](../../proposals/quire-v1/shared-foundation.md).
- [Protocol contract](../../proposals/quire-v1/protocol-contract.md).
