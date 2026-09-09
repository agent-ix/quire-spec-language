---
id: Task-021
title: "Compile native clauses with retained original mapping"
type: Task
status: done
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-022
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-095
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-096
    type: verifies
---
## Scope

Implement the mapped compiler entry point using existing source-map, native
compiler and package APIs. Exercise successful runtime workflow and typed
refusals; preserve producer ownership and make the completed PR reviewable.

Implemented at 22d4e9a; three mapped integration tests pass. PR-readiness
spec/code reviews are SR-125 through SR-133. Full LC05 remains open.
