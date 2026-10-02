---
id: FR-191
title: "Bound and reproduce a statistical run"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-022
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-024
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: depends_on
---
# FR-191: Bound and reproduce a statistical run

## Description

EN-4's cost SHALL be bounded only by caller-set budgets,
`StatisticalLimits{max_samples, max_draws, max_cycle_steps}`, each an
ADR-014 B-5 budget with a published default, plus time, the clause meter and
cancellation (ADR-024 ST-8). Reaching a budget SHALL stop the run and name
the budget, its value and the request member that raises it, `max_cycle_steps` included (FR-190). EN-4 SHALL
identify a statistical run by its provenance, from which the run reproduces
exactly (ADR-024 RP-1 to RP-3).

## Use case

An operator runs `NoFault` with `max_samples` set to 1,000,000, below
Okamoto's 9,210,341 samples, and the run stops there. The result says
`max_samples` was reached at 1,000,000 and that `statistical.max_samples` raises
it. The operator raises it
and reruns with the same seed, and the rerun reproduces every sample the
first run drew.

## Inputs

```rust
pub struct StatisticalLimits {
    pub max_samples: u64,      // counted samples or cycles per test; default 16_777_216 (2^24)
    pub max_draws: u64,        // samples drawn, counted or not, per test; default 67_108_864 (2^26)
    pub max_cycle_steps: u64,  // steps in one regeneration cycle; default 16_777_216 (2^24)
}
pub enum StatisticalLimit { MaxSamples, MaxDraws, MaxCycleSteps, Time, ClauseMeter, Cancelled }
```

## Outputs

- `StatisticalOutcome::Stopped{cause, limit, partial}` (FR-189), carrying
  the limit, its value, the request member that raises it, and the counts
  reached.
- On every outcome, `StatisticalProvenance{seed, sampler, tests}` with, per
  test, its index, initial state, binding and trace-index range.

## Behavior

### Budgets

- Each `StatisticalLimits` member SHALL be set by the request, with the
  published default above when the request omits it. Its setting name SHALL
  be `statistical.` followed by the member name (`statistical.max_samples`,
  `statistical.max_draws`, `statistical.max_cycle_steps`), by FR-255's
  convention. QSpec FR-408 owns each limit and its setting name; QSL
  publishes the default value. No member SHALL be
  fixed by the language, and none enters the obligation identity.
- When a test would count one more sample or cycle than `max_samples`,
  draw one more sample than `max_draws`, or take one more step in a cycle
  than `max_cycle_steps`, EN-4 SHALL stop and return `Stopped` with cause
  `ResourceExhausted`, the limit, its value, the setting name
  (`statistical.max_samples`, `statistical.max_draws`,
  `statistical.max_cycle_steps`) and the counts reached, which settles
  `incomplete`, `limit-reached{limit, value, setting}` (QSpec FR-408). A time budget, the clause meter and a cancelled `Cancel` handle
  (FR-276)
  SHALL stop the run the same way with their own causes.
- A run stopped before its method completes SHALL never carry a decision; an
  Undecided decision is kept for a run that completed its method.
- The method, `min_cycles` and `max_witnesses` SHALL be request settings,
  recorded in the result, and none enters the obligation identity.

### Provenance and reproduction

- The provenance SHALL record the seed, the sampler identity, and
  per test its index, initial state index, `over` binding and the
  trace-index range it used (`i · m + j` for sample `i` of test `j`).
- Re-running EN-4 with the same subject, claim, method, `min_cycles`, seed,
  sampler and limits SHALL reproduce every sample, every count, every
  estimate and the decision.
- A rerun with only the limits raised SHALL draw, for every trace index both
  runs reach, the same sample as the stopped run.
- A rerun with a different seed SHALL draw independent samples of the same
  obligation, and its result SHALL have the same obligation identity.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-191-AC-1 | `NoFault` (ADR-024 §7.1) with Okamoto and `max_samples` set to 1,000,000 stops before deciding: `Stopped`, `ResourceExhausted`, limit `MaxSamples`, value 1,000,000, member `statistical.max_samples`, 1,000,000 samples counted and no decision. With `max_samples` 9,210,341, Okamoto's `N`, it completes and decides. A request that omits `max_samples` runs with 16,777,216. | Test (TC-626) |
| FR-191-AC-2 | FR-189-AC-4's activation variant with `max_draws` 1,000 stops naming `statistical.max_draws`; a cancelled `Cancel` handle stops with `Cancelled`; neither carries a decision. FR-190-AC-2's transient variant stops `ResourceExhausted` with no decision, settling `incomplete`, `limit-reached` naming `statistical.max_cycle_steps`. | Test (TC-626) |
| FR-191-AC-3 | `P95` with SPRT and seed 7 run twice gives byte-equal results: provenance, per-test counts, estimates and decision. The run stopped by `max_samples` 100 and the rerun with the default limit draw equal samples at trace indices 0 to 99. A run with seed 8 records seed 8, the same obligation identity, and a different sample at some trace index. | Test (TC-626) |
| FR-191-AC-4 | A two-test run records per test its initial state index, binding and trace-index range, with test 1's first trace index equal to 1 and its step 2. Changing the method from Okamoto to SPRT changes the result's method and leaves its obligation identity unchanged. | Test (TC-626) |

## Dependencies

- ADR-024 ST-8, SV-4, RP-1 to RP-3; ADR-014 B-5, TR-1, TR-6.
- [FR-189](FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md).

## References

- QSpec half, which owns `StatisticalLimits` and its setting names in the
  request and the trace-index assignment, while QSL publishes the default
  values: QSpec FR-408 (Linear STD-137).
- Owning ticket: Linear QSL-371.
