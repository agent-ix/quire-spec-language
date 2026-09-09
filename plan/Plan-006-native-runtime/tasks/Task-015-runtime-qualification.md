---
id: Task-015
title: "Qualify review and hand off the complete native runtime slice"
type: Task
status: not_started
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-018
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-006
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-003
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/IT-006
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-077
    type: verifies
---
# Task-015: Qualify review and hand off the complete native runtime slice

## Scope

Execute the complete native API milestone, required local regression gates, actual code/Rust review and non-semantic gap reconciliation. Land a reviewable qualified change and retain the remaining full-workflow gates.

## Subtasks

- [ ] Run every IT-006 success criterion against real model/source/input bindings; retain healthy, violating, refused and incomplete outcomes with actual counts/events.
- [ ] Run the documented local Cargo/build/style/rustdoc/CLI/audit checks appropriate to the change, serially using existing caches; inspect terminal exits and retain exact commands/revisions.
- [ ] Use the actual agent-skills/code-review/SKILL.md and agent-skills/rust-review/SKILL.md, including relevant rust-style and implementation-gap-analysis guidance; fix substantive findings and rerun only affected verification.
- [ ] Run QUOIN gap-analysis for this plan and its actual Rust trace attributes, leaving the owner-declined optional semantic comparison off. Reconcile every task, criterion, metric and matrix status with actual evidence.
- [ ] Update the private owning LC03 issue and PR with exact spec/review/qualification revisions. Merge only when ready, without dispatching hosted CI.
- [ ] Record the next remaining LC02/FS03/backend/Quire work in its existing owning tickets; do not close issue-level prerequisites or the original assignment merely because IT-006 passes.

## Deliverables

Recorded full runtime API results, validated code/Rust and gap reviews, accurate TM-004/task statuses, and private PR/issue handoff with explicit remaining scope.

## Notes

Task-014 is qualified at 48f53ae by SR-098. Its dependency is satisfied;
TC-077/IT-006 and complete plan gap/review handoff are the next bounded work.

Run one Cargo phase at a time using nice -n 10, -j 1, --locked --offline, an explicit existing target directory and --test-threads=1. Wait for actual exit before another phase. No additional agents, producer-language runs, public publication or B/C/TL/Filament edits.
