---
id: TC-1049
title: "The completed effective view exposes a state machine's effective states and transitions and conformance"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
---
# TC-1049: The completed effective view exposes a state machine's effective states and transitions and conformance

## Description

Verify that the public effective view answers a state machine's effective states and transitions and type conformance, so the quire-specification TC-235 M19 run 1 and run 7 outcomes are asserted through the view and not by walking source records. Scope: FR-081-AC-17, FR-081-AC-18, FR-081-AC-19, FR-081-AC-20 and FR-081-AC-21.

Catches an implementation that exposes only the machine's own states and transitions (omitting the ancestor's), that orders them by hash or insertion, that drops the inherited transition's guard, that reports conformance by display name, or that adds states and transitions to `declarations()` and so changes identities.

## Test Procedure

1. Normalize K2 plus `ExpressLifecycle` (kind `lifecycle`, supertype `OrderLifecycle`, operation `expedite`, state `Expedited`, transition `[0]` `Placed` to `Expedited` on `expedite`); read `state_machine(ExpressLifecycle)` and `state_machine(OrderLifecycle)`.
2. Call `state_machine` on `Order` (an object type) and on a key absent from the package.
3. Call `conforms` for `(ExpressLifecycle, OrderLifecycle)`, `(ExpressLifecycle, ExpressLifecycle)`, `(OrderLifecycle, ExpressLifecycle)` and `(ExpressLifecycle, Fulfilment)`.
4. Compare `declarations()`, `type_identities()` and the view identity with and without steps 1 to 3's calls, and the declaration set for any state or transition entry.
5. Repeat step 1 with `ExpressLifecycle` transition `[0]` given guard `NotYetPlaced`.
6. Normalize with `ExpressLifecycle` states `[Expedited, Placed]`, and separately with `OrderLifecycle` transition `[3]` whose `to` names `ix://test/orders/ExpressLifecycle/Expedited`, a state of another machine.

## Expected Results

- Step 1: `ExpressLifecycle` yields states `Expedited` (declared by `ExpressLifecycle`, position 0), then `Draft`, `Placed`, `Cancelled` (declared by `OrderLifecycle`, positions 0, 1, 2), and transitions `ExpressLifecycle` `[0]`, then `OrderLifecycle` `[0]`, `[1]`, `[2]`, in that order because `ix://test/orders/ExpressLifecycle` sorts before `ix://test/orders/OrderLifecycle` as UTF-8 bytes; `OrderLifecycle` `[0]` is guarded by `NotYetPlaced` and emits `OrderPlaced`. `state_machine(OrderLifecycle)` yields only `Draft`, `Placed`, `Cancelled` and `OrderLifecycle` `[0]` to `[2]`.
- Step 2: nothing.
- Step 3: true, true, false, false; then false, false, false (the copy shares a display name but not a key, so it conforms to nothing and nothing but itself conforms to it).
- Step 4: byte-identical; no state or transition entry in `declarations()`.
- Step 5: as step 1, with `ExpressLifecycle` `[0]` guarded by `NotYetPlaced`.
- Step 6: `invalid_model_binding`/`derivation-conflict` for the repeated state, `invalid_model_binding`/`malformed-declaration` naming `ix://test/orders/OrderLifecycle` at `TypeDefinition.transitions[3].to` for the transition naming another machine's state (a reference naming no node at all would refuse `missing_declaration`/`missing-name` instead); no view, so no accessor.
