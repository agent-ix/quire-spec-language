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

QSL SHALL settle every `probabilistic-satisfaction` item that negotiation
routes to statistical evidence as exactly one FR-331 terminal record with
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

- A `StatisticalOutcome` (FR-189, FR-190), or the negotiation disposition of
  a `probabilistic-satisfaction` item that was not routed to EN-4.

## Outputs

```rust
// TerminalValue gains:
//   Measured(StatisticalVerdict)
pub struct StatisticalVerdict {
    pub decision: Decision,                 // FR-189
    pub estimate: Rational,
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
// UnsupportedCause gains: NotMarkov{…}, EveryScheduler, MissingConfidence
```

## Behavior

### Negotiation

- CG's `negotiate_*` arm for `probabilistic-satisfaction` SHALL route an
  item that names `statistical` evidence only to a candidate advertising
  (`probabilistic-satisfaction`, `statistical`), and an item that names
  `exact` evidence only to a candidate advertising `exact`. It SHALL never
  substitute one evidence kind for the other.
- An item naming `statistical` evidence SHALL settle `unsupported` at
  negotiation, with no engine run, when its claim states `under every
  scheduler` (`EveryScheduler`) or states no confidence parameters
  (`MissingConfidence`). An item naming an evidence kind no registered
  candidate advertises SHALL settle `unsupported`,
  `unsupported-requested-capability`.

### The map

- The map from `StatisticalOutcome` to `TerminalValue` SHALL be exhaustive,
  with no `_` arm:

| Outcome | FR-331 value | FR-241 execution | `measured` axis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- | --- |
| `Completed`, Accepted | `measured` | `completed` | `accepted` | `Measured{decision: Accepted, …}` | success, never proof evidence |
| `Completed`, Rejected | `measured` | `completed` | `rejected` | `Measured{decision: Rejected, …}` | violation |
| `Completed`, Rejected by a test whose `undefined` is set | `measured` | `completed` | `rejected`, cause `UndefinedEvaluation{where, cause}` | `Measured{decision: Rejected, …}` | violation |
| `Completed`, Undecided | `measured` | `completed` | `undecided` | `Measured{decision: Undecided(cause), …}` | inconclusive |
| `Stopped` | `failed` | `resource-incomplete` | none | `Incomplete(cause)` naming the limit | incomplete |
| `Unsupported`; negotiation refusal | `unsupported` | `unsupported` | none | `Unsupported(cause)` | unsupported |

- The record SHALL carry no FR-242 truth value and no FR-243 settlement
  basis for any of these rows.
- `estimate` SHALL be `p̂` for a probability bound, `Pr(M <= c | activated)`
  for a quantile claim with the empirical nearest-rank quantile of the
  counted samples beside it, the sample mean for a mean of a fraction, and
  the ratio estimate for a long-run fraction; for several tests it is the
  least favourable test's. `interval` SHALL be present for Okamoto and the
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
| FR-192-AC-1 | Each outcome row maps exactly as the table states: an Accepted, a Rejected and an Undecided (`IndifferenceRegion`) `Completed` outcome; a `Stopped` naming `max_samples`; `Unsupported(NotMarkov)`. None of the five records carries an FR-242 truth or an FR-243 basis. | Test (TC-627) |
| FR-192-AC-2 | FR-189-AC-1's Okamoto result at `5 ms` settles `measured`, `accepted`, with `interval` present, `coverage: FiniteSample`, `samples = 23,026`, `tests = 1`, `alpha_per_test = 1/100`; the SPRT result at `2 ms` settles `measured`, `rejected` with no `interval`; FR-190-AC-1's result settles `accepted` with `coverage: Asymptotic`. | Test (TC-627) |
| FR-192-AC-3 | A request with `P95` accepted and the `2 ms` claim rejected fails the pipeline gate with the same status as a request holding one O-16 violation; the same request without the rejected item passes. Its proof summary counts 0 proved items. | Test (TC-627) |
| FR-192-AC-4 | At negotiation, with EN-4 registered and no exact candidate: `P95` naming `statistical` routes to EN-4; `P95` naming `exact` settles `unsupported-requested-capability`; a claim `under every scheduler` naming `statistical` settles `unsupported`, `EveryScheduler`; `NoFault` with no confidence parameters naming `statistical` settles `unsupported`, `MissingConfidence`. No engine runs for the last three. | Test (TC-627) |
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
  the evidence kinds and the no-substitution rule: Linear STD-137.
- Owning ticket: Linear QSL-371.
