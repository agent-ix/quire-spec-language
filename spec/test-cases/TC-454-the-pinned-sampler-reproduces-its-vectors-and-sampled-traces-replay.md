---
id: TC-454
title: "The pinned sampler reproduces its vectors, and sampled traces replay"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: verifies
---
# TC-454: The pinned sampler reproduces its vectors, and sampled traces replay

## Description

Verify `quire.simulation.sampler/v1`, sampled-trace reproducibility, replay
and the sampler's refusals. Scope: FR-101-AC-3, FR-101-AC-4, FR-101-AC-5 and
FR-101-AC-10.

## Test Procedure

Steps 1 to 9 are integration tests in `qsl-eval/tests/it/` through the
public entries; `sample` and `Sampler` are `pub(crate)` (FR-101). Steps 1b
and 3b observe draw internals no public entry exposes, and are in-crate unit
tests in `qsl-eval/src/simulation/`.

1. Call `sample_request` with an empty `domains` set and the `quire.simulation.sampler/v1` `1-draft.1` `DefinitionRef`, seed `424242`, trace `0`, `max_steps` 5, on a system where
   every state has five successors with distinct transition identities, and
   read the selected index at each step from the trace's transitions.
   1b. In-crate: record the sampler's step-0 preimage bytes and digest at
   seed `424242`, trace `0`, `n = 5`.
2. As step 1 at seed `424242`, trace `1`; then on a system where every state
   has three successors, at seed `424242`, trace `0`.
3. Call `sample_request` with an empty `domains` set and the `quire.simulation.sampler/v1` `1-draft.1` `DefinitionRef`, seed `424242`, trace `0`, on a chain where every state has one
   successor; then on a system whose states at steps 0 and 1 have one
   successor and whose state at step 2 has five.
   3b. In-crate: count the digests the sampler computes on the one-successor
   chain.
4. Call `sample_request` with an empty `domains` set and the `quire.simulation.sampler/v1` `1-draft.1` `DefinitionRef` on a system whose initial state has no successors.
5. Call `sample_request` with an empty `domains` set and the `quire.simulation.sampler/v1` `1-draft.1` `DefinitionRef` twice on one model with equal seed and trace index; then with a
   different seed; then with a different trace index.
6. Call `sample_request` with an empty `domains` set and the `quire.simulation.sampler/v1` `1-draft.1` `DefinitionRef` at trace indices 0 to 3 on a system that lists three distinct
   initial states and repeats one of them.
7. Call `replay` (public) with each trace from step 5 against its system,
   and with a trace from `sample_request` with an empty `domains` set and the `quire.simulation.sampler/v1` `1-draft.1` `DefinitionRef` through a state with two successors that share
   a transition identity. Replay a trace from step 6 that starts at a
   non-first initial state. Then replay the step 5 trace with its initial
   digest, one step's transition, and one step's recorded digest altered in
   turn.
8. Call `sample_request` with an empty `domains` set and a `DefinitionRef`
   whose identity is `quire.simulation.sampler/v2`, then one whose version
   is `1-draft.2`, each on a system that records every method call.
9. Call `sample_request` with an empty `domains` set and the `quire.simulation.sampler/v1` `1-draft.1` `DefinitionRef` on a system with no initial state; then on the chain
   0 → 1 → 2 with `max_steps` 1.

## Expected Results

- Step 1: indices `0, 0, 4, 4, 4` (QSpec TC-210's vector).
- Step 1b: preimage `{"draw":"0","seed":"424242","step":"0","trace":"0"}`;
  digest `cb7d4b3b8b8491d8310ccc1e07c3ee236d3fe86cf713d1851533a85ef6682622`.
- Step 2: `1, 4, 3, 4, 0` and `2, 1, 0, 2, 1`.
- Step 3: index 0 at every step. The mixed system
  selects index `4` at step 2, the step counter having advanced over the two
  steps with no draw; a counter that skips them selects `0`.
- Step 3b: no digest computed.
- Step 4: the trace has no steps and ends with `StopReason::NoSuccessors`.
- Step 5: the two equal runs give equal traces and equal provenance, which
  records the seed, the trace index and the `DefinitionRef`; the seed change
  and the trace-index change each change the recorded provenance.
- Step 6: with `m = 3`, traces 0, 1, 2 and 3 start at canonical initial
  states 0, 1, 2 and 0.
- Step 7: the unaltered traces replay, the shared-identity step takes the
  successor whose digest matches, and the non-first start is found by its
  digest; the altered ones refuse with `UnknownInitial`, `MissingTransition`
  and `KeyMismatch`, each at the altered step and each carrying digests.
- Step 8: both return `NotSimulated::GeneratorMismatch` with the supplied
  `DefinitionRef`, and no `TransitionSystem` method and no draw runs.
- Step 9: `NotSimulated::EmptyInitial`; then a one-step trace with
  `StopReason::StepLimit`.

Tag each test `#[trace("TC-454", "FR-101-AC-n")]` with its AC.

## Status

🚧 Planned (QSL-272). FR-101's "Existing test disposition" table maps each
existing test to its step here. `multi_initial_sample_picks_the_sampler_selected_start`
contradicts FR-101-AC-4 and is replaced by step 6;
`counter_sampler_produces_a_pinned_index_sequence` is deleted with
`CounterSampler`, replaced by steps 1 to 3.
