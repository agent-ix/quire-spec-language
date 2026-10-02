---
id: FR-186
title: "Check probabilistic claim forms at S3"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-022
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-024
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: depends_on
---
# FR-186: Check probabilistic claim forms at S3

## Description

S3 SHALL admit a `probabilistic` claim over a model subject, check its
scheduler, its form, its event or measure, its thresholds, its units and its
confidence parameters (ADR-024 §2, PM-8), and record a
`CheckedProbabilisticClaim` whose obligation identity binds the subject, the
form, the scheduler and the confidence parameters it states (ADR-013 O-09).
QSpec owns the grammar and the meaning of each form; this requirement
specifies what the compiler checks and records.

## Use case

A modeller writes `probabilistic P95 using ev on Service under Steady on
origin with alpha 0.01, beta 0.01, indifference 0.01 { quantile 0.95 of
(accumulate duration from holds(s.phase = Busy) until holds(s.phase = Done)
within 10) <= 5 ms }`. S3 records a quantile claim under the workload
`Steady` at those confidences, or refuses it with a diagnostic naming what
is wrong: a threshold outside `(0, 1)`, a threshold in a unit of another
dimension, an unbounded event, or confidence parameters that leave no
indifference region.

## Inputs

- A parsed `probabilistic` claim, the checked model, its workloads and
  rewards (FR-185), and the profile it names (FR-123).

## Outputs

```rust
pub struct CheckedProbabilisticClaim {
    pub subject: ModelSubjectRef,          // FR-125
    pub profile: ProfileRef,
    pub scheduler: ProbScheduler,
    pub form: ProbForm,
    pub confidence: Option<Confidence>,
    pub over: Option<Binding>,
    pub obligation: ObligationIdentity,    // ADR-013 O-09
}
pub enum ProbScheduler { Workload(WireNodeId), Every }
pub enum Bound { AtLeast(Rational), AtMost(Rational) }
pub enum ProbForm {
    Probability { bound: Bound, event: Event },
    Quantile { q: Rational, measure: BoundedMeasure, threshold: Bound },
    MeanOfFraction { bound: Bound, fraction: FractionMeasure },
    LongRunFraction { bound: Bound, predicate: CheckedPredicate, weight: Option<Identifier> },
}
pub enum Event {
    Formula { formula: CheckedTemporalFormula, horizon: u64 },
    Compare { measure: BoundedMeasure, bound: Bound },
}
pub enum BoundedMeasure {
    Accumulate { reward: Identifier, from: CheckedPredicate, until: CheckedPredicate, within: u64 },
    Steps { from: CheckedPredicate, until: CheckedPredicate, within: u64 },
    Fraction(FractionMeasure),
}
pub struct FractionMeasure { pub predicate: CheckedPredicate, pub window: u64, pub weight: Option<Identifier> }
pub struct Confidence { pub alpha: Rational, pub beta: Rational, pub indifference: Rational }
```

## Behavior

### Kind and subject

- S3 SHALL classify a `probabilistic` claim with the QSpec FR-290 kind
  `probabilistic-satisfaction`.
- S3 SHALL require a model subject (FR-125). A `probabilistic` claim over a
  trace subject SHALL be refused, with a diagnostic that names window
  aggregates (FR-194) as the form for observed statistics.
- S3 SHALL require exactly one scheduler: `under W` naming a workload of the
  subject's model (FR-185), or `under every scheduler`.
- A claim under a workload SHALL carry no fairness set; S3 SHALL refuse one,
  naming ADR-024 PM-7.

### Events and measures

- An `Event::Formula` SHALL be a formula under a bounded profile with
  activation `on origin` and only interval operators (ADR-018 TP-2). S3
  SHALL record its horizon `h` (ADR-014 TR-4). A formula with an unbounded
  operator SHALL be refused, with a diagnostic stating that a probabilistic
  event needs a finite window.
- `accumulate R from holds(A) until holds(B) within h` SHALL name a reward
  `R` declared by at least one operation of the model; a step of an
  operation that declares no `R` contributes 0. `steps …` and `fraction holds(P) over [0, h]`
  SHALL be admitted with `h` a natural number, and `weighted by R` SHALL
  name a reward.
- A comparison `M <= c` or `M >= c` SHALL have a threshold `c` of `M`'s
  type: a natural number for `steps`, the reward's dimension for
  `accumulate`, and an exact rational in `(0, 1)` for `fraction`.

### Thresholds and units

- Every probability threshold `θ`, quantile level `q`, mean-of-fraction
  threshold and long-run threshold SHALL be an exact rational with
  `0 < θ < 1`. S3 SHALL refuse a threshold of 0 or 1, with a diagnostic
  naming the qualitative form (an ADR-018 TP-2 claim, or a possible
  property).
- A quantile SHALL be over an `accumulate` or `steps` measure. A `mean of`
  SHALL be over a fraction measure.
- A threshold with a dimension SHALL have the reward's dimension. A
  threshold in another unit of that dimension SHALL convert exactly by
  QSpec FR-142; a threshold of another dimension SHALL be refused.

### Confidence parameters

- A claim MAY state `alpha`, `beta` and `indifference`, all three or none.
  When it states one or two, S3 SHALL refuse it.
- When stated, S3 SHALL require `0 < α < 1`, `0 < β < 1`, `0 < ι`,
  `θ − ι > 0` and `θ + ι < 1` for the claim's threshold `θ`, and refuse
  the claim otherwise, naming the parameter.
- The obligation identity SHALL bind the subject, the profile, the form, the
  scheduler (the workload's identity, or `every`), the `over` binding and,
  when stated, the confidence parameters. It SHALL bind no method and no
  budget.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-186-AC-1 | ADR-024 §7's five claims check: `NoFault` as `Probability{AtLeast(999/1000), Formula{horizon: 1000}}`; `P95` as `Quantile{q: 19/20, Accumulate{duration, within: 10}, AtMost(5 ms)}`; `LongRun` as `LongRunFraction{AtLeast(999/1000)}`; `Monthly` as `Probability` over `Compare{Fraction{window: 10000}, AtLeast(999/1000)}`; each with `Workload(Steady)` and its stated `Confidence`. | Test (TC-621) |
| FR-186-AC-2 | `P95` with threshold `0.005 s` checks and records `5 ms`; with threshold `5 m` (length) it is refused naming the dimension. `probability >= 1 [E]` and `probability >= 0 [E]` are refused naming the qualitative form. `quantile 0.95 of (fraction …)` is refused. | Test (TC-621) |
| FR-186-AC-3 | `probability >= 0.99 [always holds(n.healthy)]` under event-position is refused as an unbounded event; `probability >= 0.99 [eventually[0,100] always[0,10] holds(n.healthy)]` checks with horizon 110. A `probabilistic` claim over a trace subject is refused, and its diagnostic names window aggregates. A claim `under Steady` with `fair weak Node::tick` is refused naming PM-7. | Test (TC-621) |
| FR-186-AC-4 | `NoFault` with `indifference 0.002` is refused (`θ + ι = 1.001`); with only `alpha` stated it is refused; with none stated it checks with `confidence: None`. `NoFault` at `alpha 0.01` and at `alpha 0.05` have different obligation identities; `NoFault` under `Steady` and under a second workload `Burst` have different obligation identities. | Test (TC-621) |

## Dependencies

- ADR-024 §2 PF-1 to PF-10, PM-7, PM-8, QS-2, QS-5, QS-6; ADR-018 TP-2,
  SM-2; ADR-013 O-09; ADR-014 TR-4.
- [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (profiles, fairness sets, interval operators),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  (model subject), [FR-185](FR-185-declare-random-parameters-workloads-and-rewards.md).

## References

- QSpec half, which owns the claim grammar, the forms and the FR-290 kind:
  QSpec FR-407 and FR-290 (Linear STD-137).
- Owning ticket: Linear QSL-371.
