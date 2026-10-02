---
id: FR-327
title: "Evaluate a temporal clause over a finite trace at S6a, with typed positions and metered work"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-326
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-090
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-092
    type: depends_on
---
# FR-327: Evaluate a temporal clause over a finite trace at S6a, with typed positions and metered work

## Description

The layer-5 TemporalTrace evaluator SHALL evaluate a checked temporal clause
(FR-326) over a trace of observations, addressing positions by
`TemporalPosition` (ADR-014 TR-2) and charging work on the caller's meter
(TR-5). Under a bounded profile it SHALL evaluate a closed finite trace with
the profile's own meaning and closed-boundary rule (ADR-018 SM-1). The
infinite-trace profile's readings of a finite prefix and of a lasso are
FR-328 and FR-329. This evaluator is the one definition of a temporal
formula's truth on a trace that replay, monitors and the model checker use
(ADR-018 SM-1).

## Use case

An operator holds a recorded run of a system as a few observations and wants
to know whether a bounded response property held on it. The evaluator
decides it exactly, names the first position where it failed, and stops
with an incomplete result, never a guess, when the work budget runs out.

## Inputs

- A `CheckedTemporalClause` (FR-326) and the checked package.
- A `TemporalTrace`: a non-empty `Vec<Observation>` of FR-106-admitted
  observations, position 0 first, each with its anchor, read as a closed
  trace; and, for an infinite-trace lasso, a loop start (FR-329).
- The binding of the clause's `over` parameter.
- A kernel `Meter` (ADR-014 B-2).

## Outputs

- `TemporalPosition(u64)`: a zero-based index into the represented trace,
  prefix first, then loop.
- An evaluation outcome: `Completed(true)`; `Completed(false)` with the
  `TemporalPosition` of the first activation that evaluated false;
  `Undefined(UndefinedEvaluation{where, cause})` with `where` a
  `TemporalPosition`; or `Incomplete` with its charge point.
- `TemporalPosition` encodes into `qsl_replay::TracePosition` as decimal
  ASCII with no leading zero, `0` for position zero.

## Behavior

### Positions

- The evaluator SHALL address each position of the represented trace by
  `TemporalPosition`, and only the TemporalTrace evaluate hook SHALL decode
  a `TracePosition` into one.
- When a `TracePosition` is not decimal ASCII, has a leading zero, or exceeds
  `u64`, decoding SHALL refuse `invalid_runtime_input`/`invalid-value`.
- When a decoded position lies outside the represented trace, reconstruction
  SHALL refuse `invalid_runtime_input`/`invalid-value`.

### Bounded-profile evaluation

- A `holds` atom SHALL evaluate through the one clause evaluator (FR-107)
  over the observation at its position.
- Each interval operator SHALL have QSpec FR-091's offset meaning (future)
  or FR-092's (past) under the selected profile.
- Under a false-extension profile, a position after the last position of
  the trace SHALL read every atomic predicate as false and every constant
  unchanged (QSpec FR-090).
- The clause SHALL be evaluated at position 0 for activation `on origin`,
  and at every activating position for activation `on each`. The outcome
  SHALL be `Completed(true)` when every activation evaluates true, and
  otherwise `Completed(false)` naming the first activation in position
  order that evaluated false.

### Undefined evaluation

- At each position the clause reads, the evaluator SHALL read the clause's
  letter, the value of every atom of the clause there (ADR-018 UE-1). An
  activation `on origin` under a bounded profile SHALL read positions 0 to
  its horizon; every other clause SHALL read every represented position.
  A position a false-extension profile supplies after the trace SHALL
  evaluate no atom.
- When an atom evaluates `Undefined` at a position the clause reads, the
  evaluator SHALL return `Undefined(UndefinedEvaluation{where, cause})`,
  `where` the first such position and `cause` the `UndefinedRecord` of the
  first such atom in clause-node order, in place of a truth value. An
  activation that evaluates false at an earlier position SHALL be returned
  instead: the claim fails at the first refuting position.
- `Undefined` SHALL map to the O-16 category violation, with cause
  `UndefinedEvaluation`: a claim that is not defined on the trace does not
  hold on it.

### Work

- The evaluator SHALL charge `quire_exact::LimitKind::WorkUnits` once for
  each visit of a temporal node at a position (ADR-014 TR-5). A temporal
  node is a temporal operator or a `holds` atom of the clause's formula,
  and a visit is one evaluation of that node at one position.
- When a charge is denied, the evaluator SHALL stop and return `Incomplete`
  with that charge point, and SHALL return no truth value.

### Results

- `Completed(false)` SHALL map to the O-16 category violation and
  `Completed(true)` to `tested`, evidence for this trace only; neither is
  `proved` (ADR-014 §8).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-327-AC-1 | Over the closed `Counter` trace with `c.value` 0, 1, 2 under event-position false-extension, `on origin`: `eventually[0,2] holds(c.value = 2)` is `Completed(true)`; `eventually[0,1] holds(c.value = 2)` is `Completed(false)` at position 0; `always[0,3] holds(c.value <= 2)` is `Completed(false)` (position 3 is past closure); `always[0,3] not holds(c.value > 2)` is `Completed(true)`. | Test (TC-837) |
| FR-327-AC-2 | Over the same trace, `always[0,0] holds(c.value <= 1)` with activation `on each` is `Completed(false)` at position 2. | Test (TC-837) |
| FR-327-AC-3 | `TracePosition` `0` and `2` decode to positions 0 and 2; `02`, `-1`, the empty string and `18446744073709551616` refuse `invalid_runtime_input`/`invalid-value`; position `3` on the three-position trace refuses `invalid_runtime_input`/`invalid-value` at reconstruction. Position 12 encodes as `12`. | Test (TC-837) |
| FR-327-AC-4 | Over AC-1's trace, `eventually[0,2] holds(c.value = 2)` `on origin` makes 4 visits: the `eventually` node at position 0 and the `holds` atom at positions 0, 1 and 2. With a meter of 3 work units it returns `Incomplete` at the `WorkUnits` charge point and no truth value; with a meter of 4 it returns `Completed(true)` and charges 4. | Test (TC-837) |
| FR-327-AC-5 | Over the closed `Counter` trace with `c.value` 0, 1, 2 under event-position false-extension, `always[0,2] holds(2 / (2 - c.value) >= 1)` `on origin` returns `Undefined{where: 2, cause: division-by-zero}`, category violation; `eventually[0,1] holds(2 / (2 - c.value) = 2)` reads positions 0 and 1 only and returns `Completed(true)`. | Test (TC-847) |

## Dependencies

- ADR-014 §3 TR-2 and TR-5, §5 A-4, §8; ADR-018 §2 SM-1.
- [FR-106](FR-106-admit-snapshots-and-invocations.md) (observations),
  [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) (the one clause
  evaluator), [FR-326](FR-326-admit-temporal-operators-by-the-unit-s-temporal-profile.md).
- QSpec FR-090 (closed-boundary rules), FR-091, FR-092.

## References

- Linear QSL-384 (spec ticket); QSL-43 (implementation).
- QSpec FR-360 to FR-370: Linear STD-131.
