---
id: FR-193
title: "Keep and replay sampled witnesses"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-022
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-024
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-188
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: depends_on
---
# FR-193: Keep and replay sampled witnesses

## Description

EN-4 SHALL keep, up to `max_witnesses`, the samples on the violating side
of the claim's bound as **sampled witnesses** (ADR-024 SV-6), the first in trace-index order. The layer-6 replay facade SHALL
replay a sampled witness through FR-128's model-trace replay plus two
checks: each drawn value is in its support, and each draw recomputed from
the seed selects the recorded choice (SV-8). A sampled witness never changes
the item's `measured` value, and it SHALL replay as an ADR-018 counterexample
for the qualitative claim "the event holds on every behaviour" (SV-7).

## Use case

The `2 ms` latency claim is rejected. The operator opens the first sampled
witness: a request, then an attempt that drew `d = 3 ms` and `outcome = Ok`,
with step probability `343/5000`. Replay re-executes it through the model,
confirms both draws from the seed, and reproduces latency 3 ms. The same
trace refutes the qualitative claim that every behaviour meets 2 ms.

## Inputs

```rust
pub struct SampledWitness {
    pub test: u32,
    pub trace: u64,
        pub steps: Vec<SampledStep>,       // FR-188: transition, post digest, choices, drawn values, probability; a timed step adds its delay (FR-254)
    pub path_probability: Rational,    // product of step probabilities
    pub value: WitnessValue,           // event false, the measure's value, or Undefined(UndefinedEvaluation)
}

pub fn replay_sampled_witness(
        request: &ReplayRequest,           // FR-098: package, provision, seed, sampler
    provenance: &StatisticalProvenance, // FR-191: per test, initial state and binding
    witness: &SampledWitness,
) -> Result<ReplayOutcome, ReplayRefusal>;
```

## Outputs

- `ReplayOutcome::Reproduced` when every check agrees, or
  `ReplayOutcome::Inconclusive(ReplayParity)`.
- `ReplayRefusal` with FR-128's catalog codes.

## Behavior

### Keeping witnesses

- EN-4 SHALL record a sample as a witness when its event evaluates false
  (a `>= θ` bound) or true (a `<= θ` bound), when a quantile claim's
  activated sample is on the violating side of its PF-4 event, a censored
  sample included, or when a mean-of-fraction sample's fraction lies below
  `θ` (a `>= θ` bound) or above it (a `<= θ` bound). A long-run fraction
  run SHALL keep no witness other than an undefined one.
- EN-4 SHALL record a sample on which the claim evaluates undefined as a
  witness with value `Undefined(UndefinedEvaluation)`, and SHALL keep it
  whatever `max_witnesses` is (ADR-024 SV-11).
- It SHALL keep at most `max_witnesses` other witnesses, the first in trace-index
  order across tests, each with its exact path probability.

### Replay

- `replay_sampled_witness` SHALL recompile the package (FR-098), re-admit
  the initial state the provenance records for the witness's test from the
  provision, and re-execute each step
  through FR-128's model-trace replay: enabledness, successor selection by
  post-state digest, and SM-1 evaluation of the event or measure over the
  replayed positions.
- It SHALL check that each drawn random value lies in its parameter's
  support; a value outside SHALL refuse with `invalid_runtime_input`/
  `invalid-value`.
- It SHALL recompute each choice with FR-188's `weighted_choice` from the
  seed, the trace index, the step and the choice index, and check that it
  selects the recorded operation, scheduled identity and values.
- It SHALL recompute the path probability exactly and compare it with the
  recorded one.
- For an `Undefined` witness, the replayed value SHALL be the first
  undefined evaluation along the replayed positions (ADR-018 UE-5), and it
  equals the recorded value when its position and cause are the recorded
  ones.
- When every check agrees and the replayed value equals the recorded value,
  it SHALL return `Reproduced`; when a recomputed draw or the path
  probability or the value disagrees, `Inconclusive(ReplayParity)`.
- The replay outcome of a witness SHALL not change the item's `measured`
  record.

### The qualitative claim

- A sampled witness SHALL convert to an ADR-018 `TemporalCounterexample`
  over the same subject (the initial state and binding that the verdict's
  provenance records for its test index, QSpec FR-410, and its steps) for
  the TP-2 claim of its bound direction (ADR-024 SV-7): "`E` on every
  behaviour" for a `>= θ` bound, "not `E` on every behaviour" for a
  `<= θ` bound, the PF-4 event's form for a quantile, and the fraction
  comparison on every behaviour for a mean of a fraction. FR-128 SHALL
  replay it as one.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-193-AC-1 | The `2 ms` claim of ADR-024 §7.2 with seed 7 and `max_witnesses` 3 keeps three witnesses in increasing trace-index order, each with latency above `2 ms` and a path probability equal to the product of its steps' FR-187 probabilities. In a witness whose first attempt drew `d = 3 ms`, `outcome = Ok`, the `request` step has probability 1 and that `attempt` step `343/5000`. | Test (TC-628) |
| FR-193-AC-2 | Each kept witness replays `Reproduced`. The same witness with `d` changed to `2 ms` (outside the support) refuses `invalid_runtime_input`/`invalid-value`; with the seed changed so a recomputed draw selects `1 ms` it settles `Inconclusive(ReplayParity)`; with its path probability changed it settles `Inconclusive(ReplayParity)`. The item's record stays `measured`, `rejected` in each case. | Test (TC-628) |
| FR-193-AC-3 | A witness converted to a `TemporalCounterexample` for the TP-2 claim "`M <= 2 ms` on every activated behaviour" replays through FR-128 and settles that claim `refuted`. For `probability <= 1/100 [eventually[0,1000] holds(not n.healthy)]` with `alpha 0.01, beta 0.01, indifference 0.005` over `Health` under `Steady` (ADR-024 §7.1), a witness is a sample with a fault; it converts to a counterexample for "not `eventually[0,1000] holds(not n.healthy)` on every behaviour" and settles that claim `refuted`. `LongRun` keeps no witness. | Test (TC-628) |
| FR-193-AC-4 | FR-189-AC-6's undefined sample is kept as a witness with value `Undefined(UndefinedEvaluation)` and replays `Reproduced`; the same witness with its cause changed to `precondition-false` settles `Inconclusive(ReplayParity)`. | Test (TC-640) |

## Dependencies

- ADR-024 SV-6 to SV-8; ADR-018 CX-1 to CX-3.
- [FR-098](FR-098-execute-a-replay-request.md),
  [FR-128](FR-128-replay-a-model-counterexample.md),
  [FR-188](FR-188-draw-weighted-choices-with-the-qspec-sampler.md),
  [FR-189](FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md).

## References

- QSpec half, which owns the sampled-witness wire and its replay checks:
  QSpec FR-410 (Linear STD-137).
- Owning ticket: Linear QSL-371.
