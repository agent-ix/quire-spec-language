---
id: Task-039
title: "Publish opaque native-temporal v2 trigger identity"
type: Task
status: done
track: serial
priority: high
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-053
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-141
    type: verifies
---
# Task-039: Publish opaque native-temporal v2 trigger identity

## Scope

Add v2 request and result contracts, public constructor-private views and strict
readers carrying a bounded nonempty byte identity. Retain exact bytes in every
canonical preimage and correction relation; leave v1 modules and their public
textual instance API unchanged.

## Subtasks

- [x] Write TC-141 controls that fail before the v2 surface exists.
- [x] Implement request v2 production and strict reading with canonical binary encoding.
- [x] Implement result v2 evaluation and strict reading tied to the exact request bytes.
- [x] Prove version separation, mutation refusal and absence of field substitution.
- [x] Run local Rust gates and PR-time reviews; then update the QProtocol pin.
