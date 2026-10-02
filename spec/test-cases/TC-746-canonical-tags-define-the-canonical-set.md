---
id: TC-746
title: "Canonical doc tags define the canonical set, and misplaced or duplicate tags fail"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-271
    type: verifies
---
# TC-746: Canonical doc tags define the canonical set, and misplaced or duplicate tags fail

## Description

Verify FR-271: the gate reads `/// quire:canonical` exactly, from anywhere
in an item's doc comment, and reports a tag on the wrong item or on two
definitions.

Scope: FR-271-AC-1 and FR-271-AC-2.

## Test Procedure

Build a fixture workspace of two crates, A and B, and run
`canonical-types` over it.

1. A: `pub struct Meter` with a doc comment of a prose line, the tag line
   and a prose line. B: `struct Meter`.
2. Repeat step 1 with A's tag removed; written as `// quire:canonical`;
   written as `/// quire:canonical type`.
3. In A: the tag on a `pub fn`; on a private `struct`; on a struct field.
4. The tag on `pub struct Meter` in both A and B.

Tag the tests `#[trace("TC-746", "FR-271-AC-n")]`.

## Expected Results

- Step 1: one `identifier` finding naming A's and B's definitions.
- Step 2: no finding, three times.
- Step 3: one `tag` finding per run, naming the file, line and item.
- Step 4: one `tag` finding naming both locations.
