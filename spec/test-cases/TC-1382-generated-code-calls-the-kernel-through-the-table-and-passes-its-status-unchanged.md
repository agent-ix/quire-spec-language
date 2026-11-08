---
id: TC-1382
title: "Generated code calls the kernel through the table and passes its status unchanged"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-561
    type: verifies
---
# TC-1382: Generated code calls the kernel through the table and passes its status unchanged

## Description

Verify that a kernel operation runs through one `KernelTable` entry with the
interpreter's charges, and that each non-value status reaches the call's
result unchanged.

Scope: FR-561-AC-1 and FR-561-AC-2.

## Test Procedure

1. Prepare `add` with a recording `KernelTable`; call it with 2^100 and 1.
   Record the kernel calls and the meter charges; run the interpreter over
   the same call and record its charges.
2. Prepare `q(a: Integer, b: Integer): Integer { a div b }`. Call it with
   `b` = 0; with `work_units` one below its measured spend; with a recording
   table whose `div` entry returns `Fault`; and with a handle cancelled
   `Requested` before the call. Run the interpreter over the first two.

Tag the tests `#[trace("TC-1382", "<AC id>")]`.

## Expected Results

- Step 1: one `add` call with operands 2^100 and 1; result 2^100 + 1; the
  charge sequences are equal.
- Step 2: `Undefined`, `division-by-zero`, at the `div`'s `Location`, equal to
  the interpreter's; the interpreter's `Incomplete`; `Fault` and no value;
  `CallFailure::Cancelled(Requested)`.
