---
id: TC-584
title: "Reduced proofs, reduction causes, depth and refutations settle through the one map"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-160
    type: verifies
---
# TC-584: Reduced proofs, reduction causes, depth and refutations settle through the one map

## Description

Verify each new row of the terminal-value map, constraint-only proofs, precedence, the horizon under partial-order reduction, and that a refutation names no reduction.

Scope: FR-160-AC-1 to FR-160-AC-4 and FR-160-AC-7.

## Test Procedure

Fixtures: The outcomes of TC-570, TC-576, TC-582, TC-568 and TC-580; ADR-021 §7.3's subject with constraints `Wide` and `Low`.

1. Settle each outcome of FR-160-AC-1.
2. Run and settle the two constrained runs of FR-160-AC-2.
3. Run and settle the runs of FR-160-AC-3 that complete to their horizon.
4. Settle §7.1's `fair weak whole` refutation from a symmetry run and from an unreduced run.
5. Run §7.2's TP-4 instance with partial-order reduction and `max_states` 3, and settle it.

Tag the tests `#[trace("TC-584", "FR-160-AC-n")]`.

## Expected Results

- Step 1: each label, basis, value and category as FR-160-AC-1 states, with three records for the orbit.
- Step 2: `Proved{Exhaustive}` with `Wide` in the method; `ConstraintReached`.
- Step 3: `inconclusive`, `unsettled`, `ReductionHorizon{max_depth: 2}`; `BoundReached{depth: 1}`.
- Step 4: `refuted`, `decisive-counterexample`, no reduction named, byte-equal counterexamples.
- Step 5: `incomplete`, cause `limit-reached` naming `max_states`, value 3 and its request member; never `failed`. Every `Reduced` proof in step 1 carries `Uncertified`.
