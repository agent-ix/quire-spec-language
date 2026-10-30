---
id: TC-1194
title: "The control-to-temporal activation map is derived from the protocol nodes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-449
    type: verifies
---
# TC-1194: The control-to-temporal activation map is derived from the protocol nodes

## Description

Verify that a protocol's control-to-temporal activation map holds one row
per required temporal clause, naming the protocol clause, its activation and
the temporal clause; that it is derived from the `protocol_clause` node alone
and equal in-process and after the I2 read-back; that no clock, profile,
name or position adds a row; and that a package whose required temporal
clause or trigger is not in the graph gives a refusal and no map. Scope:
FR-449-AC-1 to FR-449-AC-6.

Catches a row inferred from a shared clock, a duplicate row for a clause
required twice, a map that depends on source order or formatting, a map
that differs after the read-back, and a partial map returned with a reader
refusal.

Fixture **W** is TC-1061's.

## Test Procedure

| # | Unit | Expected |
| --- | --- | --- |
| 1 | **W** | one row: the `OrderFlow` clause key, `Each` with `opened`'s node key, `OrderClosureBounded`'s node key |
| 2 | **W** with `activation on origin` | one row: the clause key, `Origin` with no trigger, `OrderClosureBounded`'s node key |
| 3 | **W** with no `requires temporal` | an empty map |
| 4 | **W** with a second `requires temporal OrderClosureBounded` | the map of step 1 |
| 5 | **W** with a temporal clause `OrderPaidBounded` and `requires temporal OrderPaidBounded` | two rows, in ascending node-key order of the two temporal clause nodes |
| 6 | **W** with a temporal clause `RefundBounded` on the clock `refund-clock`, not required by the protocol | the map of step 1 |
| 7 | **W**, compiled twice; **W** reformatted with a comment added; **W**'s emitted package read back through the I2 reader | every map equals step 1's |
| 8 | **W**'s emitted package with the `requirements` entry naming a node absent from the clause's `dependencies`; separately, with the `each` trigger naming such a node | the I2 reader refuses `missing_declaration`/`missing-name` at the target; no map |
| 9 | **W** with `requires temporal Missing` and no temporal clause `Missing` | S3 refuses `missing_declaration`/`missing-name` at the name; no package, no map |

Tag the tests `#[trace("TC-1194", "FR-449-AC-n")]`: steps 1 for AC-1, 2
to 4 for AC-2, 5 and 6 for AC-3, 7 for AC-4, 8 for AC-5 and 9 for AC-6.

## Expected Results

- Each row asserts typed fields: node keys are compared as keys, the
  activation as its variant and trigger key.
- No step reads a map from any carrier other than the `protocol_clause`
  node and the nodes it references.
