---
id: FR-392
title: "Build the probabilistic claim form at S2"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-022
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-024
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-067
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-311
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-195
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-204
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-325
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-407
    type: depends_on
---
# FR-392: Build the probabilistic claim form at S2

## Description

When a `1-draft` unit holds a `probabilistic-clause`, the production of QSpec's shared grammar (§
"Probabilistic, timed and real-time forms", Linear STD-137), S2
(`qsl-forms`) SHALL build one claim form for it from S1's parse, so S3 can
check it (FR-186, FR-195, FR-204). QSpec owns the production and its meaning (QSpec
FR-407); this requirement specifies the form QSL builds. It completes the
S2 half of the probabilistic facet beside the declaration forms of FR-311.

## Use case

A modeller writes `probabilistic P95 using ev on Service under Steady on
origin with alpha 0.01, beta 0.01, indifference 0.01 { quantile 0.95 of
(accumulate duration from holds(s.phase = Busy) until holds(s.phase =
Done) within 10) <= 5 ms }`. S2 builds one form that keeps every part the
clause wrote, in order, with the span S3 needs for a diagnostic.

## Inputs

- A recovery-free `LosslessCst` of a `1-draft` unit (ADR-011 E2).

## Outputs

`DeclarationForm` gains one variant for the `probabilistic-clause` production:

```rust
pub struct ProbabilisticForm {
    pub name: Spanned<Identifier>,
    pub profile: Spanned<Identifier>,          // `using`
    pub subject: Spanned<Identifier>,          // `on`
    pub over: Option<Spanned<Parameter>>,
    pub scheduler: SchedulerForm,
    pub activation: ActivationForm,
    pub confidence: Vec<(Spanned<ConfidenceKey>, Spanned<ExactValue>)>, // declared order
    pub body: ProbabilisticBody,
}
pub enum SchedulerForm {
    Workload(Spanned<Identifier>),
    Every { fairness: Vec<FairnessConstraintForm> },    // fairness-item, declared order
}
pub enum ConfidenceKey { Alpha, Beta, Indifference }
pub enum ProbabilisticBody {                  // probabilistic-form
    Probability { op: BoundOp, threshold: Spanned<ExactValue>, event: EventForm },
    Quantile { level: Spanned<ExactValue>, measure: MeasureForm, op: BoundOp, threshold: Expression },
    MeanOfFraction { fraction: FractionForm, op: BoundOp, threshold: Spanned<ExactValue> },
    LongRunFraction { predicate: Expression, weighting: Option<WeightingForm>, op: BoundOp, threshold: Spanned<ExactValue> },
    ExpectedUntil { quantity: ExpectedQuantity, target: Expression, op: BoundOp, threshold: Expression },
}
pub enum EventForm { Temporal(Expression), Compare { measure: MeasureForm, op: BoundOp, threshold: Expression } }
```

`ExpectedQuantity` is `Accumulate(Spanned<Identifier>)` or `Elapsed`, and
`ExpectedUntil` holds the `until holds(B)` target. A horizon is
`HorizonForm::{Steps(Spanned<u64>), Quantity(Spanned<QuantityLiteral>)}`, as
the production's `horizon` writes it. `MeasureForm` (`accumulate R`, `steps` or `elapsed` from a predicate until a
predicate within a horizon, or a `FractionForm`), `FractionForm`,
`WeightingForm` and `BoundOp` mirror the productions `measure`, `fraction`,
`weighting` and `bound-op`; each part carries its span. The scheduler
variants, the fairness items, the `ActivationForm` and the temporal event
reuse the forms S2 already builds for the same productions:
`FairnessConstraintForm` and `ActivationForm::{Origin, Each{parameter,
when}}` (FR-308, FR-325).

## Behavior

- `LeadingTokenKind` SHALL gain `Probabilistic`, selecting the
  `probabilistic_clause` production for a declaration that begins
  `probabilistic`, and no other production. The dispatch arm SHALL make
  exactly one call (FR-102-AC-4).
- S2 SHALL build the form whether or not the unit selects the complete
  facets. QSpec's facet rule (an unselected facet gives
  `unsupported_construct`) applies to it, but QSL has no header layer for
  that profile, so the gate is not enforced at S2 (FR-308, FR-361 item 8).
- S2 SHALL resolve no name (FR-310): the profile, subject, workload,
  rewards, predicates and fairness operations stay as written.
- S2 SHALL build the form from the production's CST, keeping the confidence
  items, the fairness items and the measure's parts in declared order, each
  with its span.
- S2 SHALL record each stated confidence item as written, in declared
  order, and SHALL fill no default and drop no repeat, so a claim that
  states one or two of `alpha`, `beta` and `indifference`, or one of them
  twice, still builds. QSpec refuses a repeat and FR-186 refuses a partial
  set, both at S3.
- S2 SHALL add no diagnostic of its own for a well-formed CST.
- S2 SHALL leave thresholds' ranges and units, the scheduler's validity, the
  subject's kind, event boundedness and confidence completeness unchecked
  for S3. FR-186 checks the claim's kind, subject, scheduler, thresholds,
  units and confidence for `probability`, `quantile`, `mean of` and
  `long_run_fraction`; FR-195 checks `expected accumulate` and the fairness
  set under `every scheduler`; FR-204 checks `expected elapsed` and the
  `elapsed` measure.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-392-AC-1 | ADR-024 §7's `NoFault`, `P95`, `LongRun` and `Monthly`, written in the shared grammar's productions, build a `ProbabilisticForm` each: `NoFault` as `Probability` with a `Temporal` event, `P95` as `Quantile` over an `accumulate duration … within 10` measure, `LongRun` as `LongRunFraction`, `Monthly` as `Probability` over a `Compare` event on a fraction measure; each with `Workload(Steady)`, its confidence items in declared order, and the span of its source text. | Test (TC-1077) |
| FR-392-AC-2 | A unit whose declaration begins `probabilistic` dispatches to `probabilistic_clause` and to no other production, through one call (FR-102-AC-4); the form holds the profile, subject and workload names as written, resolved to nothing. | Test (TC-1077) |
| FR-392-AC-3 | `mean of (fraction holds(s.phase = Up) over [0, 100] weighted by duration) >= 0.99` builds `MeanOfFraction` with its `WeightingForm`; `expected accumulate duration until holds(s.phase = Done) <= 5 ms` and `expected elapsed until holds(s.phase = Done) <= 5 ms` build `ExpectedUntil` with `Accumulate(duration)` and `Elapsed`; `quantile 0.5 of (steps from holds(A) until holds(B) within 10)` and `… (elapsed from holds(A) until holds(B) within 10 ms)` build `Steps` and `Elapsed` measures with `Steps` and `Quantity` horizons. | Test (TC-1077) |
| FR-392-AC-4 | A claim with `over (n: Node)` and `on each` activation keeps the `over` parameter and `Each{parameter, when}`; `under every scheduler fair weak Node::tick fair strong each Node::send` builds `Every` with two fairness items holding kind and granularity as written. | Test (TC-1077) |
| FR-392-AC-5 | `NoFault` stating only `alpha 0.01` builds with exactly one confidence item and no default for `beta` or `indifference`; `NoFault` stating `alpha 0.01, alpha 0.05` builds with both items in order; a claim over a trace subject, a threshold `1` and `P95` with threshold `5 m` each build at S2 with no S2 diagnostic, and are refused only at S3 (FR-186-AC-2 to FR-186-AC-4). | Test (TC-1077) |
| FR-392-AC-6 | `NoFault` in a unit that does not select the complete facets builds the same `ProbabilisticForm` as in a unit that does, with no S2 diagnostic. | Test (TC-1077) |

## Dependencies

- ADR-024 §2, PM-7, PM-8.
- [FR-067](FR-067-add-s2-forms-and-retire-seam-5.md) (the S2 forms core),
  [FR-311](FR-311-build-random-reward-and-workload-forms-at-s2.md) (the
  declaration forms),
  [FR-186](FR-186-check-probabilistic-claim-forms-at-s3.md),
  [FR-195](FR-195-check-exact-only-forms-and-route-exact-evidence.md) and
  [FR-204](FR-204-check-probabilistic-timed-automata-through-digital-clocks.md)
  (the S3 checks of this form),
  [FR-325](FR-325-parse-temporal-operators-with-an-optional-interval.md)
  (fairness constraint forms).

## References

- QSpec half: the shared grammar's `probabilistic-clause`,
  `scheduler-clause`, `confidence`, `probabilistic-form`, `event`,
  `measure` and `fraction` productions, and QSpec FR-407 for their meaning
  (Linear STD-137).
- Owning ticket: Linear QSL-568.
