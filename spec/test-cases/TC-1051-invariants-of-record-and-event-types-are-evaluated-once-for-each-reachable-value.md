---
id: TC-1051
title: "The invariants of a type are evaluated once for each value they range over, and a false result is located by its runtime value path"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-366
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: verifies
---
# TC-1051: The invariants of a type are evaluated once for each value they range over, and a false result is located by its runtime value path

## Description

Verify `evaluate_value_invariants` and the `ValueInvariants` selection: the
values an observation and an invocation reach, one result for each, the
logical violation with no code or cause, the FR-207 path of each result, the
result order, the outcome, the shared budget, and the evaluation of an
object-type, process and state-machine invariant. These are QSpec
[TC-235](ix://agent-ix/quire-specification/TC-235) M13 run 1, M14 run 1 and M16
run 1. Scope: FR-366-AC-4 to FR-366-AC-9 and FR-106-AC-48.

Catches an evaluation that reads only the object's own fields and skips a
collection element, a present optional or a parameter; one that deduplicates
equal values; one that reports a violation with a code; one that roots a
parameter value at the current observation; one that evaluates an event type
with no value; one that follows a reference field; and one that charges a
record field read.

## Test Procedure

Fixture **F**: K2 with `Money/NonNegative` (`self.amount_minor >= 0`) and
`Order` given the field `quoted: Money`, the field `history: Money` with
multiplicity `{0, 3}`, the optional field `discount: Money` and the operation
`requote` with the parameter `amount: Money` and no contract (M13). The
current observation `s1` holds one `Order` object `o1` with `total`
`{amount_minor: 5, currency: "EUR"}`, `quoted` `{-5, "EUR"}`, `history`
`[{1, "EUR"}, {-2, "EUR"}]` and `discount` present as `{-1, "EUR"}`. Invocation
`i1` of `Order/requote` on `o1` binds `amount` to `{-7, "EUR"}`; its `pre`
snapshot is labelled `s2` and its `post` snapshot `s3`. Labels are compared as
QSpec FR-207 compares an observation key, so `s1` orders before `s2`. Admit by
`ValueInvariants { snapshot: s1, invocation: i1 }` and evaluate with an
unlimited meter unless a step says otherwise. Tag the tests
`#[trace("TC-1051", "FR-366-AC-n")]`.

1. Admit and evaluate **F** (FR-106-AC-48, FR-366-AC-4).
2. List `o1`'s fields in another order, `i1`'s parameters in another order and
   evaluate twice (FR-366-AC-5).
3. Change `o1.history[0]` to `{9, "EUR"}` and evaluate; read the result at
   `o1.quoted` (FR-366-AC-5).
4. **F** plus `OrderPlaced/Occurred` (`self.occurred_at = self.occurred_at`) and
   an observation holding one `Order` and no `OrderPlaced` value (M14 run 1)
   (FR-366-AC-6).
5. **F** plus `TaxedMoney` (supertype `Money`, field `tax_minor`), `quoted`
   holding `{amount_minor: -5, currency: "EUR", tax_minor: 20}`, and a record
   `Pair { first: Money; second: Money }` as a field `pair` of `Order` holding
   `{first: {-1, "EUR"}, second: {2, "EUR"}}` (FR-366-AC-7).
6. **F** plus the operation `confirm(placed: OrderPlaced)` with the
   `OrderPlaced/Occurred` invariant `false`, and `i4` of `confirm` on `o1`
   binding `placed` to an `OrderPlaced` value (FR-366-AC-7).
7. **F** plus a field `buddy: Reference<Order>` of `Order`, `o1.buddy` naming
   `o2`, and `o2` holding `quoted` `{-3, "EUR"}` and otherwise non-negative
   `Money` values (FR-366-AC-7).
8. Admit step 1 with a `pre` document as `snapshot`, then with `i1` missing
   `amount` (FR-106-AC-48).
9. Evaluate step 1 with a meter that denies the fourth charge, then the third
   (FR-366-AC-8).
10. **F** plus `Money/Stable`, an `ocl` clause (FR-366-AC-8).
11. The M16 process `Fulfilment` with the type-level invariant
    `NonNegativeAttempts` (`self.attempts >= 0`), observation `s1` holding the
    runs `r1` `{attempts: 0}` and `r2` `{attempts: -1}`; and TC-1045's
    `Orders` fixture (state machine `OrderLifecycle` with the fields `placed`
    and `cancelled`, `place(flags: Flags)` modifying both) plus the type-level
    invariant `Extra` (`self.placed or not self.placed`) of `OrderLifecycle`,
    with TC-1045's invocation `i1` of `place` (`flags` `{placed: true}`), its
    `pre` snapshot `P1` with context `{placed: false, cancelled: false}`
    labelled `s2` and its `post` snapshot `Q2` with context `{placed: true,
    cancelled: true}` labelled `s3`; the fixture's `ExpressLifecycle` specializes `OrderLifecycle`, and `P1` and `Q2` hold no `ExpressLifecycle` context. Neither unit declares a state clause named
    `NonNegativeAttempts`. Evaluate each by `evaluate_value_invariants`, then
    call `evaluate_clause` named `NonNegativeAttempts` over the `Orders` unit
    (FR-366-AC-9). Then remove the `OrderLifecycle` context from `P1`.

## Expected Results

1. Admission names `s1`, `i1`'s `pre` snapshot `s2` and the parameter `amount`.
   Six results of `NonNegative`, in this order: `Completed(false)`, a Boolean
   with no code or cause, at `(s1, object o1)` `[member discount,
   option_value]`; `Completed(true)` at `[member history, index 0]`;
   `Completed(false)` at `[member history, index 1]`; `Completed(false)` at
   `[member quoted]`; `Completed(true)` at `[member total]`; `Completed(false)`
   at `(s2, invocation i1 parameter amount)` with no steps. The outcome is
   `False`.
2. The six results are equal in value and order; two evaluations give equal
   charge logs.
3. The result at `o1.quoted` is unchanged, and `o1.history[0]` is
   `Completed(true)` still.
4. No result for `Occurred` and outcome `True`.
5. The `TaxedMoney` value at `quoted` is evaluated against `TaxedMoney`'s
   effective invariant `NonNegative` and is `Completed(false)` at `[member
   quoted]`; `pair` gives `Completed(false)` at `[member pair, field first]`
   and `Completed(true)` at `[member pair, field second]`.
6. One result of `Occurred`, `Completed(false)`, at `(the pre snapshot of i4,
   invocation i4 parameter placed)` with no steps.
7. `o2`'s `quoted` is evaluated once, `Completed(false)`, at `(s1, object o2)`
   `[member quoted]`, and no result has a path whose steps begin `[member
   buddy`.
8. The `pre` document refuses `wrong_snapshot`/`wrong-observation`; the missing
   parameter refuses `invalid_runtime_input`/`missing-member`.
9. Each value costs three charges: `ordering.operands`,
   `ordering.arithmetic`, `ordering.result-retain`; reading a record field
   charges nothing. With the fourth charge denied the evaluation returns the
   one result for `discount` and `Incomplete` with
   `resource_exhausted`/`insufficient-next-charge`, the denied charge being the
   `ordering.operands` of `[member history, index 0]`, which has no result.
   With the third charge denied it returns no result and `Incomplete`.
10. The six results of step 1 are unchanged and none is for `Stable`.
11. `NonNegativeAttempts` has two results, `Completed(true)` at `(s1, object
    r1)` and `Completed(false)` at `(s1, object r2)`, no steps, and outcome
    `False`. `Extra` has two results, both `Completed(true)`, at `(s2, invocation
    i1 binding receiver)` and `(s3, invocation i1 binding receiver)`, no
    steps; the snapshots admit with `ExpressLifecycle` present, since only the invoked machine's context is required. `P1` without its `OrderLifecycle` context refuses `invalid_model_binding`/`wrong-context` naming `OrderLifecycle` and `P1`. `evaluate_clause` named `NonNegativeAttempts` returns
    `CallFailure::Input(InputRefusal::UnknownClause)`.
