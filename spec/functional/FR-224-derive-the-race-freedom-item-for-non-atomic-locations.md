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
