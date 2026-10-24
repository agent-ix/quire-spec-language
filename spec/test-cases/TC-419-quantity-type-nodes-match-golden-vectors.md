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

Scope: FR-094-AC-6, FR-094-AC-7 (its compound-unit and declared-unit cases), FR-094-AC-8, FR-094-AC-11, FR-094-AC-12, FR-094-CON-1.

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
6. Over FR-093-AC-36's units, check and lower `a / 3 ms` over `a: Quantity<m>`,
   `d / 3 ms` over `d: Quantity<ms>`, `d / 3 s`, `d / rational(-3, 1) ms`,
   `d / -(3 ms)`, `a / -(3 ms)` and `a / 0.5 s`, and read the result type node
   of each division.
7. With `acme/rpc` (alias `R`) and `acme/flow` (alias `F`) each declaring base
   dimension `Time` and a root unit, emit the unit of FR-094-AC-12 with
   `dimension R::Time ~ F::Time by 1 R::s = 1000 F::ms;`, with
   `dimension F::Time ~ R::Time by 1000 F::ms = 1 R::s;` in its place, and with
   `dimension R::Time ~ Time by 1 R::s = 1 s;` added; then with a third package
   `acme/clock` (alias `G`, root `ms`) `dimension R::Time ~ G::Time by 1 R::s = 1000 G::ms;`
   and its reversed spelling; read the equivalence nodes and the `Time` nodes,
   read each package back through IR's v2 reader, and read the same packages
   with the node's `root_scale` set to `0/1`, `-1/1`, `2000/2`, removed, and
   with the base ratios not multiplying to one.

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
- Step 6: `a / 3 ms` has one `scalar_type`/`compound_unit` node with no
  `declaration` and no `owner`, whose body holds the `m` term with exponent
  `1` and the `s` term with exponent `-1` in ascending root-unit key order;
  `a / 0.5 s` and `a / -(3 ms)` have the same node. `d / 3 ms`, `d / 3 s`,
  `d / rational(-3, 1) ms` and `d / -(3 ms)` have one node whose body is `aggregate{[]}`. None
  of the nodes is the `m` unit node.
- Step 7: one `scalar_type`/`dimension_equivalence` node with no
  `declaration`, self-typed, preimage ids in ascending digest order, key
  `bbf770ea…00d4` and `root_scale` 1/1 under QSpec's vector owner, exactly the two
  package `Time` keys as dependencies and one `anchor` occurrence of ordinal 0; the
  reversed spelling gives the same key; the added `R::Time ~ Time` is a second node of
  ordinal 1; the `acme/clock` pair has `root_scale` 1000/1 and key `d707eb94…6c24` in
  both spellings; the `Time` nodes keep their keys; the node is emitted with no
  quantity type naming either dimension; IR admits each package and refuses each
  changed `root_scale` with `invalid_semantic_graph`.
