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

Check this source fixture and run `items_from_requirements` over its checked
package's requirement records with a registry that supplies the same
candidate outcome for both records:

```text
function f using v(x: Int[0, 9], p: Boolean): Integer pure {
  if p then x + 1 else x + 1
}
```

The two `x + 1` occurrences are the vector entries. Select their records by
the common application node and verify that their occurrence keys differ.
Also check an unguarded function `function g using v(x: Int[0, 9]): Integer
pure { x + 1 }` and run the builder over its record.

## Expected Results

The builder returns two items for the first fixture, in occurrence-key order.
Both items have the same application node and distinct occurrence keys. Each
item's `path_condition()` equals the exact canonical sequence in its source
record: both contain the `if p` guard occurrence, with `holds == true` for
the then occurrence and `holds == false` for the else occurrence. The sequence
order and guard members are preserved; the builder does not flatten,
deduplicate, or substitute one item's guards for the other's. The unguarded
record produces an item whose path condition is empty.
