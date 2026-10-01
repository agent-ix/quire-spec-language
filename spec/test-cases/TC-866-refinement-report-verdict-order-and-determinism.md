---
id: TC-866
title: "The refinement report orders every result, lists every regression, and gives FR-301's verdict and exit"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-344
    type: verifies
---
# TC-866: The refinement report orders every result, lists every regression, and gives FR-301's verdict and exit

## Description

Verify FR-344's verdict precedence, that a regression is listed under any
verdict, ordering by case name, and byte-equal reports across runs.

Scope: FR-344-AC-1 to FR-344-AC-3.

## Test Procedure

1. Feed the report function five result sets built in the test: only
   `holds`; `holds` and a `regression`; a `regression` and an `unresolved
   (incomplete)`; that plus an `unresolved (unsupported)`; that plus a
   `tool failure`.
2. Read the report of the fourth set.
3. Build a corpus of three identity pairs, creating their directories in
   the reverse of their names' order. Run the gate twice.

Each expected verdict and exit is a literal in the test. Tag the tests
`#[trace("TC-866", "FR-344-AC-n")]`.

## Expected Results

- Step 1: success/0, violation/10, incomplete/22, unsupported/21, tool
  failure/30.
- Step 2: the `regression` is listed with its name, both classes and the
  superseding codes.
- Step 3: cases are ordered by case name, not by creation order; the two
  reports are byte-equal.
