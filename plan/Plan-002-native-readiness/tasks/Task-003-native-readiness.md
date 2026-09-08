---
id: Task-003
title: "Repair native syntax boundaries"
type: Task
status: done
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-002
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-003
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-004
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-010
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-001
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-011
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-012
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-013
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-014
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-015
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-016
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-017
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-018
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-019
    type: verifies
---

## Scope

Implemented reviewed a10ec80 CLI/formatter/error refinements, API documentation and canonical tracing of existing/new native tests. FR-010's compatibility amendment was specified at 5d0c9de and reviewed across all eight analyses at 03dfc72 before replacing the failed derive attempt. No model linker/evaluator changes.

## Subtasks

- [x] Complete the reviewed source/test changes.
- [x] Preserve existing APIs, fixtures and claim boundaries.

## Deliverables

Native source/test implementation and documentation.
