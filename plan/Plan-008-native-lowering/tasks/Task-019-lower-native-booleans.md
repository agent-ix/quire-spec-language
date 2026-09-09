---
id: Task-019
title: "Lower checked Boolean clauses through the strict binder"
type: Task
status: done
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-009
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-092
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-093
    type: verifies
---
# Task-019: Lower checked Boolean clauses through the strict binder

## Scope

Implement complete bounded native-to-IR projection with exact source, authored
identity and explicit read correspondence. Reuse the existing binder.

## Subtasks

- [x] Write TC-092/093 assertions and establish the missing-API failure.
- [x] Implement the supported Boolean lowering and whole-package refusals.
- [x] Pass targeted Rust tests and reconcile the specified boundaries.
