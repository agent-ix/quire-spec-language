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

Passes (QSL-279). Step 2's double-compile identity case is covered by
`s4_state_package_emission_is_stable_across_compiles`. Step 1
(`s4_state_package_reads_back_through_i2`) passes: the package reads back
through I2, including the reference-edge check of its
`quire.op.model.reaches_field` application. QSL-312 covers
the rest of step 2 (`s4_renaming_parent_order_changes_no_node_id`,
`s4_changing_parent_orders_comparison_changes_its_node_id_and_the_package_id`,
`s4_parent_order2_with_parent_orders_body_adds_no_node_and_a_second_claim`,
`s4_a_second_post_on_attempt_update_adds_one_clause_node_and_no_anchor_or_frame`)
and step 3
(`s4_pre_clauses_via_config_version_and_sub_share_one_anchor_at_config_version`).
Step 4 (fault-injected all-or-nothing emission) is covered
(QSL-313): `qsl-package`'s `a_fault_injected_partway_through_node_emission_writes_nothing`
proves the mechanism generically, and `qsl-replay`'s
`a_fault_on_the_frame_node_refuses_the_whole_config_version_package` drives
it at the `frame` node of a compiled ConfigVersion state-clause package.
