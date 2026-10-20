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

Verify S3's classification of ADR-024 §7's claims and its refusals of out-of-range thresholds, wrong units, unbounded events, trace subjects, fairness under a workload, malformed confidence parameters, comparison thresholds, and the claim's normal form.

Scope: FR-186-AC-1 to FR-186-AC-6.

## Test Procedure

Fixtures: ADR-024 §7's models and claims `NoFault`, `P95`, `LongRun`, `Monthly`; a second workload `Burst` on `Health`; a trace subject.

1. Check the four claims and read back each `CheckedProbabilisticClaim`.
2. Check `P95` at `0.005 s` and at `5 m`; `probability >= 1` and `>= 0`; `quantile 0.95 of (fraction holds(v.up) over [0, 10]) <= rational(1, 2)`.
3. Check the unbounded event, the windowed recovery event, a claim over the trace subject, `NoFault` with `under Throttle` where `Throttle` is declared on `Service`, `NoFault` with `under Steady`, `under every scheduler fair weak Node::tick`, and `under Steady fair weak Node::tick`; a claim `using` a profile that is not temporal, and an event under an unbounded profile.
4. Check `NoFault` with `indifference 0.002`, with only `alpha`, with `alpha` stated twice, with `alpha decimal(1, 4294967295)`, with none; compare normal forms across confidences and workloads, across reordered workload weights, reordered fairness constraints, reward operations declared in another order, a renamed `over` binder and a commuted equality, for a claim that differs only in its subject and one that differs only in its `over` type, and for two declarations of equal content.
5. Check `steps ... >= 3`, `>= rational(1, 2)` and `>= 5 ms`; a non-literal threshold; `fraction ... <= rational(1, 4)`, `>= rational(1, 1)`, `>= rational(0, 1)`, `>= rational(3, 2)` and a negative bound; an event with two `[0, 2^63]` intervals; compile a package that holds a checked claim to S4.
6. Check claims over `Nowhere`, `using zz`, `under Nope`, accumulating an undeclared reward and `fair weak Node::nope`; a reward declared with two value types, and with one.

Tag the tests `#[trace("TC-621", "FR-186-AC-n")]`.

## Expected Results

- Step 1: the forms, schedulers and confidences of FR-186-AC-1.
- Step 2: `5 ms` recorded; `ill_typed`/`unit-mismatch`, `qualitative-threshold` and `quantile-over-fraction` as FR-186-AC-2 states.
- Step 3: refused, horizon 110, `trace-subject` naming window aggregates, `foreign-workload` naming `Throttle` and `Service`, checks, checks with the fairness set, `invalid_syntax` at S1, `not-temporal-profile`, `unbounded-event`.
- Step 4: `invalid-confidence`, `partial-confidence` naming the missing parameter, `partial-confidence` naming `alpha`, a limit refusal (`LimitExceeded`) pending QSL-883, `confidence: None`; normal forms differ for each pair of confidences and workloads, and are equal across reordered weights, reordered fairness constraints and reward operations, a renamed binder, a commuted equality and equal-content declarations, and differ for a claim that differs only in its subject or only in its `over` type.
- Step 5: threshold 3 checks; `ill_typed`/`type-mismatch` for the rational and the duration; `ill_typed`/`type-mismatch` for the non-literal; `<= 1/4` checks and the four others `fraction-bound-out-of-range` without naming the qualitative form; a resource-exhausted limit located at the overflowing operator; S4 refuses with `probabilistic-claim`.
- Step 6: each unresolved name `missing-name` at the name; `ambiguous-name` naming the reward; checks.
