---
id: Task-006
title: "Review and land native linking qualification"
type: Task
status: in_progress
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-005
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-013
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-024
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-034
    type: verifies
---
# Task-006: Linker qualification

## Scope

Apply actual code-review and rust-review, record dependency rights and local
gates, reconcile matrix/plan status to executed evidence and land when ready.

## Subtasks

- [ ] Retain actual test, fmt, Clippy, rustdoc, private-audit and Quire evidence.
- [ ] Complete Rust/code review and scoped gap analysis without the declined optional semantic review.
- [ ] Push the exact result and update the private owning/coordination records.

## Deliverables

Reviewable, tested linker PR with explicit remaining FR-006 and full-workflow
work. No external model service or repeated owner approval is introduced.
