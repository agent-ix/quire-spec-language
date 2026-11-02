---
id: TC-1294
title: "The FR-042 successor map is complete and true"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-500
    type: verifies
---
# TC-1294: The FR-042 successor map is complete and true

## Description

Verify FR-500-AC-1 to AC-3. Catches a criterion with no row, a successor test
that moved or lost its tag, and a legacy file deleted while a GAP is open.

## Test Procedure

1. List FR-042-AC-1 to AC-15 and confirm each has a row in FR-500.
2. For each successor `file:line` in FR-500, open the file and confirm the
   successor criterion tag is at or within the test at that line.
3. For each deleted legacy file, confirm none of the GAPs named for its
   criteria is open.

## Expected Results

- Every criterion has a row; every successor location carries its tag; no
  deleted file backs an open GAP.
