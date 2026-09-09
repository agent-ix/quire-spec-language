---
id: Task-008
title: "Implement and qualify source-bound native models"
type: Task
status: in_progress
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-040
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-041
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-042
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-043
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-044
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-045
    type: verifies
---
# Task-008: Implement and qualify source-bound native models

## Scope

Implement reviewed model admission and link_native by sharing the existing
linker. Qualify the source-derived Rust model producer before checker use.

## Subtasks

- [x] Write initial public API model/source tests and record missing-API failure.
- [x] Implement immutable model roles, admission and bounded artifact identity.
- [ ] Complete model mutations/boundaries and write native link qualification tests.
- [ ] Extend shared resolution for explicit native references and operations.
- [ ] Execute TC-040–045 and preserve original linker compatibility.

## Deliverables

Rust model module, domain-specific Rust fixture producer, independent source
fixture, traced model/link tests and actual focused evidence.

## Notes

No runtime object IDs are enumerated in model metadata. Every unused declaration
and role is admitted and source-corresponded. No partial model/package escapes.

Admission and the repaired source-derived producer are implemented at d1168dd.
Five initial model tests pass through the real API. TC-040–045 remain planned
until their full authored cases, including link_native, run. Task-011 is complete;
native reference/operation resolution is the next serial implementation step.
