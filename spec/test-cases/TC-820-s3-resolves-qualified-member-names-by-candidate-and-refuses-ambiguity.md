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

1. `q::m` with a union `q` and an enum `q` both declaring `m`.
2. `q::m` with two unions `q` both declaring `m`.
3. `q::m` with a union `q` declaring `m` and an import alias `q` exporting
   `m`.
4. `q::m` with two unions `q`, only one declaring `m`.
5. `Kind::Empty` for an enum, and `Pair(1, 2)` for a tuple, with unions
   declared in the unit.

Tag each test `#[trace("TC-820", "<AC id>")]`.

## Expected Results

- Steps 1 to 3: `ambiguous_declaration`/`ambiguous-name`, naming every
  candidate.
- Step 4: resolves to the declaring union's member.
- Step 5: an enum member and a tuple construction, as without unions.
