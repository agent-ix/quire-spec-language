---
id: Task-009
title: "Implement native type constraints and guarded proofs"
type: Task
status: done
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

- [x] Write unchanged FR-006 judgment cases and additional public checker tests.
- [x] Implement bounded contextual constraints and observation/lexical identity.
- [x] Implement proof DAG, bounded presence joins and ordered IR proof goals.
- [x] Execute all checker cases, including independent guard truth-table controls.

## Deliverables

CheckedPackage API, exact expression/source/proof correspondence, bounded checking
usage, and actual native/IR qualification with no runtime Boolean claim.

## Notes

All branches are typed; only potentially evaluated operations need definedness.
An initializer cannot borrow later guards. Proof carriers never invent native
object/enum identity facts.

Started after Task-008 completion at ac36598; seven initial public-API tests
recorded the missing checker API at 8eb8761. The final 24 checker tests execute
TC-025–029 and TC-046–053, including exact authored bindings, actual IR proofs,
nominal/observation/lexical constraints, all lowered budgets, shared expansion,
depth exactly 64, accumulated goals and 202 independently evaluated guard formulas.
Code inspection exposed omitted populations for nested structural references;
the recorded failing regression now passes with bounded transitive input needs,
including skipped context/parameter/result inputs and native reference cycles.
The full local run passes 110 tests and the selected private audit lane passes 3.
Evidence is under reviews/data/native-checking/checker-*. Task-010 owns the final
review and handoff. No runtime execution or full LC02 completion is claimed.
