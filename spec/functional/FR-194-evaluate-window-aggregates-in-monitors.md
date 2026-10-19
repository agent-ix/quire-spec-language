---
id: FR-194
title: "Evaluate window aggregates in monitors"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-022
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-024
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
---
# FR-194: Evaluate window aggregates in monitors

## Description

S3 SHALL admit **aggregate terms** over a past window of a trace subject's
positions (`count`, `sum`, `min`, `max`, `fraction holds(P)` and `quantile
q`), each over a value expression with an optional filter and a declared
`min_count`, compared with a threshold to form an atom (ADR-024 WA-1 to
WA-4). The layer-5 evaluator SHALL compute each aggregate atom at atom
evaluation, before temporal evaluation, as a deterministic function of the
trace, so the temporal layer stays Boolean (WA-7). A window holding fewer
than `min_count` values SHALL make the atom `Undefined`; a window holding
more than the run's `max_window_values` SHALL make the result `Incomplete`.

## Use case

A monitor checks "p95 latency of responses over the last five minutes is at
most 5 ms" on a live timestamped trace. At each position it gathers the
latencies of the responses whose time stamps lie in the past 300 s, takes the
nearest-rank 95th percentile and compares it with 5 ms. A quiet window with
too few responses is undefined, never a pass.

## Inputs

- A parsed clause over a trace subject with aggregate terms, under the
  event-position false-extension or timestamped-event finite-window profile
  (QSpec FR-090).
- An observed trace, and the run's `max_window_values` (ADR-014 B-2).

## Outputs

```rust
pub enum AggregateKind { Count, Sum, Min, Max, Fraction(CheckedPredicate), Quantile(Rational) }
pub struct AggregateTerm {
    pub kind: AggregateKind,
    pub value: Option<CheckedExpression>,  // absent for count and fraction
    pub filter: Option<CheckedPredicate>,
    pub window: PastWindow,
    pub min_count: u64,                    // >= 1
}
pub enum PastWindow { Positions { a: u64, b: u64 }, Duration { a: Quantity, b: Quantity } }
// Atom evaluation: Value::Bool, or Undefined(InsufficientData{count, min_count}),
// or Undefined(<cause of the undefined value, filter or predicate>),
// or a run stop Incomplete(LimitReached{limit, value, setting}) (ADR-018 V-7)
```

## Behavior

### S3

- S3 SHALL admit an aggregate term only in a clause over a trace subject. An
  aggregate term in a claim over a model subject SHALL be refused, with a
  diagnostic naming ADR-024 §2's measures.
- `past[a, b]` SHALL have `a <= b`: natural numbers under event-position,
  durations under the timestamped-event profile. A duration window under
  event-position, or a position window under timestamped-event, SHALL be
  refused.
- `quantile q` SHALL have an exact rational `0 < q < 1`. `min_count` SHALL
  be at least 1.
- The value expression and the threshold SHALL have equal dimensions; a
  threshold in another unit converts exactly by QSpec FR-142, and another
  dimension SHALL be refused.
- An aggregate atom SHALL add nothing to the formula's horizon (ADR-014
  TR-4).

### Evaluation

- At position `i`, under event-position, the window SHALL hold the positions
  `j` with `i − b <= j <= i − a` and `j >= 0`; under timestamped-event, the
  events whose time stamp `t_j` satisfies `t_i − b <= t_j <= t_i − a`. No
  window SHALL read a position after `i`.
- The evaluator SHALL gather the value at each window position where the
  filter holds, in exact arithmetic. `count` is the number of such
  positions; `sum`, `min` and `max` are over their values; `fraction
  holds(P)` is the number of them where `P` holds over their number;
  `quantile q` is the nearest-rank quantile, the `⌈q · n⌉`-th smallest of
  the `n` values.
- When the window holds fewer than `min_count` values, the atom SHALL
  evaluate to `Undefined` with cause `InsufficientData{count, min_count}`,
  and the clause at that position is O-16 undefined.
- When a window would hold more than `max_window_values` values, evaluation
  SHALL stop and the result SHALL be `Incomplete(LimitReached{limit, value,
  setting})` (ADR-018 V-7), naming `max_window_values`, its value and
  `monitor.max_window_values`. The
  limit SHALL have a published default of 1,048,576 (2^20), which QSL
  publishes, and the setting name `monitor.max_window_values` (FR-255's
  convention).
- The evaluator SHALL visit the window's positions oldest first. When the
  filter is undefined at a visited position, or the value expression or the
  `fraction` predicate is undefined at a position the filter admits (for
  example a division by zero), the atom SHALL evaluate `Undefined` with the
  cause of the first such position, and the evaluator SHALL stop at that
  position. FR-327's rule for an undefined atom then applies: the result is
  `Undefined(UndefinedEvaluation{where, cause})`, `where` the first
  position at which the atom is undefined, in the O-16 category violation, so a clause
  whose atom cannot be evaluated fails exactly as the same division does in a
  plain `holds` atom (FR-327-AC-5). A value expression is read only at
  admitted positions, so a filter that rejects the position keeps its
  undefined value from mattering. The evaluator SHALL NOT skip the position,
  so a window result never improves by the loss of a position it could not
  evaluate. Later positions are not visited, so a later undefined position
  and a later `max_window_values` overflow are not reached. This is an
  undefined evaluation, which `InsufficientData` is not: a window short of
  `min_count` values stays O-16 undefined and is never `UndefinedEvaluation`.
  When the same window also holds an undefined position, the undefined
  position's cause wins, because it is the first fault found and the count is
  incomplete.
- Evaluation SHALL charge one unit of ADR-014 TR-5 work per window position
  visited, whether or not the filter admits the position, and under a stop
  at an undefined position, the positions visited up to and including it.
  These visit units are separate from the units that evaluating a filter,
  value or predicate charges itself.
- Each aggregate atom SHALL become a derived Boolean signal per position,
  and the temporal layer (tl-mltl) SHALL receive those signals unchanged.

### Meaning

- A monitor result over an observed trace SHALL read by ADR-014 A-4: a
  violation is a violation of the claim on that trace, and a holding result
  is `tested`, with evidence kind `Observed{trace, positions}`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-194-AC-1 | Over a timestamped trace of 20 responses with latencies `1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 6, 7` ms in one 300 s window, `quantile 0.95 of e.latency where holds(e is Response) over past[0, 300 s] <= 5 ms` is false at the last position (the 19th smallest is 6 ms); with `quantile 0.9` it is true (the 18th smallest is 1 ms). The threshold `0.005 s` gives the same results. | Test (TC-629) |
| FR-194-AC-2 | Under event-position, `count where holds(e is Error) over past[0, 9] <= 2` at position 4 reads positions 0 to 4 only; at position 15 it reads positions 6 to 15. `fraction holds(e.ok) over past[0, 3]` over `ok` values `true, false, true, true` is `3/4` at position 3. `sum`, `min` and `max` of `1, 4, 2` are 7, 1 and 4. | Test (TC-629) |
| FR-194-AC-3 | With `min_count 5` and three responses in the window, the atom is `Undefined(InsufficientData{count: 3, min_count: 5})` and the clause is undefined at that position. With `max_window_values` 10 and 20 values in the window, the run stops `Incomplete(LimitReached{…})` naming `max_window_values` and 10. The visit charge is 20 units for one evaluation over 20 window positions, counting the units of the visits and not those of evaluating a filter, value or predicate. | Test (TC-629) |
| FR-194-AC-4 | S3 refuses an aggregate term in a claim over a model subject, `past[0, 300 s]` under event-position, `quantile 1`, `min_count 0`, and a threshold of length against a time value. AC-1's clause has the same horizon with and without its aggregate atom; its tl-mltl input is the derived Boolean signal; a holding result is `tested` with evidence `Observed`. | Test (TC-629) |
| FR-194-AC-5 | Over event-position `x` values `2, 1, 0, 4` at positions 0 to 3, `sum of 6 / e.x over past[0, 3] <= 20` has the clause result `Undefined(UndefinedEvaluation{where: position 2, cause: division-by-zero})` in the O-16 category violation, `where` being the first position at which the atom is undefined (the window at position 2 is the first to hold `x` 0). At position 3 the atom reads `Undefined` with cause `division-by-zero`, not the sum `10.5` of the other three positions and not a holding result. The guarded `sum of 6 / e.x where holds(e.x != 0) over past[0, 3] <= 20` stays defined at position 3: the sum is `10.5` and the atom is true. At position 3, `count where holds(6 / e.x > 1) over past[0, 3] >= 1` reads `Undefined` with `division-by-zero` for the undefined filter, and `fraction holds(6 / e.x > 1) over past[0, 3] >= 1/2` for the undefined predicate. At position 3, the unguarded sum with `min_count 5` reads `Undefined` with `division-by-zero`, not `InsufficientData`, and the guarded sum with `min_count 5` reads `Undefined` with `InsufficientData{count: 3, min_count: 5}`, which is O-16 `undefined` and never `UndefinedEvaluation`. `past[2, 3]` at position 3 holds positions 0 and 1 only, with no zero: `sum of 6 / e.x over past[2, 3] <= 20` is `9` and true, `count` is `2`, and `fraction holds(6 / e.x > 1)` is `1`. | Test (TC-629) |
| FR-194-AC-6 | Over `x` values `1, 0, 2, 3` at positions 0 to 3, `sum of 6 / e.x over past[0, 3]` has the clause result `UndefinedEvaluation{where: position 1, cause: division-by-zero}`. Its atom at position 3 reads `Undefined` with `division-by-zero` and charges 2 visit units, positions 0 and 1, because the evaluator visits oldest first and stops at position 1; the clause-level charge over positions 0 and 1 is 3 units (1 at position 0, 2 at position 1). With `max_window_values` 2 the atom at position 3 is the same, not `Incomplete`, because the third admitted value, at position 2, is not reached; a newest-first visit would meet position 1 as its third admitted value and stop `Incomplete`. A filter that rejects 10 of 20 positions charges 20 visit units, not 10. | Test (TC-629) |

## Dependencies

- QSpec FR-414, which states the same Undefined rule and charge in the shared half.
- ADR-024 WA-1 to WA-8, QS-10; ADR-014 A-4, B-2, TR-4, TR-5; ADR-013 O-16.
- [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (profiles at S3).

## References

- QSpec half, which owns the aggregate grammar, windows under both profiles,
  nearest-rank quantiles and `InsufficientData`: QSpec FR-414 (Linear
  STD-137).
- Owning ticket: Linear QSL-371.
