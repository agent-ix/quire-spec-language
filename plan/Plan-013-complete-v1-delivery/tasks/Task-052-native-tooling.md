---
id: Task-052
title: "Expose complete lifecycle, CLI, plugin, AOT/JIT and inspection tooling"
type: Task
status: not_started
track: A06
priority: P0
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-051
    type: depends_on
  - { target: ix://agent-ix/quire-specification/FR-300, type: references }
  - { target: ix://agent-ix/quire-specification/FR-301, type: references }
  - { target: ix://agent-ix/quire-specification/FR-305, type: references }
  - { target: ix://agent-ix/quire-specification/FR-306, type: references }
  - { target: ix://agent-ix/quire-specification/TC-220, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-221, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-225, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-226, type: verifies }
---
# Task-052: Expose complete lifecycle, CLI, plugin, AOT/JIT and inspection tooling

## Scope

Execute QSL #122: typed lifecycle APIs and matching CLI outcomes, exact provider
negotiation, isolated Rust plugin wire, deterministic compilation/cache identity
and effect-free evaluator boundaries.

## Subtasks

- [ ] Write TC-220/221/225/226 and API/CLI parity vectors first.
- [ ] Expose every lifecycle operation with stable typed envelopes and exit codes.
- [ ] Implement provider/plugin containment and exact AOT/JIT cache invalidation.
- [ ] Pass local Rust gates and PR-time Rust/gap review; merge before Task-053.

## Deliverables

- Stable Rust and CLI lifecycle surfaces with identical dispositions.
- Versioned process isolation; no dynamic-library ABI or ambient authority.
