---
id: FR-054
title: "Bind explicit channel and delivery premises"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-006
    type: implements
  - target: ix://agent-ix/quire-specification/FR-050
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-051
    type: depends_on
---
## Description

When admitting a protocol channel, the protocol linker SHALL bind its sender, receiver, message types, ordering policy, delivery cardinality and duplication premises as explicit conformance assumptions.

## Inputs

Typed channel/model references and authored communication premises.

## Outputs

A channel contract bound into protocol identity, or a located typed refusal.

## Behavior

The linker SHALL NOT infer exactly-once delivery, FIFO order across channels,
reliable delivery or synchronized clocks. A queue or endpoint SHALL remain a
channel/component reference rather than becoming a participant role. A
conformance result SHALL identify the exact premises under which it was derived.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-054-AC-1 | At-least-once delivery admits a duplicate receipt while retaining the single authored message/effect obligation. | Test (TC-054) |
| FR-054-AC-2 | A FIFO premise orders messages only within its declared channel and key scope. | Test (TC-054) |
| FR-054-AC-3 | Missing or conflicting sender, receiver, message-type, order or delivery premises refuse admission. | Test (TC-054) |
| FR-054-AC-4 | Changing any channel premise changes the conformance request identity and its result provenance. | Test (TC-054) |

## Dependencies

- [FR-050](./FR-050-bind-protocol-instances.md).
- [FR-051](./FR-051-preserve-communication-identities.md).
