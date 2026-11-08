---
id: TC-1383
title: "Generated blocks call only kernel-table entries and their own blocks"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-561
    type: verifies
---
# TC-1383: Generated blocks call only kernel-table entries and their own blocks

## Description

Verify the call boundary from the code generator's relocation records, and
that the check catches a stray call.

Scope: FR-561-AC-3.

## Test Procedure

1. Prepare the JIT over each of the 10,000 FR-295 generated units at the
   default seed. For each generated block, read its relocation records and
   classify each call target.
2. Prepare one unit with a test code generator that emits one call to an
   address outside the kernel table and the prepared blocks, and run the same
   check.

Tag the tests `#[trace("TC-1383", "FR-561-AC-3")]`.

## Expected Results

- Step 1: every call target is a `KernelTable` entry or a block of the same
  prepared value.
- Step 2: the check fails, naming the block and the stray target.
