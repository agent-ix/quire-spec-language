---
id: TC-524
title: "A checked temporal clause emits as a v2 temporal clause node and reads back"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: verifies
---
# TC-524: A checked temporal clause emits as a v2 temporal clause node and reads back

## Description

Verify that the S4 emitter writes a checked temporal clause as QSpec
FR-370's `temporal`/`temporal_clause` node, that the resolved fairness set
and intervals decide its identity, and that QSL's I2 reader admits the node
and refuses each defect with FR-370's code.

Scope: FR-337-AC-1 to FR-337-AC-4.

## Test Procedure

Use ADR-018 §6's ConfigVersion example unit.

1. Compile the unit and read the `ReachesTwo` node's body; read it back
   through the I2 reader.
2. Compile the clause with `fair weak attemptUpdate`, with `fair weak whole
   attemptUpdate`, with `fair weak each attemptUpdate`, and with
   `eventually[0,5]` in place of `eventually`; compare fairness bytes, node
   keys and `package_id`s.
3. Compile `eventually[0,5] holds(c.versionNumber = 2)` under
   event-position false-extension, and a clause with strong previous.
4. Feed the I2 reader step 1's node with its arguments reordered; with a
   `null` interval under a bounded profile; with a fairness member naming
   `Absent`; with a `quire.op.temporal.fair` application inside the
   formula.

Tag the tests `#[trace("TC-524", "FR-337-AC-n")]`.

## Expected Results

- Step 1: the body FR-337-AC-1 states; the I2 reader admits it.
- Step 2: the first two give byte-identical fairness applications and equal
  node keys; `each` against `whole` and the interval change each give a
  different node key and `package_id`.
- Step 3: `quire.op.temporal.eventually` with `{lower: "0", upper: "5"}` and
  an empty fairness argument; `quire.op.temporal.once` with `{lower: "1",
  upper: "1"}`.
- Step 4: a schema failure; `invalid_package`/`operation-member-mismatch`;
  `missing_declaration`/`missing-name`; `ill_typed`/`operator-ineligible`;
  each at FR-370's locus.

## Status

🚧 Planned.
