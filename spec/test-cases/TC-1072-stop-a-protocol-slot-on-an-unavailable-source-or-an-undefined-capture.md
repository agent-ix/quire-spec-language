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
charges made, and that a capture whose initializer makes a call with a false
precondition gives the capture slot an undefined result with its cause. Scope:
FR-390-AC-7 and FR-390-AC-8.

Catches an unavailable source that becomes false, zero or an empty value, an
unavailable source that stops a call whose short-circuit never reads it, a stop
that discards the work done before it, and an undefined capture that completes
as a Boolean.

Fixture **E** and the observation of TC-1070. Fixture **U** is E with two
additions that U defines for itself. First, the model `M` declares a query
operation `check(): Boolean` of `Node`, declared as TC-1039 step 7 declares its
query operation `ready(): Boolean`, with an effective precondition that is
`false` for the object `n5` and `true` for a second object `n6` of `nodes`
(`total` 6), and returning `true`. Second, the protocol `Refunds` has a second
protocol capture after `order`:

```
capture gate: Boolean = view.check();
```

The call precondition is not a definedness obligation (FR-301), so S3 admits the
initializer.

## Test Procedure

| # | Slot | Supply | Expected |
| --- | --- | --- | --- |
| 1 | `Refund.retry.relation` | `earlier = Attempt(tally 0)`, `later = Attempt(tally 1)`, `target = 5`, `activated` unavailable (`not-observed`) | `Unavailable`: binder `activated`, cause `not-observed`, the read of `activated` in `not activated`; the meter holds the charges of the work done before the stop (greater than zero) |
| 2 | `Refund.retry.relation` | as row 1 with `earlier = Attempt(tally 3)` | `Completed(false)`: the first operand decides it and the unavailable `activated` is never read |
| 3 | `Refund.trigger.guard` | `target = 5`, `trigger` unavailable | `Unavailable`: binder `trigger`, the read `trigger.ready` in the guard |
| 4 | `Refund.retry.relation` | `earlier = Attempt(tally 0)`, `later = Attempt(tally 1)`, `target` unavailable, `activated = false` | `Unavailable`: binder `target`, the read in `target >= 0` |
| 5 | `Refund.recovery.predicate` | `recovered = Receipt(total 5)`, `target = 5`, `activated` unavailable | `Unavailable`: binder `activated` |
| 6 | `Refund.recovery.predicate` | as row 5 with `recovered = Receipt(total 9)` | `Completed(false)`: `recovered.total = target` decides it |
| 7 | U's `Refunds.captures[1]` | `view = n5` | `FamilyResult::Undefined` with reason `precondition-false`, category undefined, exit status 10; no value |
| 8 | U's `Refunds.captures[1]` | `view = n6` | `Completed(true)` |
| 9 | the rows above | | exit status: rows 1, 3, 4 and 5 incomplete (22); row 7 undefined (10); rows 2, 6 and 8 completed (0) |

## Expected Results

An unavailable source is data about what the supplier could not observe. It
stops the evaluation at the read that needs it, naming the binder, the cause the
supplier gave and the read, and the call keeps the charges it made; it never
becomes a value. A call that decides before reading the source completes. An
undefined capture gives the capture slot an undefined result with the cause the
family gives it.
