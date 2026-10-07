---
id: TC-731
title: "Intake reports composite cycles of any length"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-260
    type: verifies
---
# TC-731: Intake reports composite cycles of any length

## Description

Verify that a composite cycle is reported however long it is.

Scope: FR-260-AC-3.

## Test Procedure

Run every step on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. Admit a package whose composite types `C0` to `C299` each hold the next,
   and `C299` holds `C0`.
2. With `intake.input_bytes` raised to fit, admit the same shape with
   100,000 types.

Tag the tests `#[trace("TC-731", "FR-260-AC-3")]`.

## Expected Results

- Steps 1 and 2: each is refused with semantic-IR's composite-cycle refusal
  naming the relationship that closes the cycle.

## Status

Implemented.
