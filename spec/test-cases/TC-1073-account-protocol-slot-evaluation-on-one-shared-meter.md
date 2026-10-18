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

Verify that the work of a slot's evaluation is charged to the caller's meter
under the accounting counters, that a meter at exactly the measured usage
completes and one below it stops with the counter named, that several calls on
one meter accumulate exactly the sum of their charges, and that the same call
over the package read back and over the checked package gives the same result
and the same charges. Scope: FR-390-AC-9 to FR-390-AC-11.

Catches a slot that costs nothing, a meter that resets between calls, a total
that depends on call order, an exhausted call that returns part of a value, a
counter that is over- or under-charged while the two named counters agree, and
a package read back from bytes that charges differently from the one that was
emitted.

Every row asserts relations and boundaries, never an absolute count: `U` is
measured on the row's own call, and a row compares a call's usage with another
measured usage, with `U` and `U` less one, or with the sum of measured usages.
The charge points and their order are FR-390's: expression nodes in evaluation
order, `model.navigate` and `model.deref` where the slot may read model state,
then `result_units`.

Fixture **E** and the observation of TC-1070. The five calls of one
compensation, with the supplies of TC-1070 rows 3, 6, 4, 8 and 12, are **R**
(`Refund.registration_captures[0]`), **A** (`Refund.activation_captures[0]`),
**G** (`Refund.trigger.guard`), **T** (`Refund.retry.relation`) and **V**
(`Refund.recovery.predicate`). The call **C** is `Refunds.captures[0]` with
`view = n5`, the one call that reads the observation. A call's **usage** is the
counters it consumed on a fresh meter with every counter unbounded, measured by
running it; its **charged counters** are those with nonzero usage. The meter
takes its limits from FR-255's accounting table: a counter at a bound `B` is set
by its name.

## Test Procedure

| # | Run | Expected |
| --- | --- | --- |
| 1 | T on a fresh unbounded meter, twice | equal `Completed(true)` and equal usage; the usage of T exceeds that of TC-1070 row 10's call, which decides on its first operand |
| 2 | T with `work_units` = its `Uw` | `Completed(true)`, the meter at exactly `Uw` |
| 3 | T with `work_units` = `Uw` - 1 | `Incomplete` naming `work_units`, `resource_exhausted`/`insufficient-next-charge`, no value |
| 4 | T with `work_units` = 0 | `Incomplete` naming `work_units`, no value |
| 5 | T with `result_units` = its `Ur` | `Completed(true)`; with `Ur` - 1, `Incomplete` naming `result_units`, no value |
| 6 | T with every counter at its measured usage | `Completed(true)` |
| 7 | T with each charged counter in turn at its usage less one and every other counter at its usage | `Incomplete` naming that counter, no value; one run per charged counter |
| 8 | T after row 3 on a fresh meter with the default limits | `Completed(true)` with usage equal to row 1's |
| 9 | C with every counter at its measured usage, then each charged counter in turn one below | the first completes with 5; each of the others is `Incomplete` naming that counter (C is the call that charges `model.deref` and `model.navigate`) |
| 10 | one meter with `work_units` = the sum of the `Uw` of R, A, G, T and V; run them in that order | all five complete; the meter's `work_units` consumed equals the sum |
| 11 | one meter with `work_units` = that sum - 1; run R, A, G, T, V in turn | R, A, G and T complete; V returns `Incomplete` naming `work_units`; the meter holds R's, A's, G's and T's charges plus V's up to the denial |
| 12 | one meter with the sum as `work_units`; run V, T, G, A, R in turn | all five complete; the consumed total equals row 10's |
| 13 | T over the checked package and over the package the contract reader returns for the bytes FR-383 emits for it | equal results and equal charge logs |
| 14 | a refused call (TC-1071 row 12) on a meter, then T on that meter | the meter after the refusal equals the meter before it; T completes |

## Expected Results

A call charges the counters for the nodes it evaluates and the value it
retains, and nothing else. A meter at exactly the measured usage on every
charged counter completes, one below on any charged counter stops with that
counter named and no value, and a fresh meter completes the same call. Several
calls on one meter add up to the sum of their charges in any order, with the
last call to reach the bound the one that stops. The package the contract reader
returns and the checked package that was emitted are the same evaluation.
