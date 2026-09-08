---
id: Task-001
title: "Implement Rust audit modes"
type: Task
status: not_started
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-012
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-001
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-002
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-003
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-004
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-005
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-006
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-007
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-008
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-009
    type: verifies
---

## Scope

Implement Rust audit modes within FR-012/NFR-005.

## Subtasks

- [ ] Implement bounded read/strict JSON/typed error helpers and actual mode checks.
- [ ] Add resolving Rust TC/AC tags and meaningful malformed, identity, path and resource cases.

## Deliverables

Rust audit executable, private modules and traced tests.

## Notes

Fresh TypeSpec/Node producer qualification stays unavailable pending explicit owner disposition.
