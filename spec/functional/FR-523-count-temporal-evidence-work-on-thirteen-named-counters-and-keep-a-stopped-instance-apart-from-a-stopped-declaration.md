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
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-522
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: references
---
# FR-523: Count temporal evidence work on thirteen named counters and keep a stopped instance apart from a stopped declaration

## Description

The spine temporal evaluator SHALL count the work of one temporal evidence call
across thirteen named counters, each with a caller-set ceiling. When a ceiling is
reached, the evaluator stops with a typed `Incomplete` that names the counter, and
every independently settled sibling keeps its truth, basis and support.

This is the spine successor of NFR-008-AC-1 to AC-5 and of FR-052-AC-11. FR-516
names the thirteen counters and the exact-usage rule; this requirement defines
what each counter counts, its setting and the stop scope. NFR-008's seven
counters and the retired fourteen-dimension document limits of the native
temporal request are mapped to the thirteen in "Counter mapping".

## Use case

A caller evaluates a clause over a long recording with a work ceiling it chose.
When the ceiling is too low for the third activation, the first two keep their
verdicts, the third reports which counter ran out and the setting that raises it,
and no activation reports a guess.

## Semantic authority and boundary

NFR-008 owns the reliability target (no truth after a stop; siblings keep
theirs). FR-255 owns setting names and defaults. FR-327 and FR-501 own the
`temporal.visit` charge. This requirement owns the counters' definitions and the
stop scope.

## Inputs

The thirteen ceilings, set through the settings of FR-255 (`temporal_evidence.*`)
or the library builder, and the evidence call's `Meter` (FR-516).

## Outputs

`usage`: the thirteen counters' measured values (FR-516). On a stop,
`Incomplete(LimitReached{limit, value, setting})` with `limit` the limit kind of
FR-255, `value` the ceiling, `setting` the counter's setting name, the located
instance (when there is one) and no truth value.

## Behavior

### Counters

Each counter is cumulative over the call, or a peak, as the table says.

| Counter | Kind | What one unit is | Replaces |
| --- | --- | --- | --- |
| `formula_nodes` | peak | a distinct node of the clause's formula reachable from its root; a node shared by two parents counts once | `formula_nodes` |
| `formula_depth` | peak | the greatest number of nodes on a path from the root to a node, the root counting 1 | `formula_depth`; NFR-008 `depth` |
| `positions` | cumulative | one inspection of a position by an operator or the scan of FR-501, so a position inspected under two operators counts twice | `positions` |
| `valuations` | cumulative | one atomic `holds` lookup, including one that finds no valuation | `valuations` |
| `captures` | cumulative | one capture record retained, across all instances | `captures` |
| `support` | peak | the greatest number of `(position, atom)` pairs an unsettled instance retains, and the number of pairs in a settled decision support | `support`; NFR-008 `retention` |
| `history_span` | peak | the greatest interval upper bound of the clause, in ticks, after checked composition (FR-326 horizon) | `history_span`; NFR-008 `horizon` |
| `evaluation_steps` | cumulative | one charge at the `temporal.visit` charge point (FR-327) | `evaluation_steps`; NFR-008 `visits` |
| `lineage` | peak | the length of the result's chain of predecessors (FR-526) | `lineage` |
| `visited` | cumulative | one unit charged to `formula_nodes`, `positions`, `valuations` or `captures`, and one evicted entry consulted | `visited` |
| `request_bytes` | peak | one byte of the request's opaque byte members, in total (FR-516) | `input_bytes` |
| `result_bytes` | peak | one byte of the result's opaque byte members, in total | `output_bytes` |
| `member_bytes` | peak | one byte of the longest single opaque member | `string_bytes` |

- The evaluator SHALL charge a counter only for work it performs, and a refused
  charge SHALL NOT raise the counter's usage.
- When a call's usage of a counter equals its ceiling, the evaluator SHALL
  complete.
- When the usage a call would need exceeds a ceiling by one, the evaluator SHALL
  return `Incomplete` naming that counter and SHALL NOT perform the unaffordable
  operation.
- The evaluator SHALL use each ceiling as given, above or below its default, and
  SHALL preserve a ceiling of zero. No ceiling clamps another.
- After an `Incomplete`, a retry under sufficient ceilings from the same
  inputs SHALL give the result the first call would have given with those
  ceilings.
- The evaluator SHALL walk the formula over an explicit heap stack, so that the
  nesting depth it accepts is bounded by `formula_depth` and not by the thread's
  stack (ADR-030 D-1).
- The `evaluation_steps` charge is the `temporal.visit` charge of FR-327 and
  FR-501-AC-12, so one evaluation's visit count equals its `evaluation_steps`
  usage.
- When the checked tick arithmetic of an activation (`t0 + Hf + m` and the
  distance between two stamps, FR-501) exceeds the `u64` tick domain, the
  evaluator SHALL return `Incomplete` naming `history_span` and SHALL NOT wrap,
  saturate or narrow.

### Stop scope

- The counters `formula_nodes`, `formula_depth` and `history_span` are properties
  of the clause. When one reaches its ceiling, the evaluator SHALL stop the whole
  declaration before it evaluates any instance, and no instance SHALL carry a
  result.
- When a counter other than those three reaches its ceiling while the evaluator
  evaluates one instance, the evaluator SHALL stop that instance with the
  `Incomplete`, and the `Incomplete` SHALL name that instance.
- An instance that settled before the stop SHALL keep its truth, basis and
  decision support.
- An instance the evaluator had not begun when another stopped SHALL be reported
  unactivated (`non_value.kind` `Unactivated`, FR-516) and SHALL carry no truth.
- The cumulative counters SHALL be shared by the instances of one call, so the
  instance at which a ceiling is reached depends on the instances before it.

### Counter mapping

- NFR-008's `positions`, `valuations`, `captures` and the `visits` counter
  (`evaluation_steps`) keep their definitions. `retention` is `support`, `horizon`
  is `history_span` and `depth` is `formula_depth`.
- NFR-008's `instances` ceiling (concurrently active instances) has no counter
  among the thirteen. Under the evidence call one request carries one delivery
  (FR-516), so one instance is active; under `run_clause` an `on each` clause's
  activations are charged by `evaluation_steps`. The ceiling itself is OPEN-8 in
  FR-525.
- The retired `json_depth` has no counter (FR-516).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-523-AC-1 | For each of the thirteen counters, the setting `temporal_evidence.<counter>` is accepted by FR-255's settings operation, sets that counter's ceiling and no other, and `temporal_evidence.depth`, the bare counter name and a value that is not a non-negative integer are each refused as unknown or non-numeric settings. | Test (TC-1318) |
| FR-523-AC-2 | Over **R**'s timestamped-event clause `eventually[0,5] holds(p)` with three eligible positions, `evaluation_steps` usage is 7, `positions` usage is 3 and `valuations` usage is 3; with `evaluation_steps` at 7 the call completes and with 6 it returns `Incomplete` naming `evaluation_steps` and setting `temporal_evidence.evaluation_steps`, with no truth. | Test (TC-1318) |
| FR-523-AC-3 | For `formula_nodes`, `formula_depth`, `positions`, `valuations`, `captures`, `support`, `history_span`, `lineage`, `visited`, `request_bytes`, `result_bytes` and `member_bytes`, the call at exactly the measured usage completes and the call one below returns `Incomplete` naming the counter with no truth. | Test (TC-1318) |
| FR-523-AC-4 | `formula_nodes` over a clause whose formula shares one subformula between two parents counts the shared node once, and `formula_depth` of `always[0,1] (holds(p) implies eventually[0,1] holds(q))` is 4. | Test (TC-1318) |
| FR-523-AC-5 | A ceiling of zero is used as zero: a call with `positions` 0 returns `Incomplete` naming `positions` at the first inspection, and a ceiling raised above its default is used as given. | Test (TC-1318) |
| FR-523-AC-6 | A refused charge leaves the counter's usage at its last successful value, and the first unaffordable operation is not performed, so the charge log of the call one below the exact usage equals the log of the successful prefix. | Test (TC-1318) |
| FR-523-AC-7 | A retry of the AC-2 call with sufficient ceilings from the same inputs returns the result of a call that never stopped, byte for byte. | Test (TC-1318) |
| FR-523-AC-8 | A clause whose `history_span` exceeds its ceiling, and a clause whose `formula_nodes` exceeds its ceiling, each stop the declaration before any instance is evaluated, and no instance carries a result. | Test (TC-1318) |
| FR-523-AC-9 | Over two activations of **R**'s `on each` clause with `evaluation_steps` set to the first activation's usage plus one, the first activation keeps its truth, basis and decision support, the second returns `Incomplete` naming `evaluation_steps` and its instance, and a third reports `non_value.kind` `Unactivated` with no truth. | Test (TC-1318) |
| FR-523-AC-10 | A stamp whose `t0 + Hf + m` exceeds the `u64` tick domain returns `Incomplete` naming `history_span` and no truth, and a stamp one below that bound evaluates. | Test (TC-1318) |
| FR-523-AC-11 | Two requests that differ only in one of the thirteen ceilings give distinct `result_identity` values (FR-522-AC-13), and the `usage` of a call reports all thirteen counters. | Test (TC-1318) |
| FR-523-AC-12 | A formula nested 100,000 deep evaluates with `formula_nodes`, `formula_depth` and `evaluation_steps` raised to fit it, on a thread with a 512 KiB stack, and the same formula with `formula_depth` one below its depth returns `Incomplete` naming `formula_depth`. | Test (TC-1318) |

## Dependencies

- NFR-008-AC-1 to AC-5 and FR-052-AC-11 (the legacy criteria this requirement
  succeeds), FR-255 (setting names and defaults, extended with the thirteen rows
  of this requirement), FR-327 and FR-501 (the visit charge), FR-516 (the
  evidence call and the counter names), FR-522 (the identity that carries the
  ceilings), FR-526 (`lineage`).
- Feeds QSL-1545 (FR-516-AC-7).

## References

- Linear QSL-1502; deletion-table rows B13 and B16; the retired native temporal
  request limits (QSL-303).
