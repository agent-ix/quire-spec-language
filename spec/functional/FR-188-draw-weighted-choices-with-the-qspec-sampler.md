---
id: FR-188
title: "Draw weighted choices with the QSpec sampler"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-022
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-024
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-187
    type: depends_on
---
# FR-188: Draw weighted choices with the QSpec sampler

## Description

FR-101's sampler SHALL draw a step of a probabilistic model as a sequence of
weighted choices (ADR-024 ST-2): one for the operation by workload weight,
one for the scheduled identity uniformly within the operation, then one per
random parameter by its weights in declared order. Each choice SHALL select
by exact integer weights with rejection, from the
`quire.simulation.sampler/v1` generator, selected by its identity, whose
preimage carries a choice index within the step, so every draw is exact,
independent of the others and reproducible from the seed. A choice over one
item selects it without a draw and takes no choice index, so the indices
count the choices that draw. QSpec's generator definition owns the preimage,
the index rule and the vectors.

## Use case

EN-4 samples a `Service` behaviour from seed 7, trace index 12. At each step
it draws the operation, then `d`, then `outcome`, and records the drawn
values. A replay of the same seed and trace index draws the same values;
with every weight 1 and one choice per step the draw equals FR-101's uniform
selection.

## Inputs

- A seed, a trace index, a step number, a choice index and a list of
  positive exact rational weights.
- For a sample, the sampler's `DefinitionRef` (FR-101's `qsl_foundation::selection`).

## Outputs

```rust
pub struct ChoicePoint { pub seed: Seed, pub trace: u64, pub step: u64, pub choice: u32 }
pub struct TotalAboveDrawRange { pub total: Integer } // quire-exact; the scaled total N, above 2^256
pub fn weighted_choice(at: ChoicePoint, weights: &[Rational]) -> Result<usize, TotalAboveDrawRange>;

pub fn sample_probabilistic(
    system: &ModelSystem, workload: &Workload, start: &ModelState,
    sampler: &DefinitionRef, seed: Seed, trace: u64, positions: u64, cancel: &Cancel, // FR-276
) -> Result<SampledBehaviour, SampleStop>;

// SampledStep, SampledBehaviour and SampleStop are public items of qsl-eval's simulation module.
pub struct SampledStep { pub transition: TransitionId, pub post_digest: Digest,
    pub choices: Vec<u32>, // selected item index of each choice that drew, in choice-index order
    pub random: Vec<Value>, pub probability: Rational, pub rewards: Vec<(Identifier, Value)> }
pub struct SampledBehaviour { pub trace: u64, pub start: StateKey, pub steps: Vec<SampledStep> }
pub enum SampleStop {
    GeneratorMismatch { supplied: DefinitionRef }, // FR-101's refusal, before any draw
    Expansion(ExpansionStop), NotMarkov(NotMarkov),
    TotalAboveDrawRange(TotalAboveDrawRange),
    Cancelled(CancelCause),
}
```

## Behavior

- `weighted_choice` SHALL scale the weights by the least common multiple of
  their denominators to positive integers `n_1 … n_k` with total `N`, and
  select index `i` with probability exactly `n_i / N` by drawing from the
  generator at the choice point and accepting a draw `v` only when
  `v < N · ⌊2^256 / N⌋`, as the generator definition states, so `N = 2^256`
  accepts every draw.
- The sampler SHALL treat a choice over one item (one enabled operation,
  one enabled scheduled identity, a one-value support, or `weighted_choice`
  over one weight) as selecting index 0 with no digest and no choice index,
  and every other choice as taking the next choice index of its step, from 0
  at each step, so the next drawing choice takes the index the one-item
  choice would have taken, as the generator definition states.
- If the scaled total `N` exceeds 2^256, then `weighted_choice` SHALL refuse
  with `TotalAboveDrawRange` before computing any digest. The generator
  definition states no rule for this case (its acceptance test accepts no
  draw, so a draw would never end); the refusal is QSL's behaviour.
- If a choice of a step reports `TotalAboveDrawRange`, then
  `sample_probabilistic` SHALL stop with `SampleStop::TotalAboveDrawRange`.
- The sampler SHALL be selected by identity only: `sample_probabilistic`
  runs a supplied `DefinitionRef` whose identity is
  `quire.simulation.sampler/v1`, and refuses any other with
  `SampleStop::GeneratorMismatch` (FR-101's `NotSimulated::GeneratorMismatch`)
  before any draw.
- Draw `d` of choice `c` at trace `t`, step `s` under seed `k` SHALL be
  SHA-256 of the JCS object
  `{"choice":"c","draw":"d","seed":"k","step":"s","trace":"t"}`,
  each member a decimal string, as FR-101 and QSpec FR-181 state, so two
  choices of one step use distinct preimages and QSpec TC-210's vectors
  reproduce.
- With unit weights and one choice per step, `weighted_choice` at choice
  index 0 SHALL select the index FR-101's uniform selection selects.
- `sample_probabilistic` SHALL start at the given state, with no draw
  selecting the start, and at each step choose the operation by workload
  weight over the enabled operations, then the scheduled identity uniformly
  over the operation's enabled identities in canonical order, then the
  random parameters in declared order, each by the rule above. With several operations the operation is choice 0
  and the identity, when it has several candidates, choice 1; with one
  operation and one identity the first random parameter with several values
  is choice 0.
  It SHALL then take the step FR-187 gives for that identity and drawn
  vector, and record the step with the selected item index of each choice that drew,
  in choice-index order (a one-item choice records none), its
  drawn values (including those of one-item choices), its exact step
  probability and its rewards.
- `SampledStep` and `SampledBehaviour` SHALL be public items of `qsl-eval`'s
  simulation module, beside `sample_probabilistic`. `SampleStop` SHALL be
  one too, because it carries `NotMarkov` and `ExpansionStop`, `qsl-eval`
  outcomes that `quire-semantic-value`, a leaf with no dependency on
  `qsl-eval`, cannot name. None of the three is a `quire-semantic-value`
  item.
- At a terminal state the sampler SHALL take the terminal stutter step with
  no draw, so a window that reaches a terminal state continues on stutter
  steps (ADR-018 SM-4).
- A drawn identity and vector that FR-187 reports `NotMarkov` SHALL stop the
  sample with `SampleStop::NotMarkov`; an `ExpansionStop` SHALL stop it with
  its cause; a cancelled `Cancel` handle (FR-276) with `Cancelled`.
- The sampled behaviour SHALL be a function of the system, the workload,
  the start, the seed, the trace index and the number of positions.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-188-AC-1 | Every weighted-sampler conformance vector of the generator definition reproduces exactly: for each (seed, trace, step, choice, weights) the selected index equals the vector's. Every QSpec TC-210 uniform vector, run as unit weights at choice index 0, selects the same index as FR-101.  `sample_probabilistic` given a sampler `DefinitionRef` with another identity returns `SampleStop::GeneratorMismatch` before any draw, observed as a test-only digest counter (incremented once per SHA-256 digest the sampler computes) reading zero. | Test (TC-623) |
| FR-188-AC-2 | Over the weights `(1/3, 1/6, 1/2)` the scaled integers are `(2, 1, 3)` with `N = 6`. For one seed and 60,000 consecutive trace indices at step 0, choice 0, every index is selected and the selection at each point equals the reference computation of the generator definition. Choices 0 and 1 of the same step use different preimages, so their draws differ for at least one point among the first 100. | Test (TC-623) |
| FR-188-AC-3 | Sampling `Service` under `Steady` from its initial state with seed 7, trace 12 and 11 positions twice gives equal behaviours. Every step records its selected item indices, its drawn values inside their supports, a step probability equal to FR-187's for that transition, and its `duration`. A sample that reaches `Done` and then `Idle` continues; the `Health` variant with a non-unique post-state stops with `SampleStop::NotMarkov`. | Test (TC-623) |
| FR-188-AC-4 | QSpec TC-354's one-item vectors reproduce through `sample_probabilistic`, with the test-only digest counter. On `Lone` (one operation, one identity, no random parameter) a step selects item 0 of each one-item choice, leaves the digest counter at zero and records `choices = []`. On `Coin` (one operation, one identity, one random parameter whose support `false, true` in canonical value order has weights `(3, 1)`) at seed `424242`, trace 0, step 0 the parameter's choice is choice index 0 with preimage `{"choice":"0","draw":"0","seed":"424242","step":"0","trace":"0"}` (the TC-210 step-0 digest), and the digest's `v mod 4` is 2, below the first weight 3, so the choice selects index 0, `false`: the step records `choices = [0]` and `random = [false]`. `weighted_choice` over one weight returns 0 and the digest counter reads zero. | Test (TC-623) |
| FR-188-AC-5 | `weighted_choice` over weights whose scaled total is `2^256` accepts every draw and selects as the generator definition states. Over weights `(1, 2^256)` (scaled total `2^256 + 1`) it returns `TotalAboveDrawRange{total: 2^256 + 1}` and the digest counter reads zero. `sample_probabilistic` on `Wide` (an operation with a random parameter of weights `(1, 2^256)`) stops with `SampleStop::TotalAboveDrawRange` at that parameter's choice. | Test (TC-623) |
| FR-188-AC-6 | `SampledStep`, `SampledBehaviour` and `SampleStop` resolve as public items of `qsl-eval`'s simulation module, and the public items of `quire-semantic-value` include none of them. | Test (TC-623) |

## Dependencies

- ADR-024 ST-2, RP-1, QS-8; ADR-014 TR-1, TR-6.
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md)
  (generator, uniform selection, replay),
  [FR-187](FR-187-give-model-transitions-step-probabilities-and-rewards.md).

## References

- QSpec half, which owns the generator definition: QSpec FR-181 and TC-210,
  with FR-408's step order (Linear STD-137). The sampler definition
  (`quire.simulation.sampler/v1`: weighted selection, the choice index in
  the preimage, the one-item rule) and TC-354 (its vectors, SM-05 and SM-07
  for the one-item rule) are read from QSpec; neither states a rule for a
  scaled total above 2^256, so that refusal is QSL's.
- Owning ticket: Linear QSL-371.
