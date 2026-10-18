---
id: TC-1073
title: "Account protocol slot evaluation on one shared meter"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: verifies
---
# TC-1073: Account protocol slot evaluation on one shared meter

## Description

Verify that the work of a slot's evaluation, captures included, is charged to
the caller's meter under the accounting counters, that a boundary of exactly
the measured usage completes and one below it stops with the counter named, that
several calls on one meter accumulate exactly the sum of their charges, and
that the same call over the emitted package and over the checked package gives
the same result and the same charges. Scope: FR-390-AC-10 to FR-390-AC-12.

Catches a capture that costs nothing, a capture charged twice, a meter that
resets between calls, a total that depends on call order, an exhausted call that
returns part of a value, and a package read back from bytes that charges
differently from the one that was emitted.

Fixture **E** and the observation of TC-1070. The three calls of one
compensation, with the supplies of TC-1070 rows 4, 8 and 12 (the true cases),
are **G** (`Refund.trigger.guard`), **T** (`Refund.retry.relation`) and **V**
(`Refund.recovery.predicate`). A call's **usage** is the counters it consumed
on a fresh meter with every counter unbounded, measured by running it. `Uw` and
`Ur` are its `work_units` and `result_units`. The meter takes its limits from
FR-255's accounting table: a counter at a bound `B` is set by its name.

Every row asserts relations and boundaries, never an absolute count: `U` is
measured on the row's own call, and a row compares a call's usage with another
measured usage, with `U` and `U` less one, or with the sum of measured usages.
The charge points and their order are FR-390's: expression nodes in evaluation
order, a capture's initializer nodes at its first read and once, field reads,
then `result_units`.

## Test Procedure

| # | Run | Expected |
| --- | --- | --- |
| 1 | T on a fresh unbounded meter, twice | equal `Completed(true)`, equal `captures()` (`target`, `activated`) and equal usage; `Uw` of T is greater than `Uw` of TC-1070 row 10's call, which reads no capture |
| 2 | T with `work_units` = `Uw` | `Completed(true)`, the meter at exactly `Uw` |
| 3 | T with `work_units` = `Uw` - 1 | `Incomplete` naming `work_units`, `resource_exhausted`/`insufficient-next-charge`, no value, `captures()` as far as completed |
| 4 | T with `work_units` = 0 | `Incomplete` naming `work_units`, no value |
| 5 | T with `result_units` = `Ur` | `Completed(true)` |
| 6 | T with `result_units` = `Ur` - 1 | `Incomplete` naming `result_units`, no value |
| 7 | T after row 3 on a fresh meter with the default limits | `Completed(true)` with usage equal to row 1's |
| 8 | one meter with `work_units` = `Uw(G) + Uw(T) + Uw(V)`; run G, T, V in turn | all three `Completed(true)`; the meter's `work_units` consumed equals the sum |
| 9 | one meter with `work_units` = that sum - 1; run G, T, V in turn | G and T complete; V returns `Incomplete` naming `work_units`; the meter holds G's and T's charges plus V's up to the denial |
| 10 | one meter with the sum as `work_units`; run V, T, G in turn | all three complete; the consumed total equals row 8's |
| 11 | T with the retry relation `... and target >= 0 and target <= 1000 and not activated` (`target` read twice) | `captures()` lists `target` once; the usage exceeds T's by the nodes of the added operand and not by a second evaluation of `target`'s initializer |
| 12 | T over the checked package and over the package the contract reader returns for the bytes FR-383 emits for it (once the reader's protocol step exists; before it, the row runs the in-process package only) | equal results, equal `captures()`, equal charge logs |
| 13 | a refused call (TC-1071 row 14) on a meter, then T on that meter | the meter after the refusal equals the meter before it; T completes |

## Expected Results

A call charges the counters for the nodes it evaluates, the captures it reads
and the value it retains, and nothing else. Exactly the measured usage
completes, one below stops with the counter named and no value, and a fresh
meter completes the same call. Several calls on one meter add up to the sum of
their charges in any order, with the last call to reach the bound the one that
stops. A capture is evaluated and charged once per call. The emitted package and
the checked package are the same evaluation.
