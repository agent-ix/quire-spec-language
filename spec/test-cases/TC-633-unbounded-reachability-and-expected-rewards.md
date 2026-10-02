---
id: TC-633
title: "Unbounded reachability and expected rewards are decided by graph precomputation, interval iteration and exact policy iteration"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-198
    type: verifies
---
# TC-633: Unbounded reachability and expected rewards are decided by graph precomputation, interval iteration and exact policy iteration

## Description

Verify the qualitative step, `+∞` expected rewards, interval iteration's two sequences, the exact policy-iteration fallback at equality, and its budget.

Scope: FR-198-AC-1 to FR-198-AC-5.

## Test Procedure

Fixtures: `Coin`; `Link` with `Deliver`'s unbounded variant and the `cost` claims; the `Loop` model of FR-198-AC-3; the `Mec` model of FR-198-AC-5.

1. Decide `Terminates` (minimum and maximum) and `Link`'s unbounded delivery.
2. Decide the `cost` claims to `delivered or attempts = 2` and to `delivered`.
3. Run interval iteration on `Loop` and read each sweep; run with `max_iterations` 3 at threshold `1/2`; decide under a workload.
4. Run step 3's equality case with `max_policy_iterations` 0; run one request twice.
5. Compute the maximum over `Mec`, its witness policy, and decide `probability <= 1/4`; replay the evidence.

Tag the tests `#[trace("TC-633", "FR-198-AC-n")]`.

## Expected Results

- Step 1: minimum 0 with no iteration and policy `wait`; maximum 1; minimum `24/25`.
- Step 2: `11/10` and `6/5` exact, the `<= 6/5` bound met at equality; `+∞`.
- Step 3: rising lower and falling upper bounds around `1/2`; `1/2` decided at equality; one exact solve.
- Step 4: `Stopped(MaxPolicyIterations)`; equal results and policies.
- Step 5: maximum `1/2`; the policy takes `b_exit`; `refuted`, and the evidence replays.
