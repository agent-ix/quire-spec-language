---
id: FR-166
title: "Sample witnesses for possible claims before exploration (EN-1 phase 0)"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-020
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-165
    type: depends_on
---
# FR-166: Sample witnesses for possible claims before exploration (EN-1 phase 0)

## Description

When a request holds a `possible` item, QSL's layer-5 `model_check` SHALL
draw seeded random walks for it before exploring the subject (ADR-022 GE-2): `witness_samples`
walks per initial state, with FR-101's sampler. A walk that reaches a state
where the target holds is a witness for its initial state. Phase 0 is on by
default, with a published default of 64 walks, and a request turns it off by
setting `witness_samples` to 0. Every walk is named by its
`SampleProvenance`, so a run is reproducible from its seed. Sampling only
finds witnesses sooner: the item settles `proved` only after exploration
has ruled out a reachable undefined evaluation of the target (ADR-022 GV-1,
RU-5).

## Use case

A verification operator requests `possible ReachesTwo` with the default
limits. Before any exhaustive exploration, a few seeded walks reach a state
where `versionNumber` is 2. Exploration then checks every reachable state,
finds no undefined evaluation of the target, and the claim settles `proved`
from the sampled witnesses. The result names the walk that found each
witness, by seed and trace index, so the operator can see which walk found
it and can rerun it. When a state budget stops a larger run first, the
result says that a witness was found and well-definedness is unchecked.

## Inputs

```rust
pub struct ModelCheckLimits {
    pub limits: Limits,                 // FR-101: max_states, max_depth, max_transitions
    pub max_automaton_states: u64,      // FR-126
    pub witness_samples: u64,           // default 64; 0 turns phase 0 off
}

pub struct ModelCheckRequest<'a> {
    // FR-126's members, plus:
    pub seed: Option<u64>,              // None runs under DEFAULT_WITNESS_SEED
}
```

- `DEFAULT_WITNESS_SEED: u64 = 0`, the published default seed of QSpec
  FR-392 (ADR-022 QS-7).
- The subject (FR-125), its `ModelSystem` (FR-120), and each `Possible` item
  (FR-165) with its instances.
- The sampler's `DefinitionRef` from the ecosystem lock, as FR-101's
  `sample_request` takes it.

## Outputs

- For each `Possible` item instance and each initial state: a sampled
  witness, or none. A sampled witness is a `ModelPath` (FR-170) from that
  initial state, ending at the first visited state where the target holds,
  with source `Sampled(SampleProvenance{seed, trace, sampler, stopped})`.
- The run's seed, recorded in each item's terminal record (FR-169).

## Behavior

- The engine SHALL run phase 0 for every `Possible` item instance of the
  subject before phase 1 (FR-167), and for no other state-graph form.
- For initial state `i` (its index in the subject's initial list) and walk
  `r` in `0..witness_samples`, the engine SHALL call FR-101 `sample_request`
  on a view of the subject's `ModelSystem` whose only initial state is
  initial state `i`, with the request's seed, trace index `i ×
  witness_samples + r` and step ceiling `max_depth`.
- The engine SHALL compute each trace index with checked arithmetic. If it
  overflows `u64`, then the engine SHALL stop before the first walk and
  return `Stopped(ResourceExhausted, WitnessSamples)`, naming
  `witness_samples`, its value and the request member that sets it.
- The engine SHALL evaluate the target, under the instance's binding,
  through the one clause evaluator (FR-107) at each state the walk visits,
  position 0 included. The first state where it holds SHALL end the walk,
  and the walk's prefix up to that state SHALL be the witness for initial
  state `i`.
- A walk that ends without visiting a target state SHALL decide nothing.
- A walk that visits a state where the target evaluates `Undefined` SHALL
  end there and decide nothing; exploration finds and settles the undefined
  evaluation (FR-168).
- For each instance and initial state, the engine SHALL keep the witness of
  the lowest walk index `r` that found one, and SHALL draw no further walk
  for that instance and initial state.
- Phase 0 SHALL NOT settle an item. Exploration (FR-167) SHALL run for
  every `Possible` item whatever phase 0 found, and FR-168 SHALL decide the
  item from the explored graph and the sampled witnesses.
- Phase 1 SHALL keep each sampled witness and search for an explored
  witness only for the initial states that have none.
- When `witness_samples` is 0, the engine SHALL draw no walk.
- When the request names no seed, the engine SHALL run under
  `DEFAULT_WITNESS_SEED`.
- The engine SHALL record the seed the run used in each item's terminal
  record, whether the request named it or not.
- The engine SHALL read the cancellation poll before each walk, and a
  `true` poll SHALL stop the run as FR-126 states.
- Phase 0's walks SHALL be functions of the subject, the item, the limits
  and the seed.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-166-AC-1 | Over ADR-022 §7.1's subject with default limits and no seed, each `ReachesTwo` instance gets a sampled witness from `(0, 0)` whose last state is the first visited state where the bound config's `versionNumber` is 2, whose source is `Sampled` with seed 0 and a trace index below 64, and phase 1 searches for no explored witness for the item. | Test (TC-591) |
| FR-166-AC-2 | The same request with `witness_samples` 0 draws no walk and every witness comes from phase 1. `ReachesThree` with the default draws 64 walks per instance, finds no witness, and leaves the item to phase 1. | Test (TC-591) |
| FR-166-AC-3 | Over the §7.1 unit with two initial snapshots, `(0, 0)` and `(1, 1)`, the witness for initial state 1 has a trace index in `64..128`. A request with seed 7 records seed 7; two requests with no seed give byte-equal witnesses and record the default seed. | Test (TC-591) |
| FR-166-AC-4 | `witness_samples` `u64::MAX` over the two-snapshot subject returns `Stopped(ResourceExhausted, WitnessSamples)` before any walk, naming the limit and its value; a poll that returns `true` stops the run before the first walk. | Test (TC-591) |
| FR-166-AC-5 | Over §7.1's subject with default limits, `possible 2 / (2 - c.versionNumber) = 2` for `c = a` gets a sampled witness ending at a node with `va = 1`, and the item does not settle from it: exploration runs to completion and the item settles `refuted` with cause `UndefinedEvaluation` at `(2, 0)` (FR-168-AC-7). `ReachesTwo` under the same limits settles `proved` with its sampled witnesses only after exploration completes with no open node. | Test (TC-613) |
| FR-166-AC-6 | `ReachesTwo` with default `witness_samples` and `max_states` 2 gets a sampled witness for each instance, the exploration stops at `max_states`, and the item settles `inconclusive`, `WellDefinednessUnchecked`, never `proved` (FR-169-AC-8). | Test (TC-614) |

## Dependencies

- ADR-022 §3 GE-2 and "Determinism", §4 GV-1, §10 RU-4 and RU-5; ADR-014 TR-1
  (`SampleProvenance`).
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md)
  (`sample_request`, `SampleProvenance`),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (`ModelSystem`), [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (`ModelCheckLimits`, the request, cancellation),
  [FR-165](FR-165-check-state-graph-claims-at-s3.md) (`Possible`).
- FR-167 runs phase 1; FR-169 settles; FR-170 replays the witness.
- QSpec FR-392 owns `witness_samples` with its default 64, the seed with its
  default 0, and the settlement method in the terminal record (ADR-022
  QS-7).

## References

- ADR-022. QSpec half: QSpec FR-392 (Linear STD-135; ADR-022 QS-7).
