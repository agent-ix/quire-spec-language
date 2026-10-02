---
id: TC-531
title: "The explicit-state model checker decides strong fairness by SCC refinement"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-130
    type: verifies
---
# TC-531: The explicit-state model checker decides strong fairness by SCC refinement

## Description

Verify `check_model` on ADR-019 §6's mutex under weak, strong `each` and
strong `whole` fairness, enabledness read from the model state, a
refinement whose remainder passes, cancellation in the second phase, mixed
kinds and determinism.

Scope: FR-130-AC-1 to FR-130-AC-4.

## Test Procedure

Fixtures: ADR-019 §6's mutex (universe `{m}`, owner 0) and the `Handoff`
variant of FR-130-AC-2, each with `always eventually holds(m.owner = 1)`.

1. The mutex claim under `weak each`, `strong each` and `strong` (no
   granularity) on `acquire`.
2. The `Handoff` claim under `strong each` on `acquire`.
3. The mutex `strong each` claim with a `Cancel` handle cancelled when the
   second phase first checks it; the mutex claim with `weak each` and
   `strong each` on `acquire`; the requests of steps 1 and 2 twice each.
4. Check the certificate of step 1's `strong each` proof; then the same
   certificate with `S`'s witness naming `acq(2)`.

Tag the tests `#[trace("TC-531", "FR-130-AC-n")]`.

## Expected Results

- Step 1: `Violated`, empty stem, loop `0 -acq(2)-> 2 -rel-> 0`;
  `Holds{Exhaustive}` over 5 product states; `Violated` with the same loop.
- Step 2: `Violated` with stem `0 -acq(2)-> 2` and loop `2 -pass-> 3
  -pass-> 2`.
- Step 3: `Stopped{Cancelled, None}`; `Holds{Exhaustive}`; equal outcomes
  and byte-equal counterexamples.
- Step 4: accepted, `UnfairStrong` listing `acq(1)` with `sub` `{(2, q1)}`
  `Trivial`, `Proved{Exhaustive, Certified}`; rejected `WitnessFails`.
