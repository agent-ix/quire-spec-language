---
id: Task-054
title: "Qualify complete-V1 QSL compiler and reference-runtime coverage"
type: Task
status: not_started
track: A08
priority: P0
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-053
    type: depends_on
  - { target: ix://agent-ix/quire-specification/FR-132, type: references }
  - { target: ix://agent-ix/quire-specification/FR-133, type: references }
  - { target: ix://agent-ix/quire-specification/FR-270, type: references }
  - { target: ix://agent-ix/quire-specification/FR-271, type: references }
  - { target: ix://agent-ix/quire-specification/FR-272, type: references }
  - { target: ix://agent-ix/quire-specification/FR-303, type: references }
  - { target: ix://agent-ix/quire-specification/FR-311, type: references }
  - { target: ix://agent-ix/quire-specification/FR-336, type: references }
  - { target: ix://agent-ix/quire-specification/TM-009, type: references }
  - { target: ix://agent-ix/quire-specification/TC-047, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-182, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-183, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-223, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-230, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-231, type: verifies }
  - { target: ix://agent-ix/quire-specification/IT-070, type: verifies }
  - { target: ix://agent-ix/quire-specification/IT-071, type: verifies }
  - { target: ix://agent-ix/quire-specification/IT-076, type: verifies }
---
# Task-054: Qualify complete-V1 QSL compiler and reference-runtime coverage

## Scope

Execute QSL #123 after all QSL/WASM implementation tasks and required temporal,
protocol, IR and integration consumers merge. Run the complete corpus against
one exact dependency/tool lock, emit the authority-bound canonical manifest and
reconcile all 83 Agent-A rows.

## Subtasks

- [ ] Pin the complete merged ecosystem and run TC-047, TC-182/183, TC-223,
      TC-230/231 plus IT-070/071/076.
- [ ] Select the exact diagnostic catalog bytes, implement its closed typed causes
      and bind every emitted producer code/cause to that authority.
- [ ] Integrate formatting with the concrete crate-owned checked-package artifact;
      do not accept a caller-mintable semantic verifier or digest.
- [ ] Build the canonical manifest only from the concrete complete checked-package artifact and admitted producer/model correspondences.
- [ ] Admit extensions only from exact declarative grammar/typed-node schemas and checked historical semantic compatibility evidence.
- [ ] Reconcile every capability and criterion to attributable Rust/cross-repo evidence.
- [ ] Preserve unsupported, incomplete, exhausted and failed partitions independently.
- [ ] Pass full local gates, Rust/gap review and the lane qualification audit.

## Deliverables

- Attributable Impl/Ver/Int/Qual state for every Agent-A inventory row.
- Authority-bound canonical package manifest with domain-separated identities.
- Authority-bound extension compatibility across syntax and checked semantics.
- Authority-bound diagnostic catalog, typed causes and formatter semantic identity.
- Exact complete-V1 corpus results without self-report or missing-consumer promotion.
