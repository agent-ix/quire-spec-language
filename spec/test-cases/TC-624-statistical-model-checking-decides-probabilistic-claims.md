---
id: TC-624
title: "EN-4 decides probabilistic claims with Okamoto and SPRT, with Bonferroni, symmetry and activation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: verifies
---
# TC-624: EN-4 decides probabilistic claims with Okamoto and SPRT, with Bonferroni, symmetry and activation

## Description

Verify EN-4's Okamoto and SPRT decisions on `P95`, the exact sample counts and thresholds, Bonferroni and symmetry, activation, and the stops and refusals.

Scope: FR-189-AC-1 to FR-189-AC-5.

## Test Procedure

Fixtures: `Service` and `P95` at `5 ms` and `2 ms`; a two-initial-state variant; a two-server variant with an admitted symmetry declaration and an asymmetric workload; the activation variant and the two-point `M` variant of FR-189-AC-4; `NoFault`; the `Health` variants; a `mean of` claim. Seed 7 throughout.

1. `P95` at `5 ms` and `2 ms` with Okamoto, then with SPRT.
2. Compute Okamoto's `N` for `m = 1` and `m = 2`, and the SPRT thresholds; run step 1's first request twice.
3. Run the two-initial-state variant; the symmetric variant with each workload.
4. Run the activation variant with Okamoto; reduce `quantile 1/2 of M >= 3` over the two-point variant.
5. Compute `N` for `NoFault`; run the `Health` variants; request `mean of` with SPRT.

Tag the tests `#[trace("TC-624", "FR-189-AC-n")]`.

## Expected Results

- Step 1: Accepted with lower end at least `19/20` and `N = 23,026`; Rejected; with SPRT Accepted and Rejected below 23,026 samples.
- Step 2: 23,026 and 26,492; thresholds bounding `±ln 100` outward; equal outcomes.
- Step 3: `m = 2` with `α' = 1/200` and trace indices `2i + j`; `m = 1`, then `m = 2`.
- Step 4: `samples = 23,026`, `draws > samples`; the event `M < 3` with the strict bound `< 1/2`, false at `Pr(M < 3) = 1/2`.
- Step 5: 9,210,341; `Unsupported(NotMarkov)` and Undecided `UndecidedSuccessor`; `NotStatistical`.
