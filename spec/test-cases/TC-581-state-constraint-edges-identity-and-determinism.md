---
id: TC-581
title: "Undefined, absent and initially-false constraints, bounds, identity and determinism"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-158
    type: verifies
---
# TC-581: Undefined, absent and initially-false constraints, bounds, identity and determinism

## Description

Verify the constraint's edge cases, that it is no bound, that it stays out of the identity and is named in the method, and determinism.

Verify the constraint over a protocol subject, read on memory values.

Scope: FR-158-AC-3 to FR-158-AC-6.

## Test Procedure

Fixtures: ADR-021 §7.3's subject; a constraint whose body is undefined at one state; ADR-025 §10's `SB` with the constraint `Zero` of FR-158-AC-5.

1. Run with the undefined-at-one-state constraint, with an absent constraint name, with a constraint false at the initial state, and with no universe for `counters`.
2. Compare the obligation and request identities of the constrained and unconstrained requests; read the method; repeat TC-580 step 1.
3. Run `SB` with `Zero` under `sc`, under `tso` and under `ra`; read `ProtocolSystem`'s `offered_reductions`; run `SB` with an absent constraint name.
4. Compile FR-158-AC-6's unit with `Low` and `Wide`; read its emitted
   `Low` node and its requirement records; run AC-1's request naming `Low`
   and write its wire request; run a request naming `Wide` and a wire
   request carrying `Wide`'s node key; compile `constraint Bad ... on
   M::Counter::bump { true }`; run a clause selecting `Low` over a snapshot
   with every counter at 0.


Tag the tests `#[trace("TC-581", "FR-158-AC-n")]`.

## Expected Results

- Step 1: the item returns `Violated` with cause `UndefinedEvaluation` and a counterexample ending at that state, which replays, and that state is not a boundary state; `invalid_runtime_input`/`invalid-value` naming the constraint with no state expanded; one stored state; `RequiresBound`.
- Step 2: equal obligation identities and different request identities, so neither result settles the other; `Low` in the method; equal outcomes and boundary counts.
- Step 3: under `sc`, 3 stored states and `ConstraintReached{boundary_states: 2}`; under `tso`, `Violated` with counterexample `Sx`, `Sy`, `Lx`, `Ly`; under `ra`, 3 stored states and `ConstraintReached{boundary_states: 2}` with no counterexample; the constraint offered; `invalid_runtime_input`/`invalid-value` naming the absent constraint with no state expanded.
- Step 4: a `state_clause` node of clause `constraint` anchored at
  `Counter` binding `[self]`, read by QSL's I2 read; a record for `Wide`
  only; AC-1's outcome with `Low`'s node key as `reductions.constraint`;
  `invalid_runtime_input`/`invalid-value` naming the constraint twice, no
  state expanded; `invalid_syntax`; `truth: true`.
