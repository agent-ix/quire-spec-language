---
id: Task-011
title: "Repair qualification and linkage construction boundaries"
type: Task
status: in_progress
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-017
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-054
    type: verifies
---
# Task-011: Repair qualification and linkage construction boundaries

## Scope

Resolve all four demonstrated/supported architecture findings in SR-074 before
qualification handoff. Apply the repair first, then complete its producer
qualification with Task-008's actual NativeModel API. Contract a350754 has all eight actual
selected reviews, SR-075–082. This task is added under QUOIN spec-to-plan.

## Subtasks

- [ ] Carry exact borrowed JSON occurrences; write independent provenance controls.
- [ ] Split source intake, typed decoding and IR lowering; unify scalar storage.
- [ ] Separate existing linker inventory/import/clause stages without policy drift.
- [ ] Separate role and syntax audit orchestration stages without policy drift.
- [ ] Execute focused and existing regressions; complete actual code/Rust review.

## Deliverables

Corrected Rust helpers, traced TC-054, preserved linker/audit behavior and evidence.
Missing NativeModel/checker APIs remain explicit ongoing Task-008/009 work.

Producer qualification consumes Task-008's real model API. Task-008 and this
repair therefore share the final Task-010 gate; an artificial Task-011-before-
Task-008 completion dependency would create a cycle. Work remains serial.

## Notes

Source borrowing uses the pinned serde_json raw_value development feature.
No new package, language, lexer or unsafe code. All checks run serially at nice
10 with one build job/test thread and explicit cached target directory.
