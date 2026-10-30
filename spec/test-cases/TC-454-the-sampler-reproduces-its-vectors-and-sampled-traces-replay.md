---
id: TC-454
title: "The sampler reproduces its vectors, and sampled traces replay"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: verifies
---
# TC-454: The sampler reproduces its vectors, and sampled traces replay

## Description

Verify QSL's conformance to QSpec TC-210's sampler vector, sampled-trace
reproducibility, replay and the sampler's refusals. Read the vector in place
from the QSpec checkout; do not copy its expected values here. Scope:
FR-101-AC-3, FR-101-AC-4, FR-101-AC-5 and FR-101-AC-10.

## Test Procedure

Steps 1 to 9 are integration tests in `qsl-eval/tests/it/` through the
public entries; `sample` and `Sampler` are `pub(crate)` (FR-101).

1. Under the conformance target, read QSpec TC-210's sampler vector from the
   QSpec checkout. Call `sample_request` on the vector's listed systems and
   inputs, and compare the selected indices with the vector.
   1b. (Retired: the sampler's draw internals are QSpec's generator
   definition and are observed only through the selected indices of step 1.)
2. As step 1 at seed `424242`, trace `1`; then on a system where every state
   has three successors, at seed `424242`, trace `0`.
3. Call `sample_request` with an empty `domains` set and the
   `quire.simulation.sampler/v1` `DefinitionRef`, seed `424242`, trace `0`,
   on a one-successor chain; then on a system whose states at steps 0 and 1
   have one successor and whose state at step 2 has five.
   3b. (Retired with step 1b: draw internals are not observed.)
4. Call `sample_request` with an empty `domains` set and the `quire.simulation.sampler/v1` `DefinitionRef` on a system whose initial state has no successors.
5. Call `sample_request` with an empty `domains` set and the `quire.simulation.sampler/v1` `DefinitionRef` twice on one model with equal seed and trace index; then with a
   different seed; then with a different trace index.
6. Call `sample_request` with an empty `domains` set and the `quire.simulation.sampler/v1` `DefinitionRef` at trace indices 0 to 3 on a system that lists three distinct
   initial states and repeats one of them.
7. Call `replay` (public) with each trace from step 5 against its system,
   and with a trace from `sample_request` with an empty `domains` set and the `quire.simulation.sampler/v1` `DefinitionRef` through a state with two successors that share
   a transition identity. Replay a trace from step 6 that starts at a
   non-first initial state. Then replay the step 5 trace with its initial
   state key and one step's transition altered in turn. (The altered recorded
   state key is retired with the `KeyMismatch` refusal of FR-101-AC-5; BLOCKED-BY QSL-1282 (owner: planner titan:QSL:plan): qsl-eval replay key compare in simulation/trace.rs.)
8. Call `sample_request` with an empty `domains` set and a `DefinitionRef`
   whose identity is `quire.simulation.sampler/v2`, on a system that records
   every method call.
9. Call `sample_request` with an empty `domains` set and the `quire.simulation.sampler/v1` `DefinitionRef` on a system with no initial state; then on the chain
   0 → 1 → 2 with `max_steps` 1.

## Expected Results

- Step 1: selected indices equal the corresponding sequence in QSpec
  TC-210, read from the QSpec checkout.
- Step 1b: retired.
- Step 2: the first system selects `4, 4, 1, 3, 4`; the second selects
  `1, 2, 0, 0, 1`.
- Step 3: index 0 at every step on the one-successor chain. The mixed system
  selects index `3` at step 2, showing the counter advances across the two
  steps without a draw; a counter that skips them selects `1`.
- Step 3b: retired.
- Step 4: the trace has no steps and ends with `StopReason::NoSuccessors`.
- Step 5: the two equal runs give equal traces and equal provenance, which
  records the seed, the trace index and the sampler identity; the seed change
  and the trace-index change each change the recorded provenance.
- Step 6: with `m = 3`, traces 0, 1, 2 and 3 start at canonical initial
  states 0, 1, 2 and 0.
- Step 7: the unaltered traces replay, the shared-identity step takes the
  successor by transition identity and then ordinal, and the non-first start is found by its
  state key; the altered ones refuse with `UnknownInitial` and
  `MissingTransition`, each at the altered step and each carrying state keys.
- Step 8: it returns `NotSimulated::GeneratorMismatch` with the supplied
  `DefinitionRef`, whose catalog code is `invalid_runtime_input`/
  `invalid-value`, and no `TransitionSystem` method and no draw runs.
- Step 9: `NotSimulated::EmptyInitial`; then a one-step trace with
  `StopReason::StepLimit`.

Tag each test `#[trace("TC-454", "FR-101-AC-n")]` with its AC.
