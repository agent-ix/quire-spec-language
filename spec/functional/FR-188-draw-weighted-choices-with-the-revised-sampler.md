---
id: FR-188
title: "Draw weighted choices with the revised sampler"
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
# FR-188: Draw weighted choices with the revised sampler

## Description

FR-101's sampler SHALL draw a step of a probabilistic model as a sequence of
weighted choices (ADR-024 ST-2): one for the operation by workload weight,
one for the scheduled identity uniformly within the operation, then one per
random parameter by its weights in declared order. Each choice SHALL select
by exact integer weights with rejection, from the
`quire.simulation.sampler/v1` generator whose preimage gains a choice index
within the step, so every draw is exact, independent of the others and
reproducible from the seed. QSpec's revision of the generator definition
owns the preimage and the vectors.

## Use case

EN-4 samples a `Service` behaviour from seed 7, trace index 12. At each step
it draws the operation, then `d`, then `outcome`, and records the drawn
values. A replay of the same seed and trace index draws the same values;
with every weight 1 and one choice per step the draw equals FR-101's uniform
selection.

## Inputs

- A seed, a trace index, a step number, a choice index and a list of
  positive exact rational weights.

## Outputs

```rust
pub struct ChoicePoint { pub seed: Seed, pub trace: u64, pub step: u64, pub choice: u32 }
pub fn weighted_choice(at: ChoicePoint, weights: &[Rational]) -> usize;

pub fn sample_probabilistic(
    system: &ModelSystem, workload: &Workload, start: &ModelState,
    seed: Seed, trace: u64, positions: u64, poll: impl FnMut() -> bool,
) -> Result<SampledBehaviour, SampleStop>;

pub struct SampledStep { pub transition: TransitionId, pub post_digest: Digest, pub choices: Vec<u32>, pub random: Vec<Value>, pub probability: Rational, pub rewards: Vec<(Identifier, Value)> }
pub struct SampledBehaviour { pub trace: u64, pub start: StateKey, pub steps: Vec<SampledStep> }
pub enum SampleStop { Expansion(ExpansionStop), NotMarkov(NotMarkov), Cancelled }
```

## Behavior

- `weighted_choice` SHALL scale the weights by the least common multiple of
  their denominators to positive integers `n_1 … n_k` with total `N`, and
  select index `i` with probability exactly `n_i / N` by drawing from the
  generator at the choice point and rejecting draws outside the largest
  multiple of `N` below the generator's range, as the generator definition
  states.
- The generator's preimage SHALL be the canonical JSON object `{choice,
  draw, seed, step, trace}` of QSpec FR-181's sampler definition, with
  `choice` the 0-based choice index within the step, so two choices of one
  step use distinct preimages and QSpec TC-210's vectors reproduce.
- With unit weights and one choice per step, `weighted_choice` at choice
  index 0 SHALL select the index FR-101's uniform selection selects.
- `sample_probabilistic` SHALL start at the given state, with no draw
  selecting the start, and at each step draw choice 0 for the operation by
  workload weight over the enabled operations, choice 1 for the scheduled
  identity uniformly over the operation's enabled identities in canonical
  order, and choices 2, 3, … for the random parameters in declared order.
  It SHALL then take the step FR-187 gives for that identity and drawn
  vector, and record the step with its choices, its drawn values, its exact
  step probability and its rewards.
- At a terminal state the sampler SHALL take the terminal stutter step with
  no draw, so a window that reaches a terminal state continues on stutter
  steps (ADR-018 SM-4).
- A drawn identity and vector that FR-187 reports `NotMarkov` SHALL stop the
  sample with `SampleStop::NotMarkov`; an `ExpansionStop` SHALL stop it with
  its cause; a `true` poll with `Cancelled`.
- The sampled behaviour SHALL be a function of the system, the workload,
  the start, the seed, the trace index and the number of positions.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-188-AC-1 | Every weighted-sampler conformance vector of the revised generator definition reproduces exactly: for each (seed, trace, step, choice, weights) the selected index equals the vector's. Every QSpec TC-210 uniform vector, run as unit weights at choice index 0, selects the same index as FR-101. | Test (TC-623) |
| FR-188-AC-2 | Over the weights `(1/3, 1/6, 1/2)` the scaled integers are `(2, 1, 3)` with `N = 6`. For one seed and 60,000 consecutive trace indices at step 0, choice 0, every index is selected and the selection at each point equals the reference computation of the generator definition. Choices 0 and 1 of the same step use different preimages, so their draws differ for at least one point among the first 100. | Test (TC-623) |
| FR-188-AC-3 | Sampling `Service` under `Steady` from its initial state with seed 7, trace 12 and 11 positions twice gives equal behaviours. Every step records its choice indices, its drawn values inside their supports, a step probability equal to FR-187's for that transition, and its `duration`. A sample that reaches `Done` and then `Idle` continues; the `Health` variant with a non-unique post-state stops with `SampleStop::NotMarkov`. | Test (TC-623) |

## Dependencies

- ADR-024 ST-2, RP-1, QS-8; ADR-014 TR-1, TR-6.
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md)
  (generator, uniform selection, replay),
  [FR-187](FR-187-give-model-transitions-step-probabilities-and-rewards.md).

## References

- QSpec half, which owns the generator revision (weighted selection, the
  choice index in the preimage, the vectors): QSpec FR-181 and TC-210, with
  FR-408's step order (Linear STD-137).
- Owning ticket: Linear QSL-371.
