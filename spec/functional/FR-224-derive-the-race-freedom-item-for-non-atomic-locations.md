---
id: FR-224
title: "Admit non-atomic shared locations and derive the race-freedom item"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-025
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-221
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-222
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-438
    type: depends_on
---
# FR-224: Admit non-atomic shared locations and derive the race-freedom item

## Description

Under a weak model, `Tso` and `Ra` SHALL explore a non-atomic access and
carry its race summary as QSpec FR-438 states (ADR-025 DR-1, DR-3). The
request writer SHALL add one derived **race-freedom item**,
`always holds(not raced)`, for each distinct subject among a request's
items whose resolved model is weak and that has a non-atomic shared access
(ADR-025 DR-4). EN-1 SHALL decide it and report a violation as a
counterexample of kind `Race{location, earlier, later}` (ADR-025 DR-5).

## Use case

A verification operator keeps data in a plain field guarded by a flag, as
real code does. They want to know, once per subject, whether any
interleaving lets two threads touch the data with no happens-before between
them, which is undefined behaviour in Rust and C11, and to see the two
accesses that race, while every other claim keeps its own verdict.

## Inputs

- The classified accesses (FR-220) and the `Tso` or `Ra` component (FR-221,
  FR-222).
- For the request writer: the request's temporal items over protocol
  subjects with their resolved models (FR-219).

## Outputs

- The race summary in the memory component: per location, the thread of its
  last write and whether it was atomic, per thread whether its last read was
  atomic, and per happens-before carrier the Booleans DR-3 names.
- A `RaceFreedom` request item per subject, with property form
  `ReachableInvariant`, requirement (`temporal-satisfaction`, `Unbounded`)
  and an obligation identity of the subject, the resolved models and the
  kind `race-freedom`.
- `CounterexampleKind::Race{location, earlier, later}` on
  `TemporalCounterexample`.

## Behavior

### Race summary

- `Tso` and `Ra` SHALL carry the race summary of QSpec FR-438 in the memory
  component, and SHALL set `raced` at the post-state of a step that
  completes a data race as QSpec FR-438 defines one.
- `Ra` SHALL hold each carrier's Booleans in its view, as the view's `race`
  member, and SHALL encode the race-summary entry in QSpec FR-438's typed
  form.
- `Ra` SHALL decide `raced` at a step from its pre-state, then set the
  location's write Boolean, for a write, or the accessing thread's read
  Boolean, for a read, true in the accessing thread's `cur` and `acq` views
  and false in every other view, and only then apply QSpec FR-437's access
  rule. An RMW whose read part is `acquire` or `seq_cst` SHALL decide its
  write part only after joining the race map of the message it reads into
  `cur`.
- `Ra` SHALL copy the Booleans with a view assigned from another view (a
  message view or a release view from `cur`, a branch's views at entry, the
  view a `send` carries), and SHALL or them where views join.
- `Tso` SHALL hold each thread's `cur`, `acq` and per-location `rel`
  summaries, each buffered element's, each memory value's and each queue
  entry's race map in QSpec FR-438's typed form, SHALL decide and update them
  in the same order as `Ra`, and SHALL apply FR-438's `tso` carrier rules: a
  `release` or `seq_cst` store's element takes `cur`'s map and sets `rel` for
  its location to it, a `relaxed` or non-atomic store's takes `rel`'s; a load
  joins the map it reads into `acq`, and an `acquire` or `seq_cst` load also
  into `cur`; an RMW reads as such a load and writes as such a store, its
  written value's map also joining the one it read, and decides its write
  part as `Ra` does; `flush` moves the element's map to the memory value; fences,
  fork, join, `send` and `receive` assign and join summaries as `Ra` does
  views.

### The item

- The request writer SHALL add one `RaceFreedom` item per distinct subject
  whose resolved model is weak and that has a non-atomic shared access, and
  none for any other subject.
- The item SHALL be routed, negotiated, checked and settled as a
  `ReachableInvariant` item (FR-126): V-1 with no race, V-4 with a race
  counterexample, and V-5, V-6 (including `MemoryBoundReached`) and V-7 as
  for any TP-1 item.
- EN-1 SHALL decide the item in its first phase, evaluating `raced` at each
  step from the summary.
- A race counterexample SHALL be the canonical finite prefix to the first
  step that completes a race, with kind `Race{location, earlier, later}`,
  `earlier` the position of the last conflicting access the summary names
  and `later` the completing step.
- A race SHALL change no authored claim's verdict: non-atomic accesses
  explore by DR-1 for every claim.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-224-AC-1 | ADR-025 §10's message-passing shape (`data := 1; flag := 1` against `r1 := flag`, then `r2 := data` only in the case of a `choice` on the flag load's binder that read 1) with `data` non-atomic and a `release` flag store and `acquire` flag load, under `ra`: the request carries one `RaceFreedom` item, which settles `proved`. | Test (TC-669) |
| FR-224-AC-2 | The same shape with `relaxed` flag accesses settles the item `refuted` under `ra` with kind `Race{location: data, earlier, later}`, `earlier` the position of `data := 1` and `later` the position of `r2 := data`; under `tso` it settles `refuted` with the same kind and location. | Test (TC-669) |
| FR-224-AC-3 | `SB`, whose shared accesses are all atomic, under `tso` carries no `RaceFreedom` item; the AC-1 shape under `sc` carries none. Two items over one subject give one `RaceFreedom` item, whose identity differs between the `tso` and `ra` requests and from every authored item's. | Test (TC-669) |
| FR-224-AC-4 | In the AC-2 shape under `ra`, the authored claim `always holds(not (r1 = 1 and r2 = 0))` settles `refuted` with a counterexample of kind `Formula`, the same verdict as with `data` atomic and `relaxed`. | Test (TC-669) |
| FR-224-AC-5 | In the AC-1 shape under `ra`, the state after the writer's `data := 1` holds the race-summary entry for `data` with value `{"last_write":{"thread":<writer>,"atomic":false},"last_reads":{}}`, and every view's `race` member maps `data` to `{"write":true,"reads":{}}` in the writer's `cur` and `acq` views and to `{"write":false,"reads":{}}` in every other view. The flag message the writer then stores has a view whose `race` member maps `data` to `"write":true` with a `release` store and `"write":false` with a `relaxed` one. An `acquire` load of that `release` message leaves the reader's `cur` mapping `data` to `"write":true`. With the flag passed instead by a `send` and `receive`, the queue entry's `view` member is the writer's `cur` view with `data` at `"write":true`, and the item settles `proved`. `SB`, whose accesses are all atomic, has no race-summary entry and no view `race` member. Each value equals QSpec FR-438-AC-6's. | Test (TC-669) |
| FR-224-AC-6 | In the AC-1 shape under `tso`, the state after the writer's `data := 1` holds the `data` race-summary entry of FR-224-AC-5, the writer's `race-summaries` entry mapping `data` to `"write":true` in `cur` and `acq` and `"write":false` in `rel` for `flag`, and the buffered `data` element's `race` member mapping it to `"write":false`. The buffered `flag` element, and after `flush` the `race-memory` entry for `flag`, map `data` to `"write":true` with a `release` store and `"write":false` with a `relaxed` one. The `RaceFreedom` item settles `proved` with the `release` store and `acquire` load, and with the flag passed by a `send` and `receive`, and `refuted` with the `relaxed` flag (FR-224-AC-2's `tso` half). Each value equals QSpec FR-438-AC-7's. | Test (TC-669) |
| FR-224-AC-7 | A protocol whose thread `a` loads `x` non-atomically and then stores `x := 1` with `release`, and whose thread `b` runs `fetch_add(1)` on `x` with `ordering acquire`, settles its `RaceFreedom` item `proved` under `ra` and under `tso`; with the RMW `ordering relaxed` it settles `refuted` with `Race{location: x, earlier, later}`, `earlier` `a`'s load and `later` the RMW. Each verdict equals QSpec FR-438-AC-8's. | Test (TC-669) |

## Dependencies

- ADR-025 §14 DR-1 to DR-5, DR-7, DR-8; ADR-018 DL-3 and DL-4 (the derived
  item pattern and counterexample `kind`); ADR-013 O-09, O-20.
- [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md)
  (the derived-item pattern), [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md),
  [FR-097](FR-097-classify-claim-extent-and-write-bounded-requests.md) (the
  request writer), FR-219, FR-220, FR-221, FR-222.
- QSpec FR-438 owns data races, the race summary, the race-freedom item,
  its `Race` counterexample kind and its place in the request.

## References

- Owning ticket: Linear QSL-372. QSpec half: QSpec FR-438 (Linear STD-138).
