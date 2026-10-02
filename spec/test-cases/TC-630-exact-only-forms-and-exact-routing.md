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

Scope: FR-195-AC-1 to FR-195-AC-4.

## Test Procedure

Fixtures: §15.6's `Coin` claims; `Link` with a `cost` reward of 1 on both sends; §15.5's `MeanTime`; `P95` with and without confidence parameters; EN-4 and EN-5 registered.

1. Check `Terminates`, `FairTerminates`, the `Link` until claim and the nested-eventually claim.
2. Check the `Link` expected-cost claim, `MeanTime`, and `expected elapsed` over an untimed subject.
3. Read `exact_only` for each claim of steps 1 and 2 and for `P95` with and without confidence.
4. Read EN-5's manifest; call `check_statistical` with `FairTerminates` and with the `Link` expected-cost claim under a workload.

Tag the tests `#[trace("TC-630", "FR-195-AC-n")]`.

## Expected Results

- Step 1: `Reach` forms with the fairness set recorded and distinct identities; the nested claim refused.
- Step 2: `ExpectedReward` with `Named(cost)` and with `Elapsed`; refused.
- Step 3: true for all but `P95` with confidence.
- Step 4: exactly (`probabilistic-satisfaction`, `exact`); `EveryScheduler`; `ExactOnlyForm`; no sample drawn for either.
