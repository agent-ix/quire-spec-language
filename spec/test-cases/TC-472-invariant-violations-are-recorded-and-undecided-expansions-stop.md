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
recorded with its anchor, that undecided invariants and contracts are
findings, that undecided successor sets stop the run incomplete,
initial-state admission, requires-bound, and sampled-trace findings, stops
and replay.

Scope: FR-120-AC-5, FR-120-AC-6, FR-120-AC-7, FR-120-AC-8, FR-120-AC-9.

## Test Procedure

Integration tests in `qsl-eval/tests/it/` through `ModelSystem::new`,
`explore_model`, `sample_model` and FR-101's `replay`, with TC-471's package
and compile path. Unless a step says otherwise, the meter budget is the
FR-100 default, `max_candidates` is 1,000, FR-101's `Limits` are 100 each,
and the sampler is the `quire.simulation.sampler/v1` `1-draft.1`
`DefinitionRef`.

1. `increment` with `pre CanInc { self.value < 2 }`, `post Inc` and
   `invariant Small … at current { self.value < 2 }`: explore. Then add
   `reset()` (frame `modifies value`, post `self.value = 0`) and explore.
2. `increment` and `Small`, no `pre` or `post` clause, meter budget 0:
   explore. Then add `CanInc`, budget 0: explore. Then `increment` with no
   clause and `max_candidates` 2: explore.
3. `ModelSystem::new` with, in turn: an initial snapshot whose
   `observation` is `pre`; one anchored `{handler, validate}`; one whose
   anchor kind is `other`; one holding `c3`; a universe `[c1, c1]`; a
   universe for population `test/counters/archive`, which the package does
   not declare; with an object type `Entry` (no fields) and its population
   `archive` added to the package, a universe `[a1]` for it and a snapshot
   that omits it.
4. `explore_model` and `sample_model` with no universe for `counters`, and
   then with `setTo(n: Integer)`, on a `ModelSystem` wrapper that records
   every method call.
5. Step 1's first package: `sample_model` with seed `424242`, trace `0` and
   `max_steps` 4; replay `ModelTrace.trace`; remove the `Small` finding from
   the recorded trace and replay it. Then the same package with budget 0:
   `sample_model`, replay the trace against that system, and replay it
   against the default-budget system. Repeat step 1 and this step.
6. Package `test/tallies` (object type `test/tallies/Tally` with
   `items: Sequence<Int[0, 2]>[1, 1]`, population `test/tallies/tallies`,
   operation `touch()` with an empty frame), unit type `Tiny = Int[0, 1]`,
   initial state `t1` with `items` `[2]`, universe `[t1]`. Expand `t1` with:
   `pre Low { sum<Tiny>(x in self.items: x) = 0 }` alone; `Low` and
   `pre Never { false }`; `Low` and `pre Aaa { false }`; `post LowPost` with
   `Low`'s body and no `pre` clause.

Tag the tests `#[trace("TC-472", "FR-120-AC-n")]`.

## Expected Results

- Step 1: `Exhaustive`, `Stats { states: 3, transitions: 2, depth: 2 }`,
  and `findings` is exactly `[StateFindings { state: <v2's digest>, depth:
  2, findings: [InvariantViolated { Small, c1, {handler, <increment>} }] }]`.
  With `reset`: `Exhaustive`, 3 states, 5 transitions, one of them
  `v2 → v0`, and the same one `findings` entry.
- Step 2: first, `Exhaustive` with 3 states, and `findings` has one entry
  per state, `InvariantUndetermined { Small, c1, <anchor>, <Incomplete,
  resource_exhausted/insufficient-next-charge> }`, where `s0`'s anchor is
  `{initialization, start}` and `v1`'s and `v2`'s is `{handler,
  <increment>}`. Second, `Outcome::Stopped` with cause `resource_exhausted`/
  `insufficient-next-charge`, frontier `[<s0's digest>]`, category
  incomplete. Third, `Stopped` with the same cause.
- Step 3: `wrong_snapshot`/`wrong-observation`; `wrong_snapshot`/
  `wrong-anchor`; `invalid_runtime_input`/`wrong-value-kind` with field
  `anchor`;
  `invalid_runtime_input`/`invalid-value` naming `counters` and `c3`;
  `invalid_runtime_input`/`conflicting-identity`;
  `invalid_runtime_input`/`wrong-role-mapping` naming
  `test/counters/archive`; `Incomplete` with `incomplete_population`/
  `incomplete-scope` naming `archive`.
- Step 4: `explore_model` returns `NotSimulated::RequiresBound` and
  `sample_model` `ModelSampleError::NotSimulated(RequiresBound)`, naming the
  `counters` root (`WireNodeId` from its `$defs.DeclarationKey` preimage),
  then the parameter `n` (`WireNodeId` from `{"operation":<setTo's
  preimage>,"parameter":"n"}`); no `initial`, `key` or `successors` call.
- Step 5: the trace ends `NoSuccessors` at `v2`; its `findings` has
  `Small`'s finding at step 2; it replays; the edited trace refuses
  `ReplayError::FindingMismatch` at step 2. With budget 0 the trace ends
  `StopReason::Stopped(resource_exhausted/insufficient-next-charge)` at
  step 0 and replays against its own system; against the default-budget
  system it refuses `ReplayError::Stopped { step: 0, recorded:
  Some(resource_exhausted/insufficient-next-charge), replayed: None }`. The
  repeated runs give equal `Exploration`s and equal `ModelTrace`s.
- Step 6: `Low` alone: no `touch` successor, and `t1` has
  `ContractUndetermined { <touch>, none, Low, Undefined(SumOutOfDomain) }`.
  `Low` with `Never`, and `Low` with `Aaa`: no `touch` successor and no
  finding. `LowPost`: no `touch` successor, and `t1` has
  `ContractUndetermined { <touch>, Some(<t1's digest>), LowPost,
  Undefined(SumOutOfDomain) }`.

## Status

Planned (QSL-274); runnable once QSL-289 lands.
