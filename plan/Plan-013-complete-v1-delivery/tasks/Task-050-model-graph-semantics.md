---
id: Task-050
title: "Implement model graph binding, inheritance and closed dispatch"
type: Task
status: not_started
track: A04
priority: P0
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-049
    type: depends_on
  - { target: ix://agent-ix/quire-specification/FR-150, type: references }
  - { target: ix://agent-ix/quire-specification/FR-151, type: references }
  - { target: ix://agent-ix/quire-specification/FR-152, type: references }
  - { target: ix://agent-ix/quire-specification/FR-153, type: references }
  - { target: ix://agent-ix/quire-specification/TC-195, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-196, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-197, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-198, type: verifies }
---
# Task-050: Implement model graph binding, inheritance and closed dispatch

## Scope

Execute QSL #120 against the accepted I03 producer interface: immutable model
correspondence, original/effective provenance, closed populations, typed
references/traversal, conformance/redefinition and unique most-specific dispatch.

## Subtasks

- [ ] Confirm the merged producer pin and write TC-195–198 first.
- [ ] Implement normalization/provenance and all systems-model reference kinds.
- [ ] Implement closed lookup, specialization validation and ambiguity refusal.
- [ ] Pass local Rust gates and PR-time Rust/gap review; merge before Task-051.

## Deliverables

- Replayable model derivations without ambient registries or display-name identity.
- Typed closure, cycle, weakening and dispatch-ambiguity failures with no substitute.

## Notes

Filament producer code is outside scope. This task consumes only its accepted,
versioned interface.
