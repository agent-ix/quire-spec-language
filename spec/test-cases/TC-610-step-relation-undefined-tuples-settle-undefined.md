---
id: TC-610
title: "An HP-1 relation with undefined tuples and no refuting tuple settles undefined"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-179
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: verifies
---
# TC-610: An HP-1 relation with undefined tuples and no refuting tuple settles undefined

## Description

Verify that tuples on which a step relation's body has no value count neither as passes nor as refutations, and that an item with undefined tuples and no refuting tuple settles `undefined`, naming the first undefined tuple in canonical order and its cause.

Scope: FR-179-AC-4, FR-182-AC-5.

## Test Procedure

Fixture: the `Cell` model of FR-179-AC-4 and the relation `R` (`1 / x.k >= y.k`).

1. Run `R` through EN-1.
2. Settle its outcome.

Tag the tests `#[trace("TC-610", "<AC id>")]`.

## Expected Results

- Step 1: `Undefined` naming the first tuple in canonical order with `x.k = 0` and the division-by-zero cause; never `Holds`.
- Step 2: `undefined`, O-16 category undefined, with a record naming that tuple, its two executions and the cause; never `proved`.
