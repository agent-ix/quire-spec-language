---
id: TC-1385
title: "Fast-path operations charge the meter as the kernel call they replace"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-562
    type: verifies
---
# TC-1385: Fast-path operations charge the meter as the kernel call they replace

## Description

Verify budget parity for fast-path and kernel-call operations.

Scope: FR-562-AC-4.

## Test Procedure

1. For each function of FR-562-AC-1 to AC-3, call it at its range ends on the
   JIT and on the interpreter with a charge-recording meter, and compare the
   charge sequences (kind, amount, charge point).
2. Repeat each call with `work_units` one below the interpreter's measured
   spend.

Tag the tests `#[trace("TC-1385", "FR-562-AC-4")]`.

## Expected Results

- Step 1: the sequences are equal.
- Step 2: each run returns `Incomplete` naming the same limit at the same
  charge point as the interpreter.
