---
id: Task-021
title: "Compile native clauses with retained original mapping"
type: Task
status: in_progress
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
