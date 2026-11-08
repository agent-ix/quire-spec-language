---
id: TC-1394
title: "The published matrix cases give their expected documents, byte-stable and self-contained"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-565
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-564
    type: verifies
---
# TC-1394: The published matrix cases give their expected documents, byte-stable and self-contained

## Description

Verify the shape of `lifecycle_matrix()`, that every case's document has its
expected category and exit code and serializes stably, and that no case reads
a file.

Scope: FR-565-AC-1 to FR-565-AC-3 and FR-564-AC-7.

## Test Procedure

1. Read `lifecycle_matrix()`; check the cell count, order, `Case` and
   `NotApplicable` positions and reasons against FR-564's table.
2. For every `Case`, call `run_case` twice; compare the bytes; check the
   document's `operation`, `category` and exit code, or the item's category
   and exit code for a `prove` case; check that the exit code is the one
   FR-285's function returns for the category.
3. Repeat step 2 in a process whose working directory is an empty temporary
   directory, with file opens observed.

Tag the tests `#[trace("TC-1394", "<AC id>")]`.

## Expected Results

- Step 1: 48 cells, 40 `Case` and 8 `NotApplicable` at FR-564's positions,
  each n/a with its reason.
- Step 2: equal bytes; each category and exit code equal the case's expected
  ones and FR-285's mapping.
- Step 3: the same bytes, and no case opens a file.
