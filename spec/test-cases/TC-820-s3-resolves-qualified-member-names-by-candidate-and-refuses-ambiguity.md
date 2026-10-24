---
id: TC-820
title: "S3 resolves qualified member names by candidate and refuses ambiguity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-317
    type: verifies
---
# TC-820: S3 resolves qualified member names by candidate and refuses ambiguity

## Description

Scope: FR-317-AC-3.

## Test Procedure

Source vectors (one program each):

1. `q::m` with a union `q` declaring `m` and an import alias `q` exporting
   `m`.
2. `q::m` with the union `q` alone.
3. A unit declaring two unions named `q`, and a unit declaring a union `q`
   and an enum `q`.
4. `Kind::Empty` for an enum, and `Pair(1, 2)` for a tuple, with unions
   declared in the unit.

Hand-built scope vectors (the assembler refuses duplicate type names before
resolution, so a source program cannot produce them):

5. `q::m` with a union `q` and an enum `q` both declaring `m`.
6. `q::m` with two unions `q` both declaring `m`.
7. `q::m` with two unions `q`, only one declaring `m`.

Tag each test `#[trace("TC-820", "<AC id>")]`.

## Expected Results

- Step 1: `ambiguous_declaration`/`ambiguous-name`, naming both candidates.
- Step 2: resolves to the union's member.
- Step 3: `ambiguous_declaration`/`ambiguous-name` at the second
  declaration, from the assembler.
- Step 4: an enum member and a tuple construction, as without unions.
- Steps 5 and 6: `ambiguous_declaration`/`ambiguous-name`, naming every
  candidate.
- Step 7: resolves to the declaring union's member.
