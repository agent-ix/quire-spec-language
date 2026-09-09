---
id: Task-029
title: "Join actual Quire extraction to the native compiler"
type: Task
status: done
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-030
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-108
    type: verifies
---
## Scope

Consume the existing pinned Rust extractor in A's optional compiler integration.
Verify exact original/body correspondence, retain extraction metadata and execute
real native cases. Preserve C's producer ownership and submit a reviewed PR.
