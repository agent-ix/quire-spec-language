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

Verify QSL's I2 read of the emitted state package, identity stability under
edits, and all-or-nothing emission.

Scope: FR-105-AC-3, FR-105-AC-4, FR-105-AC-6.

## Test Procedure

1. Read TC-462's emitted bytes through QSL's `checked_v2` I2 reader, and
   recompute the `package_id`.
2. Compile the unit twice. Then rename `ParentOrder` to `ParentFirst`; then
   change its `<` to `<=`; then add
   `post Also using v on Config::ConfigVersion::attemptUpdate { result }`.
3. Inject an emitter fault at the `frame` node (a test-only hook in the
   `ProtocolClause` emission arm) and compile.

Tag the tests `#[trace("TC-463", "FR-105-AC-n")]`.

## Expected Results

- Step 1: the read admits the package, including its frame step (QSpec
  FR-340), and the recomputed `package_id` equals the emitted one.
- Step 2: identical bytes twice; the rename changes no node id; `<=` changes
  `ParentOrder`'s `state_clause` node id and the `package_id`; `Also` adds one
  `state_clause` node and no second anchor or frame.
- Step 3: the compile refuses; no package bytes and no `state` node are
  emitted.

## Status

Planned (QSL-273). Step 1 needs QSL's reader to carry the `state_clause` and
`operation_anchor` body rule FR-105 proposes; the IR reader at 48ab5dc is not
asked to read these bodies here (FR-105 Dependencies).
