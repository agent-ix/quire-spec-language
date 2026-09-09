---
id: Task-028
title: "Expose the existing Boolean projection to standalone consumers"
type: Task
status: in_progress
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-029
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-107
    type: verifies
---
## Scope

Share source-only command intake, invoke existing lowering and export its exact
IR bytes. Test real consumer acceptance and refusal provenance; submit a reviewed
PR under LC05. Existing producer ownership and Boolean domain remain unchanged.
