---
id: FR-192
title: "Settle a statistical result on the measured axis"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-022
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-024
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: depends_on
---
# FR-192: Settle a statistical result on the measured axis

## Description

QSL's settlement map in `qsl-replay` SHALL settle every
`probabilistic-satisfaction` item that EN-4 runs as exactly one FR-331 terminal record with
the result value `measured`, carried by `TerminalValue::Measured
(StatisticalVerdict)`, and SHALL never settle it `proved` or `refuted`
(ADR-024 SV-1 to SV-5, SV-9, SV-10). The record SHALL carry QSpec FR-241's
execution disposition and a separate `measured` axis with the values
`accepted`, `rejected` and `undecided`, and no FR-242 truth and no FR-243
basis. A Rejected measurement SHALL fail the pipeline as an O-16 violation
does (ADR-024 RU-2), and proof accounting SHALL never count a measurement.

## Use case

An operator's run has `P95` accepted, `Monthly` rejected and `LongRun`
accepted with asymptotic coverage. The run fails because of `Monthly`, the
report shows "measured: rejected" with its estimate, and the proof summary
counts none of the three as proved.

## Inputs

- A `StatisticalOutcome` (FR-189, FR-190) or a `StatisticalRefusal`
  (FR-189).
- The request's FR-120 limits (the evaluation meter budget and `max_candidates`),
  from which a `Stopped` by an expansion takes its value.

## Outputs

```rust
// TerminalValue gains:
//   Measured(StatisticalVerdict)
pub struct StatisticalVerdict {
    pub decision: Decision,                 // FR-189
    pub estimate: Option<Rational>,         // the least favourable test's (FR-189); None when it has none
    pub interval: Option<(Rational, Rational)>,
    pub basis: StatisticalBasis,
    pub provenance: StatisticalProvenance,  // FR-191
    pub tests: Vec<TestResult>,             // FR-189
    pub witnesses: Vec<SampledWitness>,     // FR-193
}
pub struct StatisticalBasis {
    pub method: StatisticalMethod,
    pub confidence: Confidence,             // the claim's α, β, ι
    pub samples: u64, pub draws: u64,
    pub tests: u32, pub alpha_per_test: Rational, pub beta_per_test: Rational,
    pub coverage: Coverage,
}
pub enum Coverage { FiniteSample, Asymptotic }
// UnsupportedCause gains: NotMarkov{…}, TotalAboveDrawRange{total}, EveryScheduler, MissingConfidence, ExactOnlyForm, ConfidenceBeyondPrecision
```

## Behavior

### EN-4's capability and refusals

- EN-4's provider manifest SHALL be the one FR-280 states, which lists the evidence
  kind `statistical` only. Routing an item by its evidence kind, and never
  substituting one kind for the other, is QSpec FR-290's negotiation rule,
  which QSL does not restate.
- When EN-4's negotiation gate refuses a claim (FR-195, which owns the
  claims it refuses and the order of the causes `EveryScheduler`,
  `ExactOnlyForm`, `MissingConfidence`), the map SHALL settle the refusal
  `unsupported` with that cause.

### The map

- The map from `StatisticalOutcome` to `TerminalValue` SHALL be exhaustive,
  with no `_` arm:

| Outcome | FR-331 value | FR-241 execution | `measured` axis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- | --- |
| `Completed`, Accepted | `measured` | `completed` | `accepted` | `Measured{decision: Accepted, …}` | success, never proof evidence |
| `Completed`, Rejected | `measured` | `completed` | `rejected` | `Measured{decision: Rejected, …}` | violation |
| `Completed`, Rejected by a test whose `undefined` is set | `measured` | `completed` | `rejected`, cause `UndefinedEvaluation{where, cause}` | `Measured{decision: Rejected, …}` | violation |
| `Completed`, Undecided | `measured` | `completed` | `undecided` | `Measured{decision: Undecided(cause), …}` | inconclusive |
| `Stopped` by a limit | `incomplete` | `limit-reached{limit, value, setting}` | none | `Incomplete(LimitReached{limit, value, setting})` (ADR-018 V-7) | incomplete |
| `Stopped` by `AutomatonStates{value}` | `incomplete` | `limit-reached{limit: max_automaton_states, value}` (FR-126 `ResourceExhausted`) | none | `Incomplete(LimitReached{limit, value})` (ADR-018 V-7) | incomplete |
| `Stopped` by a sample expansion (`Expansion(ExpansionStop)`) that names a limit (`EvaluationMeter` or `MaxCandidates`, FR-126) | `incomplete` | `limit-reached{limit, value, setting}`: `limit` the stop's, `value` the configured bound of that limit in the request's FR-120 limits (the evaluation meter's budget, or `max_candidates`), which the map receives with the outcome because the stop carries only its cause and limit, and `setting` the name the request raises it by (`meter`, `max_candidates`) | none | `Incomplete(LimitReached{limit, value, setting})` (ADR-018 V-7) | incomplete |
| `Stopped` by a sample expansion that names no limit and is an internal failure (`ExpansionStop::is_internal_failure`) | `failed` | diagnostic `runtime_invariant`/`established-invariant-broken` | none | `Failed` | failed (ADR-013 O-16 internal failure) |
| `Stopped` by a sample expansion that names no limit and is not an internal failure (for example a state-constraint family that evaluation refuses) | `failed` | the stop's own `ExpansionStop.cause` (QSL FR-120), as the one diagnostic (QSpec FR-331, FR-408 "stops the run with that cause") | none | `Failed` carrying that diagnostic | failed |
| `Stopped` by a cancelled `Cancel` handle | `incomplete` | `cancelled{source}` | none | `Incomplete(Cancelled{source})` (ADR-018 V-7) | incomplete |
| `Unsupported` outcome | `unsupported` | cause `not-markov{…}` or `total-above-draw-range{total}` | none | `Unsupported(cause)` | unsupported |
| `GeneratorMismatch`, `TooManyTests`, `TraceIndexRange` or `FractionForEvent` refusal | `unsupported` | diagnostic `unsupported_projection`/`unsupported-requested-capability` naming the refusal | none | `Unsupported(diagnostic)` | unsupported |
| `ConfidenceBeyondPrecision` refusal | `unsupported` | the cause `confidence-beyond-precision` (QSpec FR-331, FR-408 Admission) | none | `Unsupported(cause)` | unsupported |
| `EveryScheduler`, `MissingConfidence` or `ExactOnlyForm` refusal | `unsupported` | the cause `every-scheduler`, `missing-confidence` or `exact-only-form` | none | `Unsupported(cause)` | unsupported |
| `Admission` or `RequiresBound` refusal, or `PredicateRefused` refusal | none: the refusal is the caller's to report (FR-106 admission failure, ADR-016 EX-8 disposition, FR-196 `PredicateRefused`) | none | none | no terminal result | none |
| `Symmetry` refusal (a malformed symmetry declaration, FR-151) | none: the refusal is the caller's `invalid_runtime_input`/`invalid-value` | none | none | no terminal result | none |
| `NotStatistical` refusal | none: a negotiation outcome, `invalid-request` | diagnostic `invalid_capability`/`not-statistical` (QSpec FR-408 Admission, FR-331) | none | no terminal result | none |
| `NoTest` refusal | `inconclusive` | `no-initial-state` | none | `Inconclusive(NoInitialState)` (ADR-018 V-6) | inconclusive |
| `Internal` refusal | `failed` | diagnostic `runtime_invariant`/`established-invariant-broken` | none | `Failed` | failed (ADR-013 O-16 internal failure) |

- The record SHALL carry no FR-242 truth value and no FR-243 settlement
  basis for any of these rows.
- `estimate` SHALL be absent when the least favourable test has none, and otherwise `p̂` for a probability bound, `Pr(M <= c | activated)`
  for a quantile claim with the empirical nearest-rank quantile of the
  counted samples beside it, the sample mean for a mean of a fraction, and
  the ratio estimate for a long-run fraction; for several tests it is the
  least favourable test's (FR-189). `interval` SHALL be present for Okamoto and the
  regenerative method and absent for SPRT. `coverage` SHALL be
  `Asymptotic` for the regenerative method and `FiniteSample` otherwise.
- `TerminalValue::category` SHALL map `Measured{Accepted}` to success,
  `Measured{Rejected}` to violation and `Measured{Undecided}` to
  inconclusive.
- The run's pipeline gate SHALL fail on a `Measured{Rejected}` record
  exactly as on a violation.
- Proof accounting (ADR-016 §6) SHALL count no `Measured` record, whatever
  its decision; `Measured` carries no `ProofBasis`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-192-AC-1 | Each outcome row maps exactly as the table states: an Accepted, a Rejected and an Undecided (`IndifferenceRegion`) `Completed` outcome; a `Stopped` naming `max_samples`, settling `Incomplete(LimitReached{limit, value, setting})` with setting `statistical.max_samples`; an `Unsupported` outcome with cause `NotMarkov`; a `NoTest` refusal settling `inconclusive`, `no-initial-state`; a `ConfidenceBeyondPrecision` refusal settling `unsupported`, `confidence-beyond-precision`; an `Internal` refusal settling `failed`; a `Stopped` by an expansion naming `EvaluationMeter` settling `incomplete`, `limit-reached` with that limit, its configured value and setting `meter`, and one naming `MaxCandidates` with that limit, its configured value and setting `max_candidates` (the FR-120 settings under their registered names, with no `model_check.` prefix and no second setting), one by an internal-failure expansion naming no limit settling `failed`, `runtime_invariant`/`established-invariant-broken`, and one by a non-internal expansion naming no limit settling `failed` with the stop's own `ExpansionStop.cause` (QSL FR-120) as its one diagnostic. None of the measured, stopped and unsupported records carries an FR-242 truth or an FR-243 basis. | Test (TC-627) |
| FR-192-AC-2 | FR-189-AC-1's Okamoto result at `5 ms` settles `measured`, `accepted`, with `interval` present, `coverage: FiniteSample`, `samples = 23,026`, `tests = 1`, `alpha_per_test = 1/100`; the SPRT result at `2 ms` settles `measured`, `rejected` with no `interval`; FR-190-AC-1's result settles `accepted` with `coverage: Asymptotic`. | Test (TC-627) |
| FR-192-AC-3 | A request with `P95` accepted and the `2 ms` claim rejected fails the pipeline gate with the same status as a request holding one O-16 violation; the same request without the rejected item passes. Its proof summary counts 0 proved items. | Test (TC-627) |
| FR-192-AC-4 | EN-4's manifest is the one FR-280-AC-4 fixes. EN-4's gate given a checked claim `under every scheduler` refuses `EveryScheduler`; given `NoFault` with no confidence parameters refuses `MissingConfidence`; given `expected accumulate duration until holds(c.phase = Replied)` refuses `ExactOnlyForm`; each draws no sample and settles `unsupported` with its cause. | Test (TC-627) |
| FR-192-AC-5 | FR-189-AC-6's result settles `measured`, `rejected`, category violation, with the test's `UndefinedEvaluation` in the record, and fails the pipeline gate as a violation does. | Test (TC-640) |

## Dependencies

- ADR-024 SV-1 to SV-5, SV-9, SV-10, RU-2, RU-5, RU-6; ADR-013 O-16 and O-24
  as amended by ADR-024; ADR-016 §6 as amended.
- [FR-069](FR-069-implement-typed-proof-result-envelope.md) (terminal record
  and category map), [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the same terminal record for temporal items),
  [FR-189](FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md).

## References

- QSpec half, which owns the FR-331 value `measured`, the `measured` axis,
  the evidence kinds and the no-substitution rule: QSpec FR-409, FR-410 and
  FR-290 (Linear STD-137).
- Owning ticket: Linear QSL-371.
