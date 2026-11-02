---
id: TC-419
title: "A declared unit's quantity type is its unit node, and a compound unit is its own node"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: verifies
---
# TC-419: A declared unit's quantity type is its unit node, and a compound unit is its own node

## Description

Verify that a quantity of a declared unit is typed by that unit's QSpec
nominal node with no wrapper node, that a compound unit's type node lists its
terms in ascending root-unit node-id order, and that a compound unit the
check stage's unit scope does not hold is an internal fault.

This catches a wrapper node that splits a declared unit's quantity type from
its unit, terms emitted in source order instead of canonical order, and a
single-term compound unit merged with its declared unit.

Scope: FR-094-AC-6, FR-094-AC-7 (its compound-unit and declared-unit cases), FR-094-AC-8, FR-094-AC-11, FR-094-AC-13, FR-094-CON-1.

## Test Procedure

The unit table admits QSpec's `dimension-length`, `dimension-time`,
`unit-metre` and `unit-second` nominal nodes, read at run time from
`QSPEC_DIR` (the opt-in `make conformance` gate).

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
   `a / rational(-3, 1) ms` and `a / 0.5 s`, and read the result type node
   of each division.
7. With `acme/rpc` (alias `R`) and `acme/flow` (alias `F`) each declaring base
   dimension `Time` and a root unit, emit the unit of FR-094-AC-13 with
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

- Step 1: the semantic type is QSpec's `unit-metre` node, and the
  graph holds no `compound_unit` node. It holds the `metre` node, a
  `scalar_type`/`unit` node typed by the `dimension-length` node, which it also
  holds as a `scalar_type`/`dimension` node.
- Step 2: `a * a`'s result type is the `compound_unit` node of `metre^2`; the
  three formed units are the `compound_unit` nodes of `metre^1 second^-1`
  (whose first term names metre), the dimensionless unit and `metre^1`. The
  `metre^1` node differs from the `metre` unit node.
- Step 3: an internal fault naming the `UnitId`, and no key; for the table
  with no admitted nodes, an internal fault naming `metre`, and no
  node.
- Step 4: no `_` arm.
- Step 5: no node is omitted; `km` depends on `Length` and `metre` and
  targets `metre` at scale `1000`; `Velocity` depends on `Length`
  and `Time` and holds both terms; `mps` is typed by
  `Velocity`; IR admits the package.
- Step 6: `a / 3 ms` has one `scalar_type`/`compound_unit` node with no
  `declaration` and no `owner`, whose body holds the `m` term with exponent
  `1` and the `s` term with exponent `-1` in ascending root-unit node-id order;
  `a / 0.5 s` and `a / rational(-3, 1) ms` have the same node. `d / 3 ms`, `d / 3 s` and
  `d / rational(-3, 1) ms` have one node whose body is `aggregate{[]}`. None
  of the nodes is the `m` unit node.
- Step 7: one `scalar_type`/`dimension_equivalence` node with no
  `declaration`, self-typed, relating the two package `Time` nodes in ascending
  node-id order with `root_scale` 1/1, exactly the two
  package `Time` nodes as dependencies and one `anchor` occurrence of ordinal 0; the
  reversed spelling gives the same node; the added `R::Time ~ Time` is a second node of
  ordinal 1; the `acme/clock` pair has `root_scale` 1000/1 in
  both spellings; the `Time` nodes are unchanged; the node is emitted with no
  quantity type naming either dimension; IR admits each package and refuses each
  changed `root_scale` with `invalid_semantic_graph`.
