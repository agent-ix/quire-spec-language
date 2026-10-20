---
id: TC-1085
title: "The liveness engine reads positions past a dead automaton and refuses unbuilt clauses"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: verifies
---
# TC-1085: The liveness engine reads positions past a dead automaton and refuses unbuilt clauses

## Description

Verify that a TP-4 item whose negation automaton has no move still has the
letters and contracts of the positions past it read, that the read is charged
to the run's budgets and joins no product total, and that the strong-fairness,
guarded-activation and unbuilt-certificate cases refuse.

Scope: FR-126-AC-12 to FR-126-AC-15.

## Test Procedure

Fixtures: ADR-018 §6's ConfigVersion example unit, universe `{a, b}`, both
versions 0, the subject's `max_candidates` 1,000,000.

1. `eventually (holds(c.versionNumber = 0) and holds(6 mod (2 -
   c.versionNumber) >= 0))`, with the default limits.
2. The same claim with `max_states` 1.
3. The TP-4 claim of FR-126-AC-1 as a checked clause with a guarded `on each`
   activation, given to the engine directly.
4. AC-1's weak `each` claim, with and without a built FR-339 certificate.
5. Step 1's claim with initial versions `a` 0 and `b` 1.
6. Step 5's request with `max_states` 6 and with `max_states` 1,000; step 1's
   request with `max_states` 1,000.

## Expected Results

- Step 1: `Violated` bound to `a`, `kind: UndefinedEvaluation` with cause
  division-by-zero at position 2, the prefix `(0,0) -upd(a)-> (1,0) -upd(a)->
  (2,0)`; no product state is reported for the dead positions.
- Step 2: `Stopped{ResourceExhausted, {MaxStates, 1}}`.
- Step 3: the `GuardedActivation` refusal, `unsupported_construct`/
  `not-yet-implemented`, with no state explored.
- Step 4: `Holds{Exhaustive}` with an accepted certificate; without one,
  `ComponentCertificateNotBuilt` with the run's statistics, not `Holds`.
- Step 5: `Violated` bound to `b` with the prefix `(0,1) -upd(b)-> (0,2)` at
  position 1.
- Step 6: `Stopped{ResourceExhausted, {MaxStates, 6}}`; the step 5 finding for
  `b`; the step 1 finding for `a`.
