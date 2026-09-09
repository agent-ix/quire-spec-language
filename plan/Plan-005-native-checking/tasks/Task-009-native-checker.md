---
id: Task-009
title: "Implement native type constraints and guarded proofs"
type: Task
status: pending
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-008
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-006
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-025
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-026
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-027
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-028
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-029
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-046
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-047
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-048
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-049
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-050
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-051
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-052
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-053
    type: verifies
---
# Task-009: Implement native type constraints and guarded proofs

## Scope

Implement the reviewed checker over exact native models and authored bindings.
Reuse IR arithmetic proof; retain original AST and runtime input requirements.

## Subtasks

- [ ] Write unchanged FR-006 judgment cases and additional public checker tests.
- [ ] Implement bounded contextual constraints and observation/lexical identity.
- [ ] Implement proof DAG, bounded presence joins and ordered IR proof goals.
- [ ] Execute all checker cases, including independent guard truth-table controls.

## Deliverables

CheckedPackage API, exact expression/source/proof correspondence, bounded checking
usage, and actual native/IR qualification with no runtime Boolean claim.

## Notes

All branches are typed; only potentially evaluated operations need definedness.
An initializer cannot borrow later guards. Proof carriers never invent native
object/enum identity facts.

