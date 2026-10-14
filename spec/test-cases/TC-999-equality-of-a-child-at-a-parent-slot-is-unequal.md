---
id: TC-999
title: "Equality of a child at a parent-typed slot is false, never a fault"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-359
    type: verifies
---
# TC-999: Equality of a child at a parent-typed slot is false, never a fault

## Description

Verify that `==` and `!=` over parent-typed operands decide by most-specific type first. Scope: FR-359-AC-4, FR-359-AC-5.

Catches an implementation that faults with `EqualityOperandSourceNotAdmitted`, one that compares only the shared fields and reports a child equal to its parent, and one that applies the parent's text-leaf list to a child.

## Test Procedure

1. Declare `record Money { amount_minor: Int; currency: Text[3, 3; nfc] }` and `record TaxedMoney` specializing it with `tax_minor: Int`. With `a: Money` and `b: Money`, run `a == b` and `a != b` for `a` a `TaxedMoney` and `b` a `Money` with the same shared fields, and again with the operands swapped.
2. Declare `GiftTaxedMoney` specializing `TaxedMoney`. Compare a `GiftTaxedMoney` with a `Money`, with a `TaxedMoney` and with another `GiftTaxedMoney` of equal fields. Run both operators for two `TaxedMoney` values with equal effective fields, then with only `tax_minor` different.
3. Declare `record Pair { first: Money; second: Money }` and compare two `Pair` values whose `first` fields hold a `TaxedMoney` and a `Money` of equal shared fields, then two whose `first` fields hold equal `TaxedMoney` values.
4. Compare two `Option<Money>`, two `Sequence<Money>`, two `Set<Money>`, two tuples `(Money, Int)` and two union values with a `Money` payload, each pair holding a `TaxedMoney` and a `Money` of equal shared fields, then equal `TaxedMoney` values.
5. Check `c == b` with `c: TaxedMoney` and `b: Money`.

## Expected Results

- Step 1 gives `false` for `==` and `true` for `!=` in both operand orders, and reports no checked-invariant cause.
- Step 2 gives `false`, `false` and `true` for the three `GiftTaxedMoney` comparisons, equal for equal effective fields and unequal when `tax_minor` differs.
- Step 3 gives unequal for the mixed pair and equal for the pair of equal `TaxedMoney` values, with no text-leaf comparison on the mixed pair.
- Step 4 gives unequal for each mixed pair and equal for each pair of equal `TaxedMoney` values, with a Boolean and no fault.
- Step 5 refuses `ill_typed`/`type-mismatch` at the operands, as it does without this requirement, because the static types differ.
- Assertions compare typed outcomes and causes, never message text.