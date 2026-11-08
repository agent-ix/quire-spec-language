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
   classify each direct call target; classify the source of each indirect
   call's target. Count the blocks that call a `KernelTable` entry.
2. Prepare one unit with a test code generator that emits one direct call to
   an address outside the kernel table and the prepared blocks, and one with
   a generator that emits an indirect call through a register not loaded from
   a table slot; run the same check on each.

Tag the tests `#[trace("TC-1383", "FR-561-AC-3")]`.

## Expected Results

- Step 1: every direct target is a `KernelTable` entry or a block of the same
  prepared value, every indirect target comes from a table slot, and at least
  one block calls a table entry.
- Step 2: each check fails, naming the block and the stray target.
