---
id: TC-1140
title: "A state-graph claim emits a v2 claim node the I2 reader resolves"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-410
    type: verifies
---
# TC-1140: A state-graph claim emits a v2 claim node the I2 reader resolves

## Description

Verify that S4 emits each checked state-graph claim as a QSpec FR-475
`claim`/`state_graph` node, that the form and predicates enter the node id,
and that the I2 reader resolves the claim by its triple and refuses a
malformed node.

Scope: FR-410-AC-1 to FR-410-AC-4.

## Test Procedure

Use ADR-022 §7's ConfigVersion and game units.

1. Compile the units and read `ReachesTwo`, `CanStillWin` (with
   `from (x.phase != Lost)`) and `InOneWay` back through the I2 reader.
2. Compile `possible P` and `always possible P` over one `P`; `CanStillWin`
   without `from` and with `from (true)`; `possible Anything using v { true }`.
3. Resolve `ReachesTwo`'s triple in the recompiled package; resolve a triple
   with an unknown NodeId, with a `temporal`/`temporal_clause` node's NodeId,
   and with another ordinal.
4. Edit `ReachesTwo`'s node to `kind` `eventually-possible`, and separately
   to `target` a reference to `c`'s parameter node, recompute its key, and
   feed each package to the I2 reader.

Tag the tests `#[trace("TC-1140", "FR-410-AC-n")]` with the AC each step
backs.

## Expected Results

- Step 1 (FR-410-AC-1): three `claim`/`state_graph` nodes with `kind`
  `possible`, `always-possible` and `unique-path`; `ReachesTwo`'s `over`
  references `c`'s parameter node and its `from` is the empty aggregate;
  `CanStillWin`'s `from` references the node of `x.phase != Lost`; every
  occurrence has role `claim`; the reader returns each form, its predicates
  and `over` node; no claim is refused as not emitted.
- Step 2 (FR-410-AC-2): different node ids and `package_id`s for the first
  pair; one node id for the second pair; `over` the empty aggregate for the
  third.
- Step 3 (FR-410-AC-3): the claim is found; each other lookup returns
  `clause-root-unresolved` at its triple.
- Step 4 (FR-410-AC-4): `invalid_package`/`invalid-value` at the `kind`
  binding; `ill_typed`/`operator-ineligible` at the target.
