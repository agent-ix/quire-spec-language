---
id: TC-463
title: "The state package reads back through I2, keeps its identity rules and emits all or nothing"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: verifies
---
# TC-463: The state package reads back through I2, keeps its identity rules and emits all or nothing

## Description

Verify QSL's I2 read of the emitted state package, identity and cardinality
rules under edits, anchoring of inherited operations, and all-or-nothing
emission.

Scope: FR-105-AC-3, FR-105-AC-4, FR-105-AC-6.

## Test Procedure

1. Read TC-462's emitted bytes through QSL's `checked_v2` I2 reader, and
   recompute the `package_id`.
2. Compile the unit twice. Then rename `ParentOrder` to `ParentFirst`; then
   change its `<` to `<=`; then add `ParentOrder2` with `ParentOrder`'s body;
   then add `post Also using v on Config::ConfigVersion::attemptUpdate
   { result }`.
3. Add object type `Sub` with supertype `ConfigVersion` to the fixture
   package, and compile a unit with
   `pre A using v on Config::ConfigVersion::attemptUpdate { true }` and
   `pre B using v on Config::Sub::attemptUpdate { true }`.
4. Inject an emitter fault at the `frame` node (a test-only hook in the
   `ProtocolClause` emission arm) and compile.

Tag the tests `#[trace("TC-463", "FR-105-AC-n")]`.

## Expected Results

- Step 1: the read admits the package, including its frame step, and the
  recomputed `package_id` equals the emitted one.
- Step 2: identical bytes twice; the rename changes no node id; `<=` changes
  `ParentOrder`'s `state_clause` node id and the `package_id`;
  `ParentOrder2` adds no node and gives `ParentOrder`'s node a second `claim`
  occurrence, ordinal 1; `Also` adds one `state_clause` node and no second
  anchor or frame.
- Step 3: one `operation_anchor` and one `frame`; the anchor's context is the
  `ConfigVersion` node; both clauses reference that anchor.
- Step 4: the compile refuses; no package bytes and no `state` node are
  emitted.

## Status

Planned (QSL-273). Steps 1 to 3 are pending STD-111: QSL's reader admits
these bodies once the QSpec body rules land, and the node ids and
`package_id` steps 2 and 3 compare have STD-111 spellings in their
preimages. Step 4 does not wait on it.
