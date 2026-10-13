---
id: TC-1000
title: "An optional field, a collection element and a payload accept a child and refuse a parent or an unrelated record"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-359
    type: verifies
---
# TC-1000: An optional field, a collection element and a payload accept a child and refuse a parent or an unrelated record

## Description

Verify that an `Option` payload and a collection element at a parent-typed slot admit a child and keep its identity. Scope: FR-359-AC-6, FR-359-AC-7.

Catches an implementation that admits the child at the field but faults on projection with `field-option-payload-rejected`, one that retags the element, and one that adds type-level covariance.

## Test Procedure

1. Declare `Money`, `TaxedMoney` (adds `tax_minor`) and an unrelated `Fee`. Build `Order { total: Money?; }` with `total` holding a `TaxedMoney`, then read `total`, `present(total)` and `value(total)`.
2. Build `Order` with `total` holding a `Money` at a `TaxedMoney?` field, and with a `Fee` at a `Money?` field.
3. Build `Sequence<Money>`, `Set<Money>`, `Bag<Money>` and `OrderedSet<Money>` literals holding a `TaxedMoney`, and run `contains`, iteration and `length`.
4. Build a `Set<Money>` from a `Money` and a `TaxedMoney` with equal shared fields, in both source orders.
5. Pass a `Sequence<TaxedMoney>` expression to a `Sequence<Money>` parameter and an `Option<TaxedMoney>` expression to an `Option<Money>` parameter.

## Expected Results

- Step 1 returns the same `TaxedMoney`, with `tax_minor` and its most-specific type intact, and reports no `field-option-payload-rejected`.
- Step 2 refuses `ill_typed`/`type-mismatch` for the `Money` at the `TaxedMoney?` field and for the `Fee`, naming the field.
- Step 3 admits each element and keeps its identity through every operation.
- Step 4 holds two elements in each order, with one canonical key.
- Step 5 refuses `ill_typed`/`type-mismatch` at each argument.
- Assertions compare typed values, causes and member paths, never message text.