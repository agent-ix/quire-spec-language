---
id: FR-523
title: "Count temporal evidence work on thirteen named counters and keep a stopped instance apart from a stopped declaration"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/NFR-008
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-326
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-501
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-516
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-522
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: references
---
# FR-523: Count temporal evidence work on thirteen named counters and keep a stopped instance apart from a stopped declaration

## Description

The spine temporal evaluator SHALL count the work of one temporal evidence call
across thirteen named counters, each with a caller-set ceiling. When a ceiling is
reached, the evaluator stops with a typed `Incomplete` that names the counter,
and the result of every other call keeps its truth, basis and support.

This is the spine successor of NFR-008-AC-1 to AC-5 and of FR-052-AC-11. FR-516
names the counters and the exact-usage rule; this requirement defines what each
counter counts, its setting and the stop scope. The mapping from NFR-008's seven
counters and from the retired fourteen-dimension document limits of the native
temporal request is in "Counter mapping".

## Use case

A caller assesses three instances of one clause over a long recording, one call
each, sharing one work meter it sized. When the meter is too small for the third
call, the first two keep their verdicts, the third reports which charge ran out,
and no call reports a guess.

## Semantic authority and boundary

NFR-008 owns the reliability target (no truth after a stop; siblings keep
theirs). FR-255 owns setting names and defaults. FR-327 and FR-501 own the
`temporal.visit` charge. This requirement owns the counters' definitions and the
stop scope. The entry that applies them is `evaluate_temporal_evidence` (FR-516).
One call assesses one instance, which activates once (FR-516), so the siblings of a
stopped call are the results of other calls, which are values and do not change.

## Inputs

The thirteen ceilings, set through the settings of FR-255 (`temporal_evidence.*`)
or the library builder `TemporalEvidenceLimits`, and the call's `Meter` (FR-516).

## Outputs

`usage`: the thirteen counters' measured values (FR-516).

On a stop, an `Incomplete(LimitReached{limit, value, setting})` result (FR-516: `Ok`
with `activation = Unavailable` and `incomplete` set, never an error) with `limit`
the limit kind of FR-255, `value` the ceiling, `setting` the counter's setting name,
the assessed instance (when the stop is at the instance) and no truth value.

## Behavior

### Counters

Each counter is cumulative over the call, or a peak, as the table says. The table
fixes the unit of one charge; the `temporal.visit` count of one evaluation is
FR-501-AC-12's (one per operator node at its activation, one per `holds` atom
at each eligible position and one scan per position).

| Counter | Kind | What one unit is | Replaces |
| --- | --- | --- | --- |
| `formula_nodes` | peak | a distinct node of the clause's formula reachable from its root; a node shared by two parents counts once | `formula_nodes` |
| `positions` | cumulative | one scan charge of FR-501: one position read by one temporal operator node at one activation, at an observed position or an observed position's false extension; a scan at a silent sample is not counted | `positions` |
| `valuations` | cumulative | one atomic `holds` lookup, including one that finds no valuation, and one window position read for a window aggregate atom (FR-516 "Window aggregate atoms"), including one that finds no sample or an `Excluded` one | `valuations` |
| `instances` | peak | one instance created from a delivery or an origin in this call, the greatest number created by the call (FR-524); a retained instance, a delivery with a false guard and an instance whose activation faulted are not created and charge nothing | NFR-008 `instances` |
| `captures` | cumulative | one capture record retained, across all instances | `captures` |
| `support` | peak | the greatest number of `(position, atom)` pairs an unsettled activation retains, and the number of pairs in a settled decision support | `support`; NFR-008 `retention` |
| `history_span` | peak | the greatest interval upper bound of the clause, in ticks (FR-326 horizon) | `history_span`; NFR-008 `horizon` |
| `evaluation_steps` | cumulative | one charge per evaluated position visit at the `temporal.visit` charge point (FR-327), including one per delivery or origin resolved, one per capture established and one per evicted record consulted (FR-524); a charge made at a silent sample reaches the same meter point but is not an `evaluation_steps` unit (see "Evaluated position visit") | `evaluation_steps`; NFR-008 `visits` |
| `lineage` | peak | the length of the result's chain of predecessors (FR-526) | `lineage` |
| `visited` | cumulative | one unit of the usage of `formula_nodes`, `positions`, `valuations` and `captures`, and one evicted record consulted | `visited` |
| `request_bytes` | peak | one byte of the request's opaque byte members, in total (FR-516) | `input_bytes` |
| `result_bytes` | peak | one byte of the result's opaque byte members, in total | `output_bytes` |
| `member_bytes` | peak | one byte of the longest single opaque member | `string_bytes` |

- **Window aggregate visit.** An aggregate atom's lookup at a position is charged as a
  `holds` lookup is (one `temporal.visit` unit counted in `evaluation_steps`, one
  `valuations` unit), and each window position it then reads, oldest first, an
  `Excluded` one included and up to and including the first undefined one, is one
  `temporal.visit` unit counted in `evaluation_steps` and one `valuations` unit
  (QSpec FR-414-AC-8's one unit per window position visited). `visited` counts those
  `valuations` units by its own definition and takes no further unit; `positions` and
  `formula_nodes` are unchanged by a window read, and a window position that is no
  request position is not visited. A silent sample holds no window position.
- **Evaluated position visit.** A silent sample is a fixed-sample watermark
  sample with no observation (FR-504); it is not an observed position. Every
  `temporal.visit` charge FR-501-AC-12 lists (a temporal node at its activation, a
  `holds` lookup, one scan) that is made at an observed position, or at an
  observed position's false extension, is one evaluated position visit and counts
  in `evaluation_steps` (the scan also in `positions`). Every such charge made at
  a silent sample, whether a lookup, a scan or a node activation, is charged to
  the call's meter at the `temporal.visit` charge point, one meter unit each, and
  counts in neither `evaluation_steps` nor `positions`. A silent-sample `holds`
  lookup is a lookup that finds no valuation and also charges `valuations` and
  its `visited` unit. Admitting a fixed-sample trace with a watermark past its last
  observation additionally charges one meter unit at `temporal.visit` for each
  silent sample within the clause's reach, before that sample is materialized
  (FR-504 silent-sample admission): with `S` the seen horizon, `last` the last
  observation's coordinate and `Hf` the clause's forward reach in samples
  (FR-501), the call charges `min(S, last + Hf) - last` materialization units, and
  a silent sample beyond the reach is neither materialized nor charged. A budget
  the reach cannot afford ends `Incomplete` with nothing allocated. The
  materialization charges come first in the call, before the origin or delivery is
  resolved, and count in no counter.
- **Charge order.** FR-501-AC-12 fixes the charges and not their order. For a
  clause with a single temporal operator, the evaluator charges, in this order: the
  materialization charges, the origin or delivery step, the `Step` of each
  temporal operator at its activation, then the scan charges of the operator's
  candidate positions in position order (a `Scan` at an observed position, a
  silent scan at a silent sample), and then the `holds` lookups in position order
  (a `Lookup` at an observed position, a silent lookup at a silent sample). For a
  clause with several temporal operators, the charges follow the evaluator's
  demand-closure order, a breadth-first queue in which nested and sibling
  operators interleave, and this requirement states no other order for it. Every
  exact-ceiling vector of this specification (AC-10, AC-21, AC-24) uses a
  single-operator clause.
- The evaluator SHALL charge a counter only for work it performs.
- The evaluator SHALL keep the counters at thirteen: no counter is added for
  silent samples or for evicted records, whose visit is one hit charged on
  `evaluation_steps` and `visited` (FR-524).
- A refused charge SHALL NOT raise the counter's usage.
- When a call's usage of a counter equals its ceiling, the evaluator SHALL
  complete.
- When the usage a call would need exceeds a ceiling by one, the evaluator SHALL
  return `Incomplete` naming that counter.
- The evaluator SHALL NOT perform the operation whose charge is refused.
- The evaluator SHALL use each ceiling as given, above or below its default.
- The evaluator SHALL preserve a ceiling of zero.
- The evaluator SHALL NOT clamp a ceiling to another value (FR-255).
- After an `Incomplete`, a retry under sufficient ceilings from the same
  inputs SHALL give the result the first call would have given with those
  ceilings.
- An `Incomplete` result SHALL carry the request's `ledger` and `progress_ledger`
  unchanged, for a counter, meter, evicted-valuation, clause and lineage stop
  alike (FR-516).
- The evaluator SHALL charge the counter before it charges the meter.
- The evaluator SHALL report a denied counter by its name.
- When the meter denies a `temporal.visit` charge that every counter admitted,
  the evaluator SHALL return `Incomplete` at the `temporal.visit` charge point as
  FR-327 states.
- The evaluator SHALL walk the formula over an explicit heap stack (ADR-030 D-1),
  so that no limit bounds nesting depth and the thread's stack does not.

### Tick arithmetic

- Before it visits any position, the evaluator SHALL compute, for every
  activation anchor `t0`, the sum `t0 + Hf + m` of FR-501 with checked `u64`
  arithmetic.
- When a sum does not fit in `u64`, the evaluator SHALL return `Incomplete` naming
  `history_span` before it visits any position.
- The evaluator SHALL NOT wrap, saturate or narrow a tick sum.
- The tick domain is `u64`, which is the domain of FR-326's horizon and of
  FR-501's non-negative stamps.

### Stop scope

- The counters `formula_nodes` and `history_span` and the tick arithmetic are
  properties of the clause and its anchor. When one stops the call, the evaluator
  SHALL assess no instance and SHALL return the `Incomplete` result, not an error.
- When another counter reaches its ceiling while the evaluator assesses the
  instance, the evaluator SHALL stop that instance with the `Incomplete`.
- The `Incomplete` SHALL name the assessed instance.
- The evaluator SHALL NOT change the result of any other call.
- Calls that share one `Meter` SHALL accumulate its charges, so the call at which
  the meter's ceiling is reached depends on the calls before it.
- The temporal counters SHALL be per call.

### Counter mapping

- NFR-008's `positions`, `valuations`, `captures` and `visits` (`evaluation_steps`)
  keep their meaning. `retention` is `support`, and `horizon` is `history_span`.
- NFR-008's `depth` and the retired `formula_depth` have no counter: ADR-030 D-1
  and NFR-008 Scope state that formula nesting is not a limit.
- NFR-008's `instances` is the counter `instances`. The legacy native request fixed
  it at 1 because it carried one trigger; the ordered `deliveries` list of FR-516
  makes it a real ceiling.
- The retired `json_depth` has no counter (FR-516).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-523-AC-1 | For each of the thirteen counters, the setting `temporal_evidence.<counter>` is accepted by FR-255's settings operation and sets that counter's ceiling and no other. | Test (TC-1318) |
| FR-523-AC-2 | The settings `temporal_evidence.formula_depth`, `temporal_evidence.depth`, the bare counter name and a value that is not a non-negative integer are each refused as unknown or non-numeric settings. | Test (TC-1318) |
| FR-523-AC-3 | Over **R**'s timestamped-event `on origin` clause `eventually[0,5] holds(p)` with one origin and three eligible positions (no silent sample), `usage` reports `formula_nodes` 2, `instances` 1, `positions` 3, `valuations` 3, `captures` 0, `evaluation_steps` 8 (one origin and the seven charges of FR-501-AC-12) and `visited` 8. | Test (TC-1318) |
| FR-523-AC-4 | Each of the thirteen counters, with its ceiling at exactly the usage the call measures, completes. | Test (TC-1318) |
| FR-523-AC-5 | Each of the thirteen counters, with its ceiling one below the usage the call measures, returns `Incomplete` naming that counter and its setting, with no truth. | Test (TC-1318) |
| FR-523-AC-6 | A formula that shares one subformula between two parents counts the shared node once in `formula_nodes`. | Test (TC-1318) |
| FR-523-AC-7 | A ceiling of zero is used as zero: a call with `positions` 0 returns `Incomplete` naming `positions` at the first scan. | Test (TC-1318) |
| FR-523-AC-8 | A ceiling raised above its default is used as given, and the setting is not clamped. | Test (TC-1318) |
| FR-523-AC-9 | A refused charge leaves the counter's usage at its last successful value. | Test (TC-1318) |
| FR-523-AC-10 | The charge log of a call whose ceiling is one below the exact usage equals the log of the successful prefix of the full call, including over the clause of AC-21, whose thirteen meter charges come in the Charge order: charges 1 to 3 the materialization charges, 4 the origin, 5 the `Step` of `eventually`, 6 the `Scan` at position 0, 7 to 9 the silent scans at positions 1 to 3, 10 the `Lookup` at position 0, 11 to 13 the silent lookups at positions 1 to 3; a meter ceiling of 9 stops the call at the tenth charge, the non-silent `Lookup` at position 0, and the log holds the first nine. | Test (TC-1318) |
| FR-523-AC-11 | A retry of the AC-3 call after an AC-5 stop, with sufficient ceilings from the same inputs, returns the result of a call that never stopped, byte for byte. | Test (TC-1318) |
| FR-523-AC-12 | A clause whose `history_span` exceeds its ceiling stops the call before the instance is assessed, and the call returns an `Incomplete` result naming `history_span` with `activation = Unavailable` and no truth. | Test (TC-1318) |
| FR-523-AC-13 | A clause whose `formula_nodes` exceeds its ceiling stops the call before the instance is assessed, and the call returns an `Incomplete` result naming `formula_nodes` with `activation = Unavailable` and no truth. | Test (TC-1318) |
| FR-523-AC-14 | Two calls for instances `t1` and `t2` of **R**'s `on each` clause share one `Meter` whose work-unit ceiling is the first call's usage plus one: the first call returns its result with its truth, basis and support, and the second returns `Incomplete` at the `temporal.visit` charge point naming `t2`; when the clause reads silent samples, the first call's usage counts each silent-sample charge as a meter unit, so a ceiling that counted only `evaluation_steps`, or one that omitted the materialization charges, would admit the second call and fails this criterion. | Test (TC-1318) |
| FR-523-AC-15 | With `evaluation_steps` at 1000 and the meter's work-unit ceiling one below the call's `temporal.visit` usage, the `Incomplete` is at the `temporal.visit` charge point; with the meter unbounded and `evaluation_steps` one below, it names `evaluation_steps`. | Test (TC-1318) |
| FR-523-AC-16 | A stamp for which `t0 + Hf + m` exceeds `u64::MAX` returns `Incomplete` naming `history_span` before any position is visited (the charge log is empty), and a stamp whose sum is `u64::MAX` evaluates. | Test (TC-1318) |
| FR-523-AC-17 | A formula nested 100,000 deep evaluates with `formula_nodes` and `evaluation_steps` raised to fit it, on a thread with a 512 KiB stack. | Test (TC-1318) |
| FR-523-AC-18 | The `usage` of a call reports all thirteen counters, and two requests that differ only in one ceiling give distinct `result_identity` values (FR-522-AC-17). | Test (TC-1318) |
| FR-523-AC-19 | After the second call of AC-14 stopped, the first call replayed with a fresh `Meter` returns a result equal to the first call's result. | Test (TC-1318) |
| FR-523-AC-20 | A call with a ledger holding `t1`, a redelivery of `t1`, one new healthy delivery and one new faulted delivery reports `instances` usage 1. | Test (TC-1318) |
| FR-523-AC-21 | Over **R**'s fixed-sample `on origin` clause `eventually[0,3] holds(p)` with one observed position at index 0 where `p` is false, watermark 3, `Complete { through: 3 }` and `Closed` scope (samples 1 to 3 are silent, within the reach, and the verdict is not settled before sample 3), `usage` reports `formula_nodes` 2, `positions` 1, `valuations` 4, `evaluation_steps` 4 (the origin, the `eventually` activation, and the scan and lookup at position 0) and `visited` 7, and the meter's `temporal.visit` usage is 13: the 4 evaluated position visits, 6 charges at the silent samples (3 scans and 3 lookups) and 3 materialization charges, one per silent sample. Wrong readings fail it: all thirteen charges counted in `evaluation_steps` reports 13, the six silent evaluation charges counted in `evaluation_steps` together with the four evaluated visits reports 10, the silent scans counted in `positions` reports 4, silent evaluation charges left off the meter reports a meter usage of 7, and omitting the materialization charges reports a meter usage of 10. | Test (TC-1318) |
| FR-523-AC-23 | For each stop of AC-5, AC-12, AC-13 and AC-16 after a call has created an instance, the result's `ledger` and `progress_ledger` equal the request's byte for byte. | Test (TC-1318) |
| FR-523-AC-22 | Every stop of AC-5, AC-12, AC-13 and AC-16 is an `Ok` result with `activation = Unavailable`, `incomplete` naming the counter and no truth, and none is an `Err`. | Test (TC-1318) |
| FR-523-AC-24 | Over **R**'s fixed-sample `on origin` clause `eventually[0,1] holds(p)` with one observed position at index 0 where `p` is false, watermark 3, `Complete { through: 3 }` and `Closed` scope, the reach (1) is less than the watermark span (3): `usage` reports `formula_nodes` 2, `positions` 1, `valuations` 2, `evaluation_steps` 4, `visited` 5, and the meter's `temporal.visit` usage is 7: 1 materialization charge (`min(3, 0 + 1) - 0`), the origin, the `Step`, the `Scan` at 0, the silent scan at 1, the `Lookup` at 0 and the silent lookup at 1. A reading that charges each silent sample (3 materialization charges) reports a meter usage of 9, and fails this criterion. | Test (TC-1318) |

## Dependencies

- NFR-008-AC-1 to AC-5 and FR-052-AC-11 (the legacy criteria this requirement
  succeeds), FR-255 (setting names and defaults, extended with the thirteen rows of
  this requirement), FR-327 and FR-501 (the visit charge), FR-522 (the identity
  that carries the ceilings).
- FR-516 carries the counter names and the entry. FR-524 (the activation charges)
  and FR-526 (`lineage`) use these counters and depend on this requirement; this
  requirement refers to them and does not depend on them.

## References

- Linear QSL-1502; deletion-table rows B13 and B16; the retired native temporal
  request limits (QSL-303).
