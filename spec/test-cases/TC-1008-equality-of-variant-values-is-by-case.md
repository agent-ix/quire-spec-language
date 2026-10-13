---
id: TC-1008
title: "Equality of variant values is by case, never a fault, and reaches nested values"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-360
    type: verifies
---
# TC-1008: Equality of variant values is by case, never a fault, and reaches nested values

## Description

Verify evaluation and equality of variant values. Scope: FR-360-AC-6 and FR-360-AC-7.

## Test Procedure

1. Evaluate `==` and `!=` over two cases, in both operand orders, over a field read and a literal of the same case.
2. Check `Config::Level::Gold == Config::Tier::Gold`.
3. Compare two `Settings` records, two `Option`, two `Sequence`, two `Set` and two tuples that differ only in the case.
4. Build a `Set<Config::Tier>` from `Gold, Gold, Standard` and from `Standard, Gold`, and a `Set` over cases `Standard`, `Gold`, `Bronze` in each source order.

## Expected Results

- Step 1 gives the Boolean of case equality with no checked-invariant refusal.
- Step 2 refuses `ill_typed`/`type-mismatch` at check.
- Step 3 compares by case at every depth.
- Step 4 yields two-element sets ordered `Gold`, `Standard` and a three-element set ordered `Bronze`, `Gold`, `Standard` whatever the source order.
- Assertions compare typed codes, causes, keys and paths.
