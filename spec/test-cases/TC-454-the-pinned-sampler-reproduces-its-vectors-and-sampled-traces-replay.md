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

1. Draw steps 0 to 4 at seed `424242`, trace `0`, `n = 5`, and record the
   step-0 preimage bytes and digest.
2. Draw steps 0 to 4 at seed `424242`, trace `1`, `n = 5`, and at seed
   `424242`, trace `0`, `n = 3`.
3. Sample a chain where every state has one successor, counting digests.
   Then sample, at seed `424242`, trace `0`, a system whose states at steps
   0 and 1 have one successor and whose state at step 2 has five.
4. Sample a state with no successors.
5. Sample one model twice with equal seed, trace index and `DefinitionRef`;
   then with a different seed; then with a different trace index.
6. Sample a system that lists three distinct initial states and repeats
   one of them, at trace indices 0 to 3.
7. Replay each trace from step 5 against its system, and a trace through a
   state with two successors that share a transition identity. Replay a
   trace from step 6 that starts at a non-first initial state. Then replay
   the step 5 trace with its initial digest, one step's transition, and one
   step's recorded digest altered in turn.
8. Call `sample_request` with a `DefinitionRef` whose identity is
   `quire.simulation.sampler/v2`, then one whose version is `1-draft.2`,
   each on a system that records every method call.
9. Call `sample_request` on a system with no initial state. Sample the chain
   0 → 1 → 2 with `max_steps` 1.

## Expected Results

- Step 1: indices `0, 0, 4, 4, 4`; preimage
  `{"draw":"0","seed":"424242","step":"0","trace":"0"}`; digest
  `cb7d4b3b8b8491d8310ccc1e07c3ee236d3fe86cf713d1851533a85ef6682622`
  (QSpec TC-210's vector).
- Step 2: `1, 4, 3, 4, 0` and `2, 1, 0, 2, 1`.
- Step 3: index 0 at every step and no digest computed. The mixed system
  selects index `4` at step 2, the step counter having advanced over the two
  steps with no draw; a counter that skips them selects `0`.
- Step 4: the trace ends with `StopReason::NoSuccessors` and no draw.
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
