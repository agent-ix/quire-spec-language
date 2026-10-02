---
id: TC-621
title: "S3 checks probabilistic claim forms, thresholds, units and confidence parameters"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: verifies
---
# TC-621: S3 checks probabilistic claim forms, thresholds, units and confidence parameters

## Description

Verify S3's classification of ADR-024 §7's claims and its refusals of out-of-range thresholds, wrong units, unbounded events, trace subjects, fairness under a workload and malformed confidence parameters, and the obligation identity.

Scope: FR-186-AC-1 to FR-186-AC-4.

## Test Procedure

Fixtures: ADR-024 §7's models and claims `NoFault`, `P95`, `LongRun`, `Monthly`; a second workload `Burst` on `Health`; a trace subject.

1. Check the four claims and read back each `CheckedProbabilisticClaim`.
2. Check `P95` at `0.005 s` and at `5 m`; `probability >= 1` and `>= 0`; `quantile 0.95 of (fraction …)`.
3. Check the unbounded event, the windowed recovery event, a claim over the trace subject, and `NoFault` with a fairness set.
4. Check `NoFault` with `indifference 0.002`, with only `alpha`, with none; compare obligation identities across confidences and workloads.

Tag the tests `#[trace("TC-621", "FR-186-AC-n")]`.

## Expected Results

- Step 1: the forms, schedulers and confidences of FR-186-AC-1.
- Step 2: `5 ms` recorded; the others refused as FR-186-AC-2 states.
- Step 3: refused, horizon 110, refused naming window aggregates, refused naming PM-7.
- Step 4: refused, refused, `confidence: None`; identities differ for each pair.
