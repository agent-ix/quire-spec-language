---
id: TC-521
title: "The explicit-state model checker proves, refutes and stops over model subjects"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: verifies
---
# TC-521: The explicit-state model checker proves, refutes and stops over model subjects

## Description

Verify `check_model` on ADR-018 §6's worked example under both fairness
granularities, a bounded clause, an invariant, deadlock detection, a mixed
formula, every limit including `max_automaton_states`, and determinism.

Scope: FR-126-AC-1 to FR-126-AC-6.

## Test Procedure

Fixtures: ADR-018 §6's ConfigVersion example unit and its shipped unit,
universe `{a, b}`, both versions 0; the `Counter` unit (FR-124-AC-1); the
`Health` unit and its `Restless` variant (FR-126-AC-4).

1. The TP-4 claim under `fair weak each` and under `fair weak`, over the
   example unit; under `fair weak each` over the shipped unit.
2. The TP-2 bounded variant and the TP-1 invariant over the example unit.
3. `DeadlockFreedom` over `Counter` with no `terminal` member and with
   `terminal when`; `always eventually holds(c.value = 0)` under
   infinite-trace.
4. The recovery-stability formula over `Health` and over `Restless`.
5. The TP-4 claim with `max_depth` 1, with `max_states` 2, and with a poll
   that returns `true`; the `eventually[0,100]` formula over `Restless` with
   `max_automaton_states` 50 and with the default; a subject with an
   undecided contract conjunction; a subject with an unbounded population
   root.
6. Step 1's first two requests, twice each.

Tag the tests `#[trace("TC-521", "FR-126-AC-n")]`.

## Expected Results

- Step 1: `Holds{Exhaustive}` with 15 product states; `Violated` with an
  empty stem and the three-step `upd(a)` loop for `c = b`; `Violated` with
  the loop `(0,0) -upd(a)-> (0,0) -upd(b)-> (0,0)`.
- Step 2: `Violated` with a five-step prefix on which `vb` is never 2;
  `Holds{Exhaustive}`.
- Step 3: `Violated`, `kind: Deadlock`, prefix `0 -inc-> 1 -inc-> 2 -inc->
  3`; `Holds{Exhaustive}`; `Violated` with the stutter loop at value 3 and
  the stutter marker set.
- Step 4: `Holds{Exhaustive}`; `Violated` with a lasso on which `healthy`
  never holds at three consecutive positions.
- Step 5: `BoundReached{depth: 1}`; `Stopped(ResourceExhausted,
  MaxStates)`; `Stopped(Cancelled, …)`; `Stopped(ResourceExhausted,
  MaxAutomatonStates)` with an automaton-state count of 50 and no
  counterexample, then `Violated`; `Undecided(UndecidedSuccessor)`;
  `RequiresBound` with no state explored.
- Step 6: equal outcomes and byte-equal counterexamples.
