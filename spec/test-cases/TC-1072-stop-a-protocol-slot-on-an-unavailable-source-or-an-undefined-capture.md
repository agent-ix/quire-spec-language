---
id: TC-1072
title: "Stop a protocol slot on an unavailable source or an undefined capture"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: verifies
---
# TC-1072: Stop a protocol slot on an unavailable source or an undefined capture

## Description

Verify that a source supplied as unavailable stops an evaluation only when the
evaluation reads it, that the stop names the binder and the read and keeps the
charges made, and that an undefined capture gives the slot an undefined result
with its cause. Scope: FR-390-AC-8 and FR-390-AC-9.

Catches an unavailable source that becomes false, zero or an empty value, an
unavailable source that stops a call whose short-circuit never reads it, a stop
that discards the work done before it, and an undefined capture that completes
as a Boolean.

Fixture **E** and the observation of TC-1070. Fixture **U** is E with a query
operation `check` of `Node`, with result type `Boolean`, whose effective
precondition is false for the object `a` and true for `b`, and a second
protocol capture after `order` in `Refunds`:

```
capture gate: Boolean = view.check();
```

## Test Procedure

| # | Slot | Supply | Expected |
| --- | --- | --- | --- |
| 1 | `Refund.retry.relation` | `earlier = a`, `later = b`, `forward = a`, `trigger` unavailable (`not-observed`) | `Unavailable`: binder `trigger`, cause `not-observed`, the read `trigger` inside the initializer of `activated`; `captures()` is `target`; the meter holds the charges of the work done before the stop (greater than zero) |
| 2 | `Refund.retry.relation` | as row 1 with `earlier = c` | `Completed(false)`, `captures()` none: the first operand decides it and the unavailable `trigger` is never read |
| 3 | `Refund.trigger.guard` | `forward = a`, `trigger` unavailable | `Unavailable`: binder `trigger`, the read `trigger.ready` in the guard; `captures()` none |
| 4 | `Refund.retry.relation` | `earlier = a`, `later = b`, `forward` unavailable, `trigger = off` | `Unavailable`: binder `forward`, the read inside the initializer of `target`; `captures()` none |
| 5 | `Refund.recovery.predicate` | `recovered = a`, `forward = a`, `trigger` unavailable | `Unavailable`: binder `trigger`; `captures()` is `target` |
| 6 | U's `Refunds.captures[1]` | `view = a` | `FamilyResult::Undefined` with reason `precondition-false`, category undefined, exit status 10; no value |
| 7 | U's `Refunds.captures[1]` | `view = b` | `Completed(true)` |
| 8 | the rows above | | exit status: row 1, 3, 4 and 5 incomplete (22); row 6 undefined (10); rows 2 and 7 completed (0) |

## Expected Results

An unavailable source is data about what the supplier could not observe. It
stops the evaluation at the read that needs it, naming the binder, the cause the
supplier gave and the read, and the call keeps the charges it made; it never
becomes a value. A call that decides before reading the source completes. An
undefined capture gives the slot an undefined result with the cause the family
gives it.
