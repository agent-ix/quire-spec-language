---
id: Task-022
title: "Expose native execution with retained request results"
type: Task
status: in_progress
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-023
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-097
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-098
    type: verifies
---
## Scope

Compose actual native validation/evaluation without duplicating runtime logic.
Return the complete offered request and native outcome to callers, including
validation failures. Exercise parent/aggregate, operation, failure and retry
cases, then submit the next reviewable LC05 PR.
