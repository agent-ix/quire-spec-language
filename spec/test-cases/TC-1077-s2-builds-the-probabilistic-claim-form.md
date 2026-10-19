---
id: TC-1077
title: "S2 builds the probabilistic claim form"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-392
    type: verifies
---
# TC-1077: S2 builds the probabilistic claim form

## Description

Verify that S2 builds the `probabilistic-clause` form for each of the five body shapes, every measure, the scheduler, activation and confidence parts, dispatches the leading token to its one production, builds whatever the facet selection, and leaves every range, unit and completeness check to S3.

Scope: FR-392-AC-1 to FR-392-AC-6.

## Test Procedure

Fixtures: ADR-024 §7's models and claims `NoFault`, `P95`, `LongRun`, `Monthly`; the `mean of`, `expected accumulate`, `expected elapsed`, `steps` and `elapsed` claims of FR-392-AC-3; a claim with `over (n: Node)` and `on each`; a claim `under every scheduler fair weak Node::tick fair strong each Node::send`; `NoFault` with only `alpha`; `NoFault` with `alpha` twice; a trace-subject claim; a threshold-`1` claim; `P95` at `5 m`; `NoFault` in a unit without the complete facets.

1. Build the four claims and read back each form, its scheduler, confidence items and spans.
2. Dispatch a unit beginning `probabilistic`; count the dispatch arm's calls; read the profile, subject and workload names back as written.
3. Build the `mean of`, `expected` and `steps`/`elapsed` claims and read back each body, measure, weighting and horizon.
4. Build the `over`/`on each` claim and the two-item fairness claim; read back the parameter, activation and fairness items.
5. Build the lone-`alpha` and repeated-`alpha` claims and read back the confidence items; build the trace-subject, threshold-`1` and `5 m` claims and assert S2 emits no diagnostic; then check each at S3 and assert the refusals of FR-186.
6. Build `NoFault` with and without the complete facets and compare the forms.

Tag the tests `#[trace("TC-1077", "FR-392-AC-n")]`.

## Expected Results

- Step 1: the bodies, schedulers and confidence items of FR-392-AC-1, each with its source span.
- Step 2: exactly one call, to `probabilistic_clause`; names unresolved.
- Step 3: `MeanOfFraction` with its weighting; `ExpectedUntil` with `Accumulate(duration)` and `Elapsed`; `Steps` and `Elapsed` measures with `Steps` and `Quantity` horizons.
- Step 4: the `over` parameter and `Each{parameter, when}`; two fairness items with their kind and granularity.
- Step 5: one confidence item and no default; two `alpha` items in order; no S2 diagnostic for the three claims; S3 refuses each.
- Step 6: equal forms, no S2 diagnostic.
