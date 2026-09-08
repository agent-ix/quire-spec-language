---
id: Task-004
title: "Verify native syntax merge readiness"
type: Task
status: in_progress
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-003
    type: depends_on
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

Run the actual local runtime, trace and code/Rust review gates, resolve SR-009 dispositions, and record the exact merge candidate. The final plan audit and authorized landing follow task completion. No hosted dispatch.

## Subtasks

- [ ] Complete local gates and durable reviews.
- [ ] Record the exact verified candidate and the next LC02 gate.

## Deliverables

Reproducible local results, resolving matrix and code/Rust review. The final
plan audit and private merge are follow-on workflow actions, not a circular
prerequisite for this task's own completion.
