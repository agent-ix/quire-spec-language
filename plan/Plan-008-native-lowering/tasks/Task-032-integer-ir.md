---
id: Task-032
title: "Deliver compiler-owned bounded integer IR lowering"
type: Task
status: done
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-033
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-111
    type: verifies
---
## Scope

Extend the existing compiler projection with explicit integer IR selection,
actual bound scalar/source correspondence and standalone export. C retains
backend implementation. Local checks and the requested reviews occur at PR
readiness; Task-020's deferred activation assurance does not gate this delivery.
