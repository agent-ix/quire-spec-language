---
id: TC-630
title: "S3 checks exact-only forms and fairness sets, and EN-5 advertises exact evidence"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-195
    type: verifies
---
# TC-630: S3 checks exact-only forms and fairness sets, and EN-5 advertises exact evidence

## Description

Verify S3's exact-only forms, fairness sets on every-scheduler claims, the `exact_only` classification, and routing by evidence kind.

Scope: FR-195-AC-1 to FR-195-AC-5.

## Test Procedure

Fixtures: §15.6's `Coin` claims; `Link` with a `cost` reward of 1 on both sends; §15.5's `MeanTime`; `P95` with and without confidence parameters; EN-4 and EN-5 registered.

1. Check `Terminates`, `FairTerminates`, the `Link` until claim, the nested-eventually claim, an `until` whose right operand is `eventually`, `always not holds(…)`, `eventually true`, and `eventually[0, 5] holds(…)` under the infinite-trace profile and `eventually holds(…)` under a bounded one.
2. Check the `Link` expected-cost claim at `rational(6, 5)` and at `6/5`, an integer-reward claim at `3`, `0` and `rational(-1, 2)` and at `5 ms`, a quantity-reward claim at `0.005 s`, `5 m` and a bare `3`, a claim naming an undeclared reward, `MeanTime`, and another `expected elapsed`.
3. Read `exact_only` for each claim of steps 1 and 2 and for `P95` with and without confidence.
4. Run EN-4's gate on the checked `FairTerminates`, on the `Link` expected-cost claim under a workload, on a bounded event under a workload with no confidence parameters, and on `P95` under `Steady` with its confidence parameters.
5. Read EN-5's manifest; settle each refusal of step 4 through FR-192's map.

Tag the tests `#[trace("TC-630", "FR-195-AC-n")]`.

## Expected Results

- Step 1: `Reach` forms with the fairness set recorded and distinct identities; the nested claim, the `until` over `eventually`, `always not` and `eventually true` each `not-state-predicate` at the operand; `unbounded-event` for the interval-bearing event and for `eventually` under the bounded profile.
- Step 2: `ExpectedReward` over `cost` with threshold `6/5`, refused for `6/5` written with `/`; thresholds 3, 0 and `-1/2` check for the integer reward, `5 ms` is `unit-mismatch`; `0.005 s` records 5, `5 m` is `unit-mismatch`, the bare `3` is `type-mismatch`; `missing-name` for the undeclared reward; `MeanTime` and the other `expected elapsed` claim refused `not-yet-implemented`.
- Step 3: true for all but `P95` with confidence.
- Step 4: `EveryScheduler`; `ExactOnlyForm`; `MissingConfidence`; the `P95` claim passes; no sample drawn for any refusal.
- Step 5: `probabilistic-satisfaction` with the evidence kind `exact` only; each refusal settles `unsupported` with `every-scheduler`, `exact-only-form` or `missing-confidence`.
