---
id: TC-474
title: "The engine records findings, stops on an expansion stop, and replays a stopped trace"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: verifies
---
# TC-474: The engine records findings, stops on an expansion stop, and replays a stopped trace

## Description

Verify FR-101's findings and `ExpansionStop` behaviour with test
`TransitionSystem`s: the `Stopped` outcome, its frontier order and its
category per cause, findings recording, `StopReason::Stopped`, and replay of
a stopped trace.

Scope: FR-101-AC-12, FR-101-AC-13, FR-101-AC-14, FR-097-AC-5 (`Stopped`).

## Test Procedure

Integration tests in `qsl-eval/tests/it/` through `explore_request`,
`sample_request` and `replay`, with an empty `domains` set, `Limits` of 100
each and the `quire.simulation.sampler/v1` `DefinitionRef`. A
test system is an integer graph whose `successors` returns a configured
`Expansion` or `ExpansionStop` per state.

1. Graph `0 → {1, 2}`, `1 → 3`, whose expansion of `1` returns
   `ExpansionStop` with `resource_exhausted`/`insufficient-next-charge`:
   explore. Repeat with `runtime_invariant`/`established-invariant-broken`.
2. Graph `0 → {1, 2}`, `1 → 3`, no stop, whose expansion of `2` returns
   finding `"f"`: explore. Repeat with `max_depth` 1. Then step 1's first
   system with finding `"g"` on `1`: explore.
3. Chain `0 → 1` whose expansion of `1` stops with `resource_exhausted`/
   `insufficient-next-charge`: sample with seed `1`, trace `0`,
   `max_steps` 4, then replay the trace against the same chain.
4. Replay step 3's trace against: the chain whose `1` does not stop; the
   chain whose `1` stops with `runtime_invariant`/
   `established-invariant-broken`.
5. Sample the non-stopping chain with `max_steps` 1 and replay the trace
   against the stopping chain.
6. Sample step 2's graph from `0` with seed `424246`, trace `0` and
   `max_steps` 2, then replay the trace with `"f"` removed from its
   findings. Seed `424246`'s step-0 draw preimage
   `{"choice":"0","draw":"0","seed":"424246","step":"0","trace":"0"}` hashes to a digest
   whose big-endian value is odd, so with `n = 2` it selects index 1, the
   successor `2` in canonical order.

Tag the tests `#[trace("TC-474", "FR-101-AC-n")]` and, for step 1's
categories, `#[trace("TC-474", "FR-097-AC-5")]`.

## Expected Results

- Step 1: `Outcome::Stopped { cause: resource_exhausted/
  insufficient-next-charge, frontier: [<1>, <2>] }`, category incomplete.
  With `runtime_invariant`: `Stopped`, same frontier, category internal
  failure.
- Step 2: `Exploration.findings` is `[StateFindings { state: <2>, depth: 1,
  findings: ["f"] }]`. With `max_depth` 1: `BoundReached{depth: 1}`
  with frontier `[<1>, <2>]`, and `findings` is `[]`, since the frontier
  states are unexpanded. With the stopping system, `findings`
  holds no entry for `1`.
- Step 3: the trace ends `StopReason::Stopped(resource_exhausted/
  insufficient-next-charge)` at step 1, and replay succeeds.
- Step 4: `ReplayError::Stopped { step: 1, recorded: Some(resource_exhausted/
  insufficient-next-charge), replayed: None }`; then `ReplayError::Stopped
  { step: 1, recorded: Some(resource_exhausted/insufficient-next-charge),
  replayed: Some(runtime_invariant/established-invariant-broken) }`.
- Step 5: the trace ends `StepLimit` at step 1; replay refuses
  `ReplayError::Stopped { step: 1, recorded: None, replayed:
  Some(resource_exhausted/insufficient-next-charge) }`.
- Step 6: the trace is `0 → 2`, ending `NoSuccessors` at step 1 with
  `"f"` in its findings; the edited trace refuses
  `ReplayError::FindingMismatch { step: 1 }`.

## Status

Planned.
