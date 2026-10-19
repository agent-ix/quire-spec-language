---
id: TC-1083
title: "The liveness engine reads positions past a dead automaton and refuses unbuilt clauses"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: verifies
---
# TC-1083: The liveness engine reads positions past a dead automaton and refuses unbuilt clauses

## Description

Verify that a TP-4 item whose negation automaton has no move still has the
letters and contracts of the positions past it read, that the read is charged
to the run's budgets and joins no product total, and that the strong-fairness,
guarded-activation and unbuilt-certificate cases refuse.

Scope: FR-126-AC-12 and FR-126-AC-13.

## Test Procedure

Fixtures: ADR-018 §6's ConfigVersion example unit, universe `{a, b}`, both
versions 0, the subject's `max_candidates` 1,000,000.

1. `eventually (holds(c.versionNumber = 0) and holds(6 / (2 -
   c.versionNumber) >= 0))`, with the default limits.
2. The same claim with `max_states` 1.
3. The TP-4 claim of FR-126-AC-1 under `fair strong each attemptUpdate`; the
   same claim with a guarded `on each` activation.
4. A TP-4 claim with no fair accepting cycle, on an engine that does not build
   the FR-339 certificate.

## Expected Results

- Step 1: `Violated` bound to `a`, `kind: UndefinedEvaluation` with cause
  division-by-zero at position 2, the prefix `(0,0) -upd(a)-> (1,0) -upd(a)->
  (2,0)`; no product state is reported for the dead positions.
- Step 2: `Stopped{ResourceExhausted, {MaxStates, 1}}`.
- Step 3: the `StrongFairness` refusal, then the `GuardedActivation` refusal,
  each `unsupported_construct`/`not-yet-implemented`, with no state explored.
- Step 4: `ComponentCertificateNotBuilt` with the run's statistics, not `Holds`.
