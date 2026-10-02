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

Scope: FR-094-AC-6, FR-094-AC-7 (its compound-unit and declared-unit cases), FR-094-AC-8, FR-094-CON-1.

## Test Procedure

The unit table admits QSpec's `dimension-length`, `dimension-time`,
`unit-metre` and `unit-second` preimages from `node-identity-vectors.json`,
read at run time from `QSPEC_DIR` (the opt-in `make conformance` gate).

1. Check a parameter typed as a quantity of the declared unit `metre`. Read
   its semantic type, and list the graph's `compound_unit` nodes.
2. Over `a` (a quantity of `metre`) and `t` (a quantity of `second`), check
   a function whose body holds `a * a`, and read its result type node. Form
   the compound units of `a / t`, `a / a` and `(a * a) / a` with
   `value::quantity::result_unit`, hold them in the unit scope, and key the
   quantity type node of each.
3. Key a quantity type whose compound `UnitId` the check stage's unit scope
   does not hold. Then build a unit table from the admitted graph's units
   through `FromIterator`, not `UnitTable::declared`, and key a `metre`
   quantity against it.
4. Scan the `UnitId`-domain `match` for a `_` arm.
5. Under FR-094-AC-8's source-declared graph (`Length`, `Time`,
   `Velocity = Length / Time`, `metre`, `second`, `mps`, `km = 1000 ×
   metre`), emit `Trip{d: km, v: mps}` and read it back through IR's v2
   reader.

Tag the tests `#[trace("FR-094-AC-n", "TC-419")]` with the AC each backs.

## Expected Results

- Step 1: the semantic type is QSpec's `unit-metre` key,
  `79637623a46d29e884b62c6fa292aeb29d41e4ecc4e800b4d7ee910a3eaf23a4`, and the
  graph holds no `compound_unit` node. It holds the `metre` node, a
  `scalar_type`/`unit` node carrying its `quire.unit-node/v1` preimage, whose
  bytes hash to that key, typed by the `dimension-length` node, which it also
  holds as a `scalar_type`/`dimension` node.
- Step 2: `a * a`'s result type is U1; the three formed units key to U2
  (whose first term names metre), U3 and U4, byte for byte. U4 differs from
  the `metre` unit key.
- Step 3: an internal fault naming the `UnitId`, and no key; for the table
  with no admitted nodes, an internal fault naming `metre`'s key, and no
  node.
- Step 4: no `_` arm.
- Step 5: no node is omitted; `km` depends on `Length` and `metre` and its
  preimage targets `metre` at scale `1000`; `Velocity` depends on `Length`
  and `Time` and its preimage holds both terms; `mps` is typed by
  `Velocity`; IR admits the package.

## Status

Implemented (#384). The tests back every step.
