---
id: TC-984
title: "Request items preserve each occurrence's path condition"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
---
# TC-984: Request items preserve each occurrence's path condition

## Description

Verify that `route`'s request builder preserves the canonical path-condition
sequence attached to each requirement record. Two records for one application
node must remain separate items with their own guard sequence. Scope:
FR-075-AC-9.

## Test Procedure

Build a requirement-record map with these entries, in occurrence-key order:

| Record | Application occurrence key | Canonical path condition |
|--------|----------------------------|--------------------------|
| 1 | `OccurrenceKey(appNode, Origin(expression, 0))` | `[PathGuard(OccurrenceKey(guardNodeA, Origin(expression, 0)), true)]` |
| 2 | `OccurrenceKey(appNode, Origin(expression, 1))` | `[PathGuard(OccurrenceKey(guardNodeB, Origin(expression, 1)), false)]` |

Use distinct guard nodes `guardNodeA` and `guardNodeB`; both records otherwise
have valid requirement data. Run `items_from_requirements` with a registry
that supplies the same candidate outcome for both records. Also run it with a
single unguarded record.

## Expected Results

The builder returns two items for the first vector, in occurrence-key order.
Both items have `appNode` as their node and distinct occurrence keys. Each
item's `path_condition()` equals the exact canonical sequence in its source
record: item 1 has the `guardNodeA` guard with `holds == true`, and item 2 has
the `guardNodeB` guard with `holds == false`. The sequence order and guard
members are preserved; the builder does not flatten, deduplicate, or
substitute one item's guards for the other's. The unguarded record produces
an item whose path condition is empty.
