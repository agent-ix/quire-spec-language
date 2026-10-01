---
id: FR-236
title: "Carry exact rational delays in a timed counterexample"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-231
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-232
    type: depends_on
---
# FR-236: Carry exact rational delays in a timed counterexample

## Description

A counterexample over a timed subject SHALL carry the delay before each
discrete step as an exact non-negative rational in the binding's unit, a
time-lock counterexample SHALL end with a final delay, and a liveness
counterexample SHALL be a timed lasso whose repetition is a time-divergent
behaviour (ADR-026 CT-1, CT-2, CT-4). Every refutation an engine reports
has such a counterexample, so it replays with no engine present.

## Use case

An auditor receives a refutation of a 3 ms deadline. The counterexample
names each step and the exact delay before it, including a boundary value
such as exactly 3, so the auditor can replay it in exact arithmetic and see
the reply land just outside a half-open interval.

## Inputs

- A counterexample produced by the zone engine (FR-241), the digitization
  path (FR-243), or the statistical sampler (FR-254).

## Outputs

`TemporalCounterexample` (FR-128) over a timed subject, with:

```rust
pub struct ModelStep {
    pub transition: ModelTransition,
    pub post_state: DigestRecord,          // discrete state and exact clock valuation
    pub delay: Option<ExactRational>,      // Some over every timed subject
}
// TemporalCounterexample gains `final_delay: Option<ExactRational>`,
// Some exactly when kind is TimeLock.
```

## Behavior

- Over a timed subject every `ModelStep` SHALL carry `delay: Some(d)`, the
  exact delay before the step in the binding's unit; under a `Tick` source
  `d` SHALL be 0. Over an untimed subject it SHALL be `None`.
- The post-state digest SHALL be the canonical identity digest of the
  discrete state together with the exact clock valuation.
- A `TimeLock` counterexample SHALL be a finite prefix ending with
  `final_delay`, the delay that reaches the time-locked state; every other
  kind SHALL carry `final_delay: None`.
- **Timed lasso.** Each engine SHALL report a liveness counterexample over a
  timed subject as a lasso whose loop has positive total delay `D`, whose
  last state has the entry's discrete state, in which each clock the loop
  resets has its entry value, and in which each clock the loop never resets
  is, at entry, above the largest constant it is compared with in the model
  and the claim.
- An idle tail SHALL be the lasso whose loop is one stutter step with a
  positive delay.
- A counterexample's length SHALL be its number of discrete steps.
- Each engine SHALL state every delay of a counterexample it reports as an
  exact rational.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-236-AC-1 | `NoLateReply` over `Rpc` with `T = 3 ms` yields steps `send`, `timeout`, `reply` with delays `0`, `3`, `0` and `trace_position` 3, length 3; each post-state digest changes when the step's resulting clock value changes and nothing else does. | Test (TC-691) |
| FR-236-AC-2 | The strict-guard variant's time-lock counterexample has one step, `send` with delay 0, `final_delay` `3` and `kind: TimeLock`; a `Formula` counterexample has `final_delay: None`. | Test (TC-691) |
| FR-236-AC-3 | A liveness refutation of `always eventually holds(c.phase = Replied)` over `Rpc` with `T = 3 ms` yields a lasso whose loop has positive total delay, ends in the entry's discrete state, and in which every clock the loop does not reset is above 3 at entry. | Test (TC-691) |
| FR-236-AC-4 | A counterexample over the untimed `Counter` subject carries `delay: None` on every step, and its bytes equal FR-126's counterexample for the same item. | Test (TC-691) |

## Dependencies

- ADR-026 §7 CT-1, CT-2, CT-4; ADR-018 CX-1, CX-2 and DL-4 as amended by
  ADR-026.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md),
  [FR-128](FR-128-replay-a-model-counterexample.md),
  [FR-231](FR-231-read-a-timed-subject-s-behaviours-as-timed-traces.md),
  [FR-232](FR-232-derive-the-time-lock-freedom-item-and-read-deadlocks-over-time.md).

## References

- QSpec half: QSpec FR-416 and FR-417, with QSpec FR-181 and FR-331
  (Linear STD-139), own the step `delay`, the final delay and the timed
  lasso on the counterexample wire (ADR-026 OV-5, OV-6).
