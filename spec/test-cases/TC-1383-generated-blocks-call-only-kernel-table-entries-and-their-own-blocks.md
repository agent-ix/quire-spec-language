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

Scope: FR-561-AC-3 and FR-561-AC-4.

## Test Procedure

1. Prepare the JIT over each of the 10,000 FR-295 generated units at the
   default seed. For each generated block, read its relocation records,
   classify each direct call target and count its indirect calls. Count the blocks that call a `KernelTable` entry.
2. Prepare one unit with a test code generator that emits one direct call to
   an address outside the kernel table and the prepared blocks, and one with
   a generator that emits an indirect call through a register; run the same
   check on each.
3. Inside `qsl_jit::exec`, scan the field types of `EntryFrame`; cancel
   `&Cancel` after a call starts and before its first charge.

Tag the tests `#[trace("TC-1383", "FR-561-AC-3")]` and
`#[trace("TC-1383", "FR-561-AC-4")]`.

## Expected Results

- Step 1: every direct target is a `KernelTable` shim or a block of the same
  prepared value, no block holds an indirect call, and at least one block
  calls a shim.
- Step 2: each check fails, naming the block and the stray target.
- Step 3: `EntryFrame` has no function-pointer, `KernelTable` or `Cancel`
  field, and the cancelled call returns `CallFailure::Cancelled(Requested)`.
