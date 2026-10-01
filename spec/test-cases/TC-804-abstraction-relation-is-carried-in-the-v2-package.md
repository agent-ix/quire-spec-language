---
id: TC-804
title: "The abstraction relation is emitted as a v2 node and enters the package_id"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-306
    type: verifies
---
# TC-804: The abstraction relation is emitted as a v2 node and enters the package_id

## Description

Scope: FR-306-AC-1, FR-306-AC-2.

## Test Procedure

1. Compile FR-304-AC-1's unit; read the relation node back from the emitted
   v2 bytes.
2. Recompile the unit unchanged; then change one `ObjectBinding` field's
   `RustField` and recompile.

## Expected Results

1. Exactly one relation node; its keys and binding values equal the
   in-process `CheckedAbstractionRelation`'s.
2. The unchanged recompile gives the same `package_id`; the changed one
   gives a different `package_id`.
