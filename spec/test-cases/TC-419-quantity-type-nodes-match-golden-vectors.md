---
id: TC-419
title: "A declared unit's quantity type is its unit node, and a compound unit's keys to the golden vectors"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: verifies
---
# TC-419: A declared unit's quantity type is its unit node, and a compound unit's keys to the golden vectors

## Description

Verify that a quantity of a declared unit is typed by that unit's QSpec
nominal node with no wrapper node, that a compound unit's type node lists its
terms in the compound-unit preimage's order, and that a compound unit the
check stage's unit scope does not hold is an internal fault.

This catches a wrapper node that splits a declared unit's quantity type from
its unit, terms emitted in source order instead of canonical order, and a
single-term compound unit merged with its declared unit.

Scope: FR-094-AC-6, FR-094-AC-7 (its compound-unit case), FR-094-CON-1.

## Test Procedure

The unit table admits QSpec's `dimension-length`, `dimension-time`,
`unit-metre` and `unit-second` preimages from `node-identity-vectors.json`,
read at run time from `QSPEC_DIR` (the opt-in `make conformance` gate).

1. Check a parameter typed as a quantity of the declared unit `metre`. Read
   its semantic type, and list the graph's `compound_unit` nodes.
2. Over `a` (a quantity of `metre`) and `t` (a quantity of `second`), check
   `a * a`, `a / t`, `a / a` and `a * a / a`, and read each result type node.
3. Key a quantity type whose compound `UnitId` the check stage's unit scope
   does not hold.
4. Scan the `UnitId`-domain `match` for a `_` arm.

Tag the tests `#[trace("FR-094-AC-n", "TC-419")]` with the AC each backs.

## Expected Results

- Step 1: the semantic type is QSpec's `unit-metre` key,
  `79637623a46d29e884b62c6fa292aeb29d41e4ecc4e800b4d7ee910a3eaf23a4`, and the
  graph holds no `compound_unit` node.
- Step 2: U1, U2 (whose first term names metre), U3 and U4, byte for byte.
  U4 differs from the `metre` unit key.
- Step 3: an internal fault naming the `UnitId`, and no key.
- Step 4: no `_` arm.

## Status

Planned; QSL-156 A4b.
