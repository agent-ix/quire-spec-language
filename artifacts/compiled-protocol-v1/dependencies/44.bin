---
id: FR-051
title: "Preserve message, delivery, attempt and effect identities"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-006
    type: implements
  - target: ix://agent-ix/quire-specification/FR-050
    type: depends_on
---
## Description

When binding communications and business observations to a protocol instance, the protocol binder SHALL preserve message, send, receive, delivery, attempt and effect as distinct typed subjects.

## Inputs

A linked protocol, typed message/channel declarations and F-owned observation
bindings with explicit business relationships and provenance.

## Outputs

Bound protocol events and effects, or typed identity/correlation refusals and
explicitly missing-observation dependencies.

## Behavior

The binder SHALL require explicit relationships between transport records,
attempts and effects. The binder SHALL NOT infer a business effect from a send,
receive, successful transport status, repeated delivery, trace identifier or
equal payload alone. The binder SHALL retain source and observation identities
for every accepted correspondence.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-051-AC-1 | One message sent once, delivered twice and applied as one payment effect retains cardinalities 1, 2 and 1 for the respective subjects. | Test (TC-051) |
| FR-051-AC-2 | Relabeling a duplicate delivery as a second business effect refuses the binding or yields the authored duplicate-effect violation. | Test (TC-051) |
| FR-051-AC-3 | A payment attempt with no established effect remains an attempt and cannot activate effect-dependent compensation. | Test (TC-051) |
| FR-051-AC-4 | A transport trace ID substituted for an authored workflow, attempt or effect identity refuses before conformance evaluation. | Test (TC-051) |

## Dependencies

- [FR-050](./FR-050-bind-protocol-instances.md).
- [Shared ecosystem binding](./FR-035-bind-ecosystem-subjects.md).
- F-owned observation/correlation contract under OB01.
