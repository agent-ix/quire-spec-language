---
id: TC-615
title: "A single-existential witness proves only after exploration rules out an undefined evaluation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-181
    type: verifies
---
# TC-615: A single-existential witness proves only after exploration rules out an undefined evaluation

## Description

Verify that an HP-5 witness lasso does not settle the item on its own: the
item settles `proved` only after the product exploration completes with no
open node, and a letter of the body that evaluates undefined at a reachable
product state refutes it even when every initial state has a witness.

Scope: FR-181-AC-5.

## Test Procedure

Fixture: ADR-023 §8's secure vault.

1. `exists trace b of V { eventually always holds(v.l @ b = 0 and 1 / (1 - v.l @ b) = 1) }`
   with the default `witness_samples` and with 0; settle each outcome after
   replay.
2. `exists trace b of V { eventually always holds(v.l @ b = 1) }` with the
   default `witness_samples`, recording the run's end and open nodes, and
   settle it.

Tag the tests `#[trace("TC-615", "FR-181-AC-5")]`.

## Expected Results

- Step 1: each initial state has a witness lasso looping `step(0)` at
  `(h, 0)`; the outcome is `Undefined` at the first product state with
  `l = 1`, cause `division-by-zero`; `refuted`, `decisive-counterexample`,
  cause `UndefinedEvaluation`, in both runs.
- Step 2: `Witnessed` from a run with `end` `Completed` and no open node;
  `proved`, `decisive-witness`.
