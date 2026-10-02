---
id: TC-885
title: "Formatting keeps the checked package identity of every complete-V1 fixture unit"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-003
    type: verifies
---
# TC-885: Formatting keeps the checked package identity of every complete-V1 fixture unit

## Description

Verify that formatting changes no meaning: the formatted source checks to the same checked package as the original.

Scope: FR-003-AC-9.

## Test Procedure

For each complete-V1 fixture unit in the repository's tests that checks: format it, compile the original and the formatted source through the S1 to S4 spine, and format the formatted source again.

Tag the tests `#[trace("TC-885", "FR-003-AC-9")]`.

## Expected Results

- Each formatted unit checks, with a checked package identity equal to the original's.
- The second formatting returns identical bytes.
