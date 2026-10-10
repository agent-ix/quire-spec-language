---
id: TC-827
title: "Union nodes lower and emit in QSpec's spelling and survive the I2 read and recompile"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-320
    type: verifies
---
# TC-827: Union nodes lower and emit in QSpec's spelling and survive the I2 read and recompile

## Description

Scope: FR-320-AC-1, FR-320-AC-2.

## Test Procedure

1. Compile and emit a package with `union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }`, `area` (FR-318-AC-1) and a function returning
   `Shape::Rect(2, 3)`; read it back through I2.
2. Recompile the digest-addressed source through replay's S1 to S4.
3. Emit the same package against an IR build that cannot decode the union
   (tag, form).

Tag each test `#[trace("TC-827", "<AC id>")]`.

## Expected Results

- Step 1: v2 nodes in QSpec's union spelling; each `WireNodeId` equals the
  emitted id; member identities unchanged.
- Step 2: recompiled `package_id` equals the emitted one.
- Step 3: the union node and every node naming it are omitted with
  `UnsupportedForm`; no partial body is written.

