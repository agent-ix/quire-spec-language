---
id: TC-1049
title: "A type-level clause is classified as an invariant, a guard or an unevaluated clause, and an invariant is parsed and checked"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-366
    type: verifies
---
# TC-1049: A type-level clause is classified as an invariant, a guard or an unevaluated clause, and an invariant is parsed and checked

## Description

Verify FR-366's classification of a domain package's type-level clauses, the
effective invariants of a type, and the parse and check of an invariant: the
Boolean root, the `pre` refusal and the clause location. These are QSpec
[TC-235](ix://agent-ix/quire-specification/TC-235) M06 run 4, M13 run 2 and
M14 run 2, with M03 run 4, M17 run 1 and M19 runs 1 and 7 for the effective
invariants. Scope: FR-366-AC-1, FR-366-AC-2 and FR-366-AC-3.

Catches a classification that calls a guard an invariant, one that drops an
inherited invariant, a `pre` read in a type-level clause that is accepted, a
refusal located at the clause's text instead of `TypeDefinition.clauses[k]`,
and an `ocl` clause that is parsed.

## Test Procedure

Admit TC-235's K2 package (21 charges) with the changes below, compile it in a
`1-draft` unit with the spine driver passing the FR-146 parser to
`with_parsed_type_invariants`, and read each clause's role and each type's
effective invariants. Tag the tests `#[trace("TC-1049", "FR-366-AC-n")]`.

1. K2 plus the `quire` clause `Extra` (`self.placed or not self.placed`) on
   `OrderLifecycle`, which no transition names (M06 run 4). Read the roles of
   `Extra` and `NotYetPlaced` and the effective invariants of
   `OrderLifecycle`.
2. K2 plus `ExpressLifecycle` as in M19 run 1, and `Extra` on `OrderLifecycle`.
   Read the roles and the effective invariants of `ExpressLifecycle`.
3. K2 with `Money/NonNegative` in `ocl` (M03 run 4), then in `quire`
   (`self.amount_minor >= 0`) plus `TaxedMoney` (M17 run 1). Read the roles and
   the effective invariants of `Money` and `TaxedMoney`.
4. Read the package of step 1 with its clause records in reverse document order.
5. K2 with `Money/NonNegative` as `pre(self.amount_minor) >= 0` (M13 run 2).
6. K2 with `OrderPlaced/Occurred` as `self.occurred_at = self.occurred_at`,
   then as `pre(self.order_id) = self.order_id` (M14 runs 1 and 2).
7. K2 with `NonNegative` as `self.amount_minor`, then as
   `self.amount_minor >=`, then in `ocl` as `this is not quire (`.
8. K2 with `Money` given the clauses `Stable` (`true`) and `NonNegative` as
   `pre(self.amount_minor) >= 0`, in that order.

## Expected Results

1. `Extra` is `Invariant`; `NotYetPlaced` is `Guard` and is in no effective
   invariant list; the effective invariants of `OrderLifecycle` are `[Extra]`.
2. `NotYetPlaced` is `Guard` for `ExpressLifecycle`; its effective invariants
   are `[Extra]`.
3. In `ocl` the clause is `Unevaluated`, kept with its text and span; in
   `quire` it is `Invariant`. The effective invariants of `Money` and of
   `TaxedMoney` are each `[NonNegative]`.
4. The roles and effective invariants equal those of step 1.
5. The compile refuses with one refusal, `wrong_snapshot`/`forbidden-pre-read`,
   naming `ix://test/orders/Money/NonNegative` at `TypeDefinition.clauses[0]`,
   and produces no checked package.
6. The first compile succeeds with `Occurred` checked as an invariant of
   `OrderPlaced`; the second refuses `wrong_snapshot`/`forbidden-pre-read`
   naming `ix://test/orders/OrderPlaced/Occurred` at
   `TypeDefinition.clauses[0]`, and produces no checked package.
7. The first refuses `ill_typed`/`non-boolean-root` at
   `TypeDefinition.clauses[0]`, the second the parser's refusal at the same
   location, and the third succeeds: an `ocl` text is never parsed.
8. The refusal of `NonNegative` is located at `TypeDefinition.clauses[1]`.
