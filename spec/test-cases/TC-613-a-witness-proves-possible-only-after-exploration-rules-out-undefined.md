---
id: TC-613
title: "A witness proves possible only after exploration rules out an undefined evaluation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-166
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-168
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: verifies
---
# TC-613: A witness proves possible only after exploration rules out an undefined evaluation

## Description

Verify that a sampled witness does not settle a `possible` item on its own:
the item settles `proved` only after exploration completes with no open
node and no undefined evaluation of the target, and an undefined evaluation
anywhere reachable refutes it even when every initial state has a sampled
witness.

Scope: FR-166-AC-5, FR-168-AC-7, FR-169-AC-7.

## Test Procedure

Fixture: ADR-022 §7.1's subject with default limits (`witness_samples` 64,
seed 0).

1. Run `possible 2 / (2 - c.versionNumber) = 2` for `c = a` through EN-1,
   recording phase 0's witnesses and the outcome.
2. Settle step 1's outcome with its replay result.
3. Run `ReachesTwo` through EN-1, recording the run's end, its open nodes
   and the explored node count.
4. Settle step 3's outcome with its replay results.

Tag the tests `#[trace("TC-613", "<AC id>")]`.

## Expected Results

- Step 1: phase 0 holds a `Sampled` witness from `(0, 0)` ending at a node
  with `va = 1`; exploration runs to completion; the outcome is `Undefined`
  at `(2, 0)` with cause `division-by-zero`, not `Witnessed`.
- Step 2: `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation`.
- Step 3: `Witnessed` with `Sampled` sources; `end` `Completed`, no open
  node, 9 explored nodes.
- Step 4: `proved`, `decisive-witness`, `Proved{basis: Witness{[Sampled(…)]}, certification: Certified}`,
  and the record names a completed exploration.
