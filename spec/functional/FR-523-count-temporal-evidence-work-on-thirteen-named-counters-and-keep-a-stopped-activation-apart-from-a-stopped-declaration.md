---
id: FR-523
title: "Count temporal evidence work on thirteen named counters and keep a stopped activation apart from a stopped declaration"
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
# FR-523: Count temporal evidence work on thirteen named counters and keep a stopped activation apart from a stopped declaration

## Description

The spine temporal evaluator SHALL count the work of one temporal evidence call
across thirteen named counters, each with a caller-set ceiling. When a ceiling is
reached, the evaluator stops with a typed `Incomplete` that names the counter,
and every activation that settled before the stop keeps its truth, basis and
support.

This is the spine successor of NFR-008-AC-1 to AC-5 and of FR-052-AC-11. FR-516
names the counters and the exact-usage rule; this requirement defines what each
counter counts, its setting and the stop scope. The mapping from NFR-008's seven
counters and from the retired fourteen-dimension document limits of the native
temporal request is in "Counter mapping".

## Use case

A caller evaluates a clause over a long recording with a work ceiling it chose.
When the ceiling is too low for the third activation, the first two keep their
verdicts, the third reports which counter ran out and the setting that raises it,
and no activation reports a guess.

## Semantic authority and boundary

NFR-008 owns the reliability target (no truth after a stop; siblings keep
theirs). FR-255 owns setting names and defaults. FR-327 and FR-501 own the
`temporal.visit` charge. This requirement owns the counters' definitions and the
stop scope. The entry that applies them is `evaluate_temporal_evidence` (FR-516).

## Inputs

The thirteen ceilings, set through the settings of FR-255 (`temporal_evidence.*`)
or the library builder `TemporalEvidenceLimits`, and the call's `Meter` (FR-516).

## Outputs

`usage`: the thirteen counters' measured values (FR-516).

`settled_activations`: for each activation that settled before a stop, its
position, truth, settlement basis and decision support. It is the successor of the
legacy `Report.settled_siblings`.

On a stop, `Incomplete(LimitReached{limit, value, setting})` with `limit` the limit
kind of FR-255, `value` the ceiling, `setting` the counter's setting name, the
position of the activation being evaluated (when there is one) and no truth value.

## Behavior

### Counters

Each counter is cumulative over the call, or a peak, as the table says. The table
fixes the unit of one charge; the `temporal.visit` count of one evaluation is
FR-501-AC-12's (one per operator node at its activation, one per `holds` atom
at each eligible position and one scan per position).

| Counter | Kind | What one unit is | Replaces |
| --- | --- | --- | --- |
| `formula_nodes` | peak | a distinct node of the clause's formula reachable from its root; a node shared by two parents counts once | `formula_nodes` |
| `positions` | cumulative | one scan charge of FR-501: one position read by one temporal operator node at one activation | `positions` |
| `valuations` | cumulative | one atomic `holds` lookup, including one that finds no valuation | `valuations` |
| `instances` | peak | one instance created from a delivery or an origin, the greatest number created by the call (FR-524) | NFR-008 `instances` |
| `captures` | cumulative | one capture record retained, across all instances | `captures` |
| `support` | peak | the greatest number of `(position, atom)` pairs an unsettled activation retains, and the number of pairs in a settled decision support | `support`; NFR-008 `retention` |
| `history_span` | peak | the greatest interval upper bound of the clause, in ticks (FR-326 horizon) | `history_span`; NFR-008 `horizon` |
| `evaluation_steps` | cumulative | one charge at the `temporal.visit` charge point (FR-327), including one per delivery or origin resolved, one per capture established and one per evicted record consulted (FR-524) | `evaluation_steps`; NFR-008 `visits` |
| `lineage` | peak | the length of the result's chain of predecessors (FR-526) | `lineage` |
| `visited` | cumulative | one unit of the usage of `formula_nodes`, `positions`, `valuations` and `captures`, and one evicted record consulted | `visited` |
| `request_bytes` | peak | one byte of the request's opaque byte members, in total (FR-516) | `input_bytes` |
| `result_bytes` | peak | one byte of the result's opaque byte members, in total | `output_bytes` |
| `member_bytes` | peak | one byte of the longest single opaque member | `string_bytes` |

- The evaluator SHALL charge a counter only for work it performs.
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
- The evaluator SHALL charge the counter before it charges the meter, and SHALL
  report a denied counter by its name.
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
  `history_span` before it visits any position, and SHALL NOT wrap, saturate or
  narrow.
- The tick domain is `u64`, which is the domain of FR-326's horizon and of
  FR-501's non-negative stamps.

### Stop scope

- The counters `formula_nodes` and `history_span` and the tick arithmetic are
  properties of the clause and its anchors. When one stops the call, the evaluator
  SHALL evaluate no activation.
- The evaluator SHALL leave `settled_activations` empty for such a stop.
- When another counter reaches its ceiling during one activation, the evaluator
  SHALL stop that activation with the `Incomplete`.
- The `Incomplete` SHALL name the position of that activation.
- The evaluator SHALL list every activation that settled before the stop in
  `settled_activations` with its truth, basis and decision support.
- The evaluator SHALL begin no activation after the stop.
- The cumulative counters SHALL be shared by the activations of one call, so the
  activation at which a ceiling is reached depends on the activations before it.

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
| FR-523-AC-3 | Over **R**'s timestamped-event `on origin` clause `eventually[0,5] holds(p)` with one origin and three eligible positions, `usage` reports `formula_nodes` 2, `instances` 1, `positions` 3, `valuations` 3, `captures` 0, `evaluation_steps` 8 (one origin and the seven charges of FR-501-AC-12) and `visited` 8. | Test (TC-1318) |
| FR-523-AC-4 | Each of the thirteen counters, with its ceiling at exactly the usage the call measures, completes. | Test (TC-1318) |
| FR-523-AC-5 | Each of the thirteen counters, with its ceiling one below the usage the call measures, returns `Incomplete` naming that counter and its setting, with no truth. | Test (TC-1318) |
| FR-523-AC-6 | A formula that shares one subformula between two parents counts the shared node once in `formula_nodes`. | Test (TC-1318) |
| FR-523-AC-7 | A ceiling of zero is used as zero: a call with `positions` 0 returns `Incomplete` naming `positions` at the first scan. | Test (TC-1318) |
| FR-523-AC-8 | A ceiling raised above its default is used as given, and the setting is not clamped. | Test (TC-1318) |
| FR-523-AC-9 | A refused charge leaves the counter's usage at its last successful value. | Test (TC-1318) |
| FR-523-AC-10 | The charge log of a call whose ceiling is one below the exact usage equals the log of the successful prefix of the full call. | Test (TC-1318) |
| FR-523-AC-11 | A retry of the AC-3 call after an AC-5 stop, with sufficient ceilings from the same inputs, returns the result of a call that never stopped, byte for byte. | Test (TC-1318) |
| FR-523-AC-12 | A clause whose `history_span` exceeds its ceiling stops the call before any activation is evaluated, with `settled_activations` empty. | Test (TC-1318) |
| FR-523-AC-13 | A clause whose `formula_nodes` exceeds its ceiling stops the call before any activation is evaluated, with `settled_activations` empty. | Test (TC-1318) |
| FR-523-AC-14 | Over **R**'s `on each` clause evaluated by `evaluate_temporal_evidence` over four positions, with `evaluation_steps` set to the first activation's usage plus one, the first activation appears in `settled_activations` with its truth, basis and support, the second activation's `Incomplete` names `evaluation_steps` and the second activation's position, and the third and fourth activations are not begun. | Test (TC-1318) |
| FR-523-AC-15 | With `evaluation_steps` at 1000 and the meter's work-unit ceiling one below the call's `temporal.visit` usage, the `Incomplete` is at the `temporal.visit` charge point; with the meter unbounded and `evaluation_steps` one below, it names `evaluation_steps`. | Test (TC-1318) |
| FR-523-AC-16 | A stamp for which `t0 + Hf + m` exceeds `u64::MAX` returns `Incomplete` naming `history_span` before any position is visited (the charge log is empty), and a stamp whose sum is `u64::MAX` evaluates. | Test (TC-1318) |
| FR-523-AC-17 | A formula nested 100,000 deep evaluates with `formula_nodes` and `evaluation_steps` raised to fit it, on a thread with a 512 KiB stack. | Test (TC-1318) |
| FR-523-AC-18 | The `usage` of a call reports all thirteen counters, and two requests that differ only in one ceiling give distinct `result_identity` values (FR-522-AC-17). | Test (TC-1318) |

## Dependencies

- NFR-008-AC-1 to AC-5 and FR-052-AC-11 (the legacy criteria this requirement
  succeeds), FR-255 (setting names and defaults, extended with the thirteen rows of
  this requirement), FR-327 and FR-501 (the visit charge), FR-522 (the identity
  that carries the ceilings), FR-524 (the activation charges), FR-526
  (`lineage`).
- FR-516 carries the counter names and the entry and is a reference, not a
  dependency, of this requirement.

## References

- Linear QSL-1502; deletion-table rows B13 and B16; the retired native temporal
  request limits (QSL-303).
