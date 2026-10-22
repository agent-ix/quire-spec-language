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

Scope: FR-189-AC-1 to FR-189-AC-5, FR-189-AC-7 to FR-189-AC-12, FR-189-AC-13 to FR-189-AC-15.

## Test Procedure

Fixtures: the `Undef` model and its two-initial-state variant (FR-189-AC-8), `BehaviourSampler` stubs giving FR-189-AC-10's (decision, estimate) pairs, an empty-subject request, the activation-probability-0 variant of FR-189-AC-4 with `max_draws` 1,000; `Service` and `P95` at `5 ms` and `2 ms`; a two-initial-state variant; a two-server variant with an admitted symmetry declaration and an asymmetric workload; the activation variant and the two-point `M` variant of FR-189-AC-4; `NoFault`; the `Health` variants; a `mean of` claim; the `Wide` model of FR-189-AC-7. Seed 7 throughout.

1. `P95` at `5 ms` and `2 ms` with Okamoto, then with SPRT.
2. Compute Okamoto's `N` for `m = 1` and `m = 2`, and the SPRT thresholds; run step 1's first request twice.
3. Run the two-initial-state variant; the symmetric variant with each workload.
4. Run the activation variant with Okamoto; reduce `quantile 0.5 of (M) >= 3` (`M` is a placeholder for the claim's parenthesised measure) over the two-point variant.
5. Compute `N` for `NoFault`; run the `Health` variants; request `mean of` with SPRT.
6. Run the `Wide` claim of FR-189-AC-7 with Okamoto.
7. Run `Undef` and its two-initial-state variant (FR-189-AC-8).
8. Read the estimate and interval of FR-189-AC-9's tests: Okamoto and the mean of a fraction ended by an undefined second sample, a decided Okamoto test, an SPRT test, and a `LongRun` test ended before its first positive-weight cycle.
9. Select the least favourable test for each case of FR-189-AC-10.
10. Run the empty-subject request (FR-189-AC-11) and the activation-probability-0 variant (FR-189-AC-12).

11. Give a mean-of-fraction claim event samples from a stub: 3 holding of 4 (FR-189-AC-13).
12. Run a two-test regenerative request and a two-test Okamoto request, each at `max_draws` `u64::MAX` (FR-189-AC-14).
13. Give a quantile claim samples from a stub: 1, 3, 2, 5 at `q = 1/2`; 1, 3, 2 and a censored sample at `q = 9/10` and at `q = 1/2`; the `P95` test (FR-189-AC-15).

Tag the tests `#[trace("TC-624", "FR-189-AC-n")]`.

## Expected Results

- Step 1: Accepted with lower end at least `19/20` and `N = 23,026`; Rejected; with SPRT Accepted and Rejected below 23,026 samples.
- Step 2: 23,026 and 26,492; thresholds bounding `±ln 100` outward; equal outcomes.
- Step 3: `m = 2` with `α' = 1/200` and trace indices `2i + j`; `m = 1` under each workload.
- Step 4: `samples = 23,026`, `draws > samples`; the event `M < 3` with the strict bound `< 1/2`, false at `Pr(M < 3) = 1/2`.
- Step 5: 9,210,341; an `Unsupported` outcome with cause `NotMarkov` and Undecided `UndecidedSuccessor`; `NotStatistical`.
- Step 6: an `Unsupported` outcome with cause `TotalAboveDrawRange{total}` and `total` `2^256 + 1`, settled `unsupported`, cause `total-above-draw-range`, `total` the decimal string `"115792089237316195423570985008687907853269984665640564039457584007913129639937"`.
- Step 7: the rejected test with `estimate` `None`, `undefined` naming test 0, trace 0, position 0, and the verdict's `estimate` `None`; the variant keeps test 0's Accepted result.
- Step 8: the first sample's value as the estimate and no interval for the ended tests; an interval for the decided Okamoto test only; `None` for the `LongRun` test.
- Step 9: the verdict's `estimate` and `interval` of the test AC-10 names for each case.
- Step 10: `NoTest`, settled `inconclusive`, `no-initial-state`; `Stopped` `Limit{MaxDraws, 1000}` with 0 counted.
- Step 11: estimate `3/4`, no refusal.
- Step 12: the regenerative request runs; the Okamoto request is refused `TraceIndexRange`.
- Step 13: `quantile` `Value(2)`; `Censored`; `Value(2)`; absent for the `P95` test.
