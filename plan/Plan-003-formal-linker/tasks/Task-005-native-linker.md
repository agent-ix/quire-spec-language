---
id: Task-005
title: "Implement exact native formal-environment linkage"
type: Task
status: done
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-013
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-020
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-021
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-022
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-023
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-024
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-030
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-031
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-032
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-033
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-034
    type: verifies
---
# Task-005: Native formal linker

## Scope

Implement FR-013 in a native module, consuming the pinned public IR API. Add
structured declaration provenance to the existing native diagnostic envelope.
Use borrowed immutable models, owned parsed source and no cross-request cache.

## Subtasks

- [x] Integrate the exact IR dependency and Rust 1.98.1/serde policy changes.
- [x] Implement import selection, scoped references, shape lookup and atomic output.
- [x] Exercise every listed public-API test and existing parser/audit regressions.

## Deliverables

Actual LinkedPackage API with located owner-qualified declarations and stable
refusals. Successful linkage does not claim static typing or evaluability.
