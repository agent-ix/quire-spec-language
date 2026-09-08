---
id: Task-002
title: "Adopt Rust verification in CI and retire Python"
type: Task
status: done
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-001
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-012
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
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
  - target: ix://agent-ix/quire-spec-language/TC-010
    type: verifies
---

## Scope

Adopt Rust verification in CI and retire Python within FR-012/NFR-005.

## Subtasks

- [x] Run real selected fixture modes and the Rust code-review gates.
- [x] Switch CI/docs, remove all four Python helpers, and record language inventory and evidence pins.

## Deliverables

Rust-only owned verification paths, current workflow documentation and recorded actual results.

## Notes

Fresh TypeSpec/Node producer qualification stays unavailable pending explicit owner disposition.
Hosted CI is manual-dispatch only at the owner's direction. All required checks
ran locally; future hosted execution also needs private trace-dependency access.
