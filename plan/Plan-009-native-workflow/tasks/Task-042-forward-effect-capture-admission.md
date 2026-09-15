---
id: Task-042
title: "Admit compensation forward-effect capture sources"
type: Task
status: done
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-009
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-142
    type: verifies
---
## Scope

Complete issue #108, the independent-consumer gap found after #105. Admit an
authored compensation `ForwardEffect` input only through its exact registration
anchor, compensation-registration requirement, compensation subject, type,
model and authority so registration-capture initializers can execute.

## Subtasks

- [x] Extend TC-142 so Full and Partial retry/recovery expressions actually
  read their registration captures from external forward-effect inputs.
- [x] Add the exact registration/forward-effect admission case without a
  generic binder fallback.
- [x] Preserve evaluator-owned captures, typed source failures, exact replay
  and one-short accounting behavior.
- [x] Run the required local Rust gates and PR-readiness reviews.
- [x] Record issue #108 closure and repin the blocked `quire-protocol` consumer.

## Delivery

The v2 state evaluator now admits a forward-effect binder only when the
admitted registration anchor and compensation-registration authority agree
exactly. TC-142 executes both Full and Partial retry/recovery capture reads,
while direct capture substitution and crossed authority remain refusals.
