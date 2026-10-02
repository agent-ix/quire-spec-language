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
- Evaluation SHALL charge one unit of ADR-014 TR-5 work per value visited.
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
| FR-194-AC-3 | With `min_count 5` and three responses in the window, the atom is `Undefined(InsufficientData{count: 3, min_count: 5})` and the clause is undefined at that position. With `max_window_values` 10 and 20 values in the window, the run stops `Incomplete(LimitReached{…})` naming `max_window_values` and 10. The run's work meter counts 20 for one evaluation over 20 values. | Test (TC-629) |
| FR-194-AC-4 | S3 refuses an aggregate term in a claim over a model subject, `past[0, 300 s]` under event-position, `quantile 1`, `min_count 0`, and a threshold of length against a time value. AC-1's clause has the same horizon with and without its aggregate atom; its tl-mltl input is the derived Boolean signal; a holding result is `tested` with evidence `Observed`. | Test (TC-629) |

## Dependencies

- ADR-024 WA-1 to WA-8, QS-10; ADR-014 A-4, B-2, TR-4, TR-5; ADR-013 O-16.
- [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (profiles at S3).

## References

- QSpec half, which owns the aggregate grammar, windows under both profiles,
  nearest-rank quantiles and `InsufficientData`: QSpec FR-414 (Linear
  STD-137).
- Owning ticket: Linear QSL-371.
