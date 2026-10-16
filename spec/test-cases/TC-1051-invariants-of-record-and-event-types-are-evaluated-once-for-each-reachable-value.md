---
id: TC-1051
title: "The invariants of a record value type and an event type are evaluated once for each reachable value, and a false result is located by its runtime value path"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-366
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: verifies
---
# TC-1051: The invariants of a record value type and an event type are evaluated once for each reachable value, and a false result is located by its runtime value path

## Description

Verify `evaluate_value_invariants` and the `ValueInvariants` selection:
the values an observation and an invocation reach, one result for each, the
logical violation with no code or cause, the FR-207 path of each result, the
result order, the outcome and the budget. These are QSpec
[TC-235](ix://agent-ix/quire-specification/TC-235) M13 run 1 and M14 run 1.
Scope: FR-366-AC-4 to FR-366-AC-8 and FR-106-AC-48.

Catches an evaluation that reads only the object's own fields and skips a
collection element, a present optional or a parameter; one that deduplicates
equal values; one that reports a violation with a code; one that roots a
parameter value at the current observation; and one that evaluates an event
type with no value.

## Test Procedure

Fixture: K2 with `Money/NonNegative` (`self.amount_minor >= 0`) and `Order`
given the field `quoted: Money`, the field `history: Money` with multiplicity
`{0, 3}`, the optional field `discount: Money` and the operation `requote`
with the parameter `amount: Money` and no contract (M13). Observation `s1`
holds one `Order` object `o1` with `total` `{amount_minor: 5, currency:
"EUR"}`, `quoted` `{-5, "EUR"}`, `history` `[{1, "EUR"}, {-2, "EUR"}]` and
`discount` present as `{-1, "EUR"}`. Invocation `i1` of `Order/requote` on
`o1` binds `amount` to `{-7, "EUR"}`. Admit by `ValueInvariants { snapshot:
s1, invocation: i1 }` and evaluate with an unlimited meter unless a step says
otherwise. Tag the tests `#[trace("TC-1051", "FR-366-AC-n")]`.

1. Admit and evaluate (FR-106-AC-48, FR-366-AC-4).
2. List `o1`'s fields in another order, `i1`'s parameters in another order and
   evaluate twice (FR-366-AC-5).
3. Change `o1.history[0]` to `{9, "EUR"}` and evaluate; read the result at
   `o1.quoted` (FR-366-AC-5).
4. Fixture plus `OrderPlaced/Occurred` (`self.occurred_at = self.occurred_at`)
   and an observation holding one `Order` and no `OrderPlaced` value (M14
   run 1) (FR-366-AC-6).
5. Fixture plus `TaxedMoney` (supertype `Money`, field `tax_minor`), `quoted`
   holding `{amount_minor: -5, currency: "EUR", tax_minor: 20}`; and a record
   `Pair { first: Money; second: Money }` as a field `pair` of `Order` holding
   `{first: {-1, ..}, second: {2, ..}}` (FR-366-AC-7).
6. Admit step 1 with a `pre` document as `snapshot`, then with `i1` missing
   `amount`.
7. Evaluate step 1 with a work budget that denies the third charge, and with
   `Money/Stable`, an `ocl` clause, added (FR-366-AC-8).

## Expected Results

1. Admission names `s1`, `i1`'s `pre` snapshot and the parameter `amount`.
   Six results of `NonNegative`: `Completed(true)` at `o1.total` and at
   `o1.history[0]`; `Completed(false)`, a Boolean with no code or cause, at
   exactly these four paths, in this order: `(s1, object o1)` `[member
   discount, option_value]`; `(s1, object o1)` `[member history, index 1]`;
   `(s1, object o1)` `[member quoted]`; `(pre observation of i1, invocation
   i1 parameter amount)` with no steps. The outcome is `False`.
2. The six results are equal in value and order; two evaluations give equal
   charge logs.
3. The result at `o1.quoted` is unchanged, and `o1.history[0]` is
   `Completed(true)` still.
4. No result for `Occurred` and outcome `True`.
5. The `TaxedMoney` value at `quoted` is evaluated against `TaxedMoney`'s
   effective invariant `NonNegative` and is `Completed(false)` at `[member
   quoted]`; `pair` gives a `Completed(false)` at `[member pair, field first]`
   and a `Completed(true)` at `[member pair, field second]`.
6. The `pre` document refuses `wrong_snapshot`/`wrong-observation`; the
   missing parameter refuses `invalid_runtime_input`/`missing-member`.
7. The evaluation returns the first two results and `Incomplete` with
   `resource_exhausted`/`insufficient-next-charge`, and no later result; with
   the `ocl` clause added, the six results of step 1 are unchanged and none is
   for `Stable`.
