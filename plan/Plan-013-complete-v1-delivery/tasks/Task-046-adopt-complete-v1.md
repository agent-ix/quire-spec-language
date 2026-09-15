---
id: Task-046
title: "Adopt the frozen complete-V1 baseline"
type: Task
status: done
track: A00
priority: P0
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-055
    type: references
  - target: ix://agent-ix/quire-spec-language/IT-011
    type: references
  - target: ix://agent-ix/quire-specification/Task-010
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-144
    type: verifies
---
# Task-046: Adopt the frozen complete-V1 baseline

## Scope

Complete QSL #116 by installing Plan-013 against QSpec `8d0fbad`, reconciling
all 83 Agent-A rows and existing L1–L6 evidence, and preserving the accepted
issue/test dependency graph without changing central semantics.

## Subtasks

- [x] Add IT-011 and TM-010 as the local adoption and coverage boundary.
- [x] Map each capability to one primary issue, central TestCase and qualification owner.
- [x] Create Task-047 through Task-054 with explicit serial and external dependencies.
- [x] Complete the selected lane-boundary specification review and resolve findings.
- [x] Run local Rust, Quire and PR-readiness gates and prepare QSL #116 for merge.

## Deliverables

- Plan-013, its nine typed tasks and TC-144's executable Rust audit.
- An evidence-honest L1–L6 reconciliation and explicit external owner boundaries.
- A merged issue #116 PR that unblocks Task-047 / QSL #117.

## Notes

The reviewed QSpec commit is immutable input. A contradiction returns to QSpec;
it is not resolved by weakening QSL or editing another lane's authority.
