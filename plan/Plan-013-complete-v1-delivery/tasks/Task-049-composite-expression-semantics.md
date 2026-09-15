---
id: Task-049
title: "Implement composite values, collection algebra and total functions"
type: Task
status: not_started
track: A03
priority: P0
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-048
    type: depends_on
  - { target: ix://agent-ix/quire-specification/FR-143, type: references }
  - { target: ix://agent-ix/quire-specification/FR-144, type: references }
  - { target: ix://agent-ix/quire-specification/FR-145, type: references }
  - { target: ix://agent-ix/quire-specification/FR-146, type: references }
  - { target: ix://agent-ix/quire-specification/FR-149, type: references }
  - { target: ix://agent-ix/quire-specification/FR-307, type: references }
  - { target: ix://agent-ix/quire-specification/TC-188, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-189, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-190, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-191, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-194, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-227, type: verifies }
---
# Task-049: Implement composite values, collection algebra and total functions

## Scope

Execute QSL #119 for record, tuple, finite-recursive and all collection kinds;
explicit conversions and higher-order collection operations; lexical values;
named predicates; and termination-checked total pure functions.

## Subtasks

- [ ] Write TC-188–191, composite TC-194 and TC-227 before implementation.
- [ ] Implement value kinds and collection operations with kind/order/multiplicity bounds.
- [ ] Implement pure calls and checked recursion with no reflection, mutation, I/O or callbacks.
- [ ] Pass local Rust gates and PR-time Rust/gap review; merge before Task-050.

## Deliverables

- Complete composite value/expression kernel and immutable semantic libraries.
- Property evidence for operation matrices, empty cases, bounds and termination.
