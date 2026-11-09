---
id: TC-1394
title: "The published matrix cases give their expected documents, stable and independent of the working directory"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-565
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-564
    type: verifies
---
# TC-1394: The published matrix cases give their expected documents, stable and independent of the working directory

## Description

Verify the cell positions of `lifecycle_matrix()`, that every case's document
has its expected category, exit code and source identities and serializes
stably, and that the working directory changes nothing.

Scope: FR-565-AC-1 to FR-565-AC-3 and FR-564-AC-7.

## Test Procedure

1. Read `lifecycle_matrix()`; check the cell count, order, and `Case` and
   `NotApplicable` positions against FR-564's table.
2. For every `Case`, call `run_case` twice; compare the two `OutcomeDocument`
   values; check the document's `operation`, `category` and exit code, or the
   item's category and exit code for a `prove` case; check that the exit code
   is the one FR-285's function returns for the category; check each source
   identity against `SourceIdentity::new("quire", <the case's file name>)`;
   serialize each document or item twice through FR-286 and compare the
   bytes.
3. Repeat step 2's `run_case` call in a process whose working directory is
   an empty temporary directory.

Tag the tests `#[trace("TC-1394", "<AC id>")]`.

## Expected Results

- Step 1: 48 cells, 40 `Case` and 8 `NotApplicable` at FR-564's positions.
- Step 2: equal documents; each category and exit code equal the case's
  expected ones and FR-285's mapping; every source identity is the case's
  `quire` identity; the two serializations are byte-equal.
- Step 3: the same `OutcomeDocument` as step 2.
