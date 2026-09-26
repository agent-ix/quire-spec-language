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

Verify `quire.simulation.sampler/v1`, sampled-trace reproducibility and
replay. Scope: FR-101-AC-3, FR-101-AC-4 and FR-101-AC-5.

## Test Procedure

1. Draw steps 0 to 4 at seed `424242`, trace `0`, `n = 5`, and record the
   step-0 preimage bytes and digest.
2. Draw steps 0 to 4 at seed `424242`, trace `1`, `n = 5`, and at seed
   `424242`, trace `0`, `n = 3`.
3. Sample a chain where every state has one successor, counting digests.
4. Sample a state with no successors.
5. Sample one model twice with equal seed, trace index and `DefinitionRef`;
   then with a different trace index.
6. Sample a system with three initial states at trace indices 0 to 3.
7. Replay each trace from step 5 against its system. Then replay it with
   its initial key, one step's transition, and one step's recorded key
   altered in turn.

## Expected Results

- Step 1: indices `0, 0, 4, 4, 4`; preimage
  `{"draw":"0","seed":"424242","step":"0","trace":"0"}`; digest
  `cb7d4b3b8b8491d8310ccc1e07c3ee236d3fe86cf713d1851533a85ef6682622`
  (QSpec TC-210's vector).
- Step 2: `1, 4, 3, 4, 0` and `2, 1, 0, 2, 1`.
- Step 3: index 0 at every step and no digest computed.
- Step 4: the trace ends with `StopReason::NoSuccessors` and no draw.
- Step 5: the two equal runs give equal traces and equal provenance, which
  records the seed, the trace index and the `DefinitionRef`; the trace-index
  change changes the recorded provenance.
- Step 6: traces 0, 1, 2 and 3 start at canonical initial states 0, 1, 2
  and 0.
- Step 7: the unaltered trace replays; the altered ones refuse with
  `UnknownInitial`, `MissingTransition` and `KeyMismatch`, each at the
  altered step.

Tag each test `#[trace("TC-454", "FR-101-AC-n")]` with its AC.

## Status

🚧 Planned (QSL-272). The replay tests in
`qsl-eval/tests/it/finite_simulation.rs` back step 7 today under QSpec ids
and are retagged. `counter_sampler_produces_a_pinned_index_sequence` and
the `CounterSampler`-seeded tests are replaced by steps 1 to 6.
