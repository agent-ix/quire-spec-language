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
2. Check `P95` at `0.005 s` and at `5 m`; `probability >= 1` and `>= 0`; `quantile 0.95 of (fraction holds(v.up) over [0, 10]) <= rational(1, 2)`.
3. Check the unbounded event, the windowed recovery event, a claim over the trace subject, `NoFault` with `under Throttle` where `Throttle` is declared on `Service`, `NoFault` with `under Steady`, `under every scheduler fair weak Node::tick`, and `under Steady fair weak Node::tick`.
4. Check `NoFault` with `indifference 0.002`, with only `alpha`, with `alpha` stated twice, with `alpha decimal(1, 4294967295)`, with `alpha rational(n, n)` (`n` of 70,000 bits), with none; compare obligation identities across confidences and workloads.

Tag the tests `#[trace("TC-621", "FR-186-AC-n")]`.

## Expected Results

- Step 1: the forms, schedulers and confidences of FR-186-AC-1.
- Step 2: `5 ms` recorded; `ill_typed`/`unit-mismatch`, `qualitative-threshold` and `quantile-over-fraction` as FR-186-AC-2 states.
- Step 3: refused, horizon 110, refused naming window aggregates, `foreign-workload` naming `Throttle` and `Service`, checks, checks with the fairness set, `invalid_syntax` at S1.
- Step 4: `invalid-confidence`, `partial-confidence` naming the missing parameter, `partial-confidence` naming `alpha`, `DecimalScaleBeyondSource` at S2, the rational-bits limit outcome, `confidence: None`; identities differ for each pair.
