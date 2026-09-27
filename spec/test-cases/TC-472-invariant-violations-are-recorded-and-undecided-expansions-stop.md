---
id: TC-472
title: "Invariant-violating successors are recorded, and undecided expansions stop the run"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: verifies
---
# TC-472: Invariant-violating successors are recorded, and undecided expansions stop the run

## Description

Verify that a state violating an invariant stays in exploration and is
recorded, that undecided invariants are findings, that undecided successor
sets stop the run incomplete, initial-state admission, requires-bound, and
sampled-trace findings and replay.

Scope: FR-120-AC-5, FR-120-AC-6, FR-120-AC-7, FR-120-AC-8.

## Test Procedure

Integration tests in `qsl-eval/tests/it/` through `ModelSystem::new`,
`explore_request`, `sample_request` and `replay`, with TC-471's package and
compile path. Unless a step says otherwise, the meter budget is the FR-100
default, `max_candidates` is 1,000, and FR-101's `Limits` are 100 each.

1. `increment` with `pre CanInc`, `post Inc` and `invariant Small … at
   current { self.value < 2 }`: explore. Then add `reset()` (frame
   `modifies value`, post `self.value = 0`) and explore.
2. `increment` and `Small`, no `pre` or `post` clause, meter budget 0:
   explore. Then add `CanInc`, budget 0: explore. Then `increment` with no
   clause and `max_candidates` 2: explore.
3. `ModelSystem::new` with, in turn: an initial snapshot whose
   `observation` is `pre`; one anchored `{handler, validate}`; one holding
   `c3`; a universe `[c1, c1]`; with an object type `Entry` (no fields)
   and its population `archive` added to the package, a universe `[a1]` for it and a snapshot
   that omits it.
4. `explore_request` and `sample_request` with no universe for `counters`,
   and then with `setTo(n: Integer)`, on a `ModelSystem` wrapper that
   records every method call.
5. Step 1's first package: `sample_request` with seed `424242`, trace `0`,
   `max_steps` 4 and the `quire.simulation.sampler/v1` `1-draft.1`
   `DefinitionRef`; replay it; remove the `Small` finding from the recorded
   trace and replay it. Repeat step 1 and this step.

Tag the tests `#[trace("TC-472", "FR-120-AC-n")]`.

## Expected Results

- Step 1: `Exhaustive`, `Stats { states: 3, transitions: 2, depth: 2 }`,
  and `findings` is exactly `[StateFindings { state: <v2's digest>, depth:
  2, findings: [InvariantViolated { Small, c1 }] }]`. With `reset`:
  `Exhaustive`, 3 states, 5 transitions, one of them `v2 → v0`, and the
  same one `findings` entry.
- Step 2: first, `Exhaustive` with 3 states, and `findings` has one entry
  per state, `InvariantUndetermined { Small, c1, <Incomplete,
  resource_exhausted/insufficient-next-charge> }`. Second,
  `Outcome::Stopped` with cause `resource_exhausted`/
  `insufficient-next-charge`, frontier `[<s0's digest>]`, category
  incomplete. Third, `Stopped` with the same cause.
- Step 3: `wrong_snapshot`/`wrong-observation`; `wrong_snapshot`/
  `wrong-anchor`; `invalid_runtime_input`/`invalid-value` naming
  `counters` and `c3`; `invalid_runtime_input`/`conflicting-identity`;
  `Incomplete` with `incomplete_population`/`incomplete-scope` naming
  `archive`.
- Step 4: `NotSimulated::RequiresBound` naming the `counters` population
  domain, then the parameter `n`; no `initial`, `key` or `successors` call.
- Step 5: the trace's `findings` has `Small`'s finding at the step whose
  state is `v2`; the trace replays; the edited trace refuses
  `ReplayError::FindingMismatch` at that step. The repeated runs give equal
  `Exploration`s and equal traces.

## Status

Planned (QSL-274).
