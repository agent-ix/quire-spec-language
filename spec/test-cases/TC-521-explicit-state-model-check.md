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
granularities, bounded clauses `on origin` and `on each`, an invariant,
deadlock detection, a mixed formula, every limit with its value, the second
phase at `max_depth`, `max_automaton_states`, and determinism.

Scope: FR-126-AC-1 to FR-126-AC-7, FR-126-AC-10, FR-126-AC-11 and FR-126-AC-16.

## Test Procedure

Fixtures: ADR-018 §6's ConfigVersion example unit and its shipped unit,
both with `attemptUpdate` framed `modifies self.versionNumber` (FR-154),
universe `{a, b}`, both versions 0, and the subject's `max_candidates` set
to 1,000,000 (TC-472's 1,000 would stop the example at its first expansion); the `Counter` unit (FR-124-AC-1); the
`Health` unit and its `Restless` variant (FR-126-AC-4).

1. The TP-4 claim under a weak `each` constraint and under a weak
   constraint with no granularity, over the example unit; under the weak
   `each` constraint over the shipped unit.
2. The TP-2 `on origin` variant and the TP-1 invariant over the example
   unit; the two TP-2 `on each` clauses of FR-126-AC-2 over `Counter`.
3. `DeadlockFreedom` over `Counter` with no `terminal` declaration and with a
   `When` member covering value 3; `always eventually holds(c.value = 0)`
   under infinite-trace.
4. The recovery-stability formula over `Health` and over `Restless`.
5. The TP-4 claim with no granularity with `max_depth` 2, with `max_depth`
   3, with `max_states` 2, with `max_transitions` 3, and with a `Cancel`
   handle already cancelled; `Counter` with an evaluation meter budget of zero.
6. The `eventually[0,100]` formula over `Restless` with
   `max_automaton_states` 50 and with the default; a subject whose
   contract conjunction is refused, with no clause false and none
   undefined; `always holds(true)` over the `Counter` subject with `pre CanInc
   { 6 mod (2 - self.value) >= 0 }` from value 2; a subject with an unbounded population root.
7. Step 1's first two requests, twice each.
8. The TP-4 claim with no granularity over the example unit with initial
   versions `a` 2 and `b` 0 (FR-126-AC-10); `always holds(6 mod (2 -
   c.versionNumber) >= 0)` over the example unit with both versions 0 and
   with `a` 0 and `b` 1 (FR-126-AC-11).
9. The guarded `on each` clauses of FR-126-AC-16 over `Counter`: each with its
   `when` condition read by the test's own letters.

Tag the tests `#[trace("TC-521", "FR-126-AC-n")]`.

## Expected Results

- Step 1: `Holds{Exhaustive}` with 30 product states, the 15-state product of
  each bound instance; `Violated` with an
  empty stem and the three-step loop `(0,0) -upd(b)-> (0,1) -upd(b)-> (0,2)
  -upd(b)-> (0,0)` bound to `a`; `Violated` with
  the loop `(0,0) -upd(a)-> (0,0) -upd(b)-> (0,0)`.
- Step 2: `Violated` with a five-step prefix bound to `a`, on which `va` is never 2;
  `Holds{Exhaustive}`; `Holds{Exhaustive}`; `Violated` with the prefix
  `0 -inc-> 1`.
- Step 3: `Violated`, `kind: Deadlock`, prefix `0 -inc-> 1 -inc-> 2 -inc->
  3`; `Holds{Exhaustive}`; `Violated` with the stutter loop at value 3 and
  the stutter marker set.
- Step 4: `Holds{Exhaustive}`; `Violated` with a lasso on which `healthy`
  never holds at three consecutive positions.
- Step 5: `BoundReached{depth: 2}`; `Violated` with the AC-1 loop;
  `Stopped{ResourceExhausted, {MaxStates, 2}}`;
  `Stopped{ResourceExhausted, {MaxTransitions, 3}}`;
  `Stopped{Cancelled, None}`; `Stopped{ResourceExhausted,
  {EvaluationMeter, 0}}`.
- Step 6: `Stopped{ResourceExhausted, {MaxAutomatonStates, 50}}` with an
  automaton-state count of 50 and no counterexample, then `Violated` with
  at least 101 consecutive unhealthy positions;
  `Undecided(UndecidedSuccessor)`; `Violated` with the empty prefix at
  value 2, `kind: UndefinedEvaluation`, cause `DivisionByZero`, at position 0;
  `RequiresBound` with no state explored.
- Step 7: equal outcomes and byte-equal counterexamples.
- Step 8: `Violated` bound to `b` with an empty stem and the loop through
  `(2,0)`, length 3; `Violated` bound to `a` with the prefix to position 2,
  `kind: UndefinedEvaluation`, cause division-by-zero; `Violated` bound to
  `b` with the prefix `(0,1) -upd(b)-> (0,2)` at position 1.
- Step 9: `Violated` with the prefix `0 -inc-> 1 -inc-> 2`; `Holds{Exhaustive}`;
  `Violated` with the prefix `0 -inc-> 1 -inc-> 2 -inc-> 3`; `Holds` with an
  empty closure certificate; `Violated`, `kind: UndefinedEvaluation` naming
  the activation at position 2.
