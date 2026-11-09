---
id: TC-1384
title: "The fast path is taken exactly where checked bounds prove the result fits"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-562
    type: verifies
---
# TC-1384: The fast path is taken exactly where checked bounds prove the result fits

## Description

Verify the fast-path decision at the boundary of the signed 64-bit range,
for division with and without zero in the divisor's bound, for operand types
that never take it, and with a result-bound obligation.

Scope: FR-562-AC-1, FR-562-AC-2, FR-562-AC-3, FR-562-AC-5, FR-562-AC-6, FR-562-AC-7 and FR-562-AC-8.

## Test Procedure

With a recording `KernelTable`, prepare and call each function FR-562 names,
at each parameter's range ends where it has a bounded type:

1. `f` and `g` (AC-1).
2. `m`, the same function with bounds 3037000500, and `n` (AC-2).
3. `d`, and `d0` including `b` = 0 (AC-3).
4. `r` with 1 and 2, `t`, and `u` with 1 and 2 (AC-5).
5. `s` with 5 and 5, checked under `CheckMode::Linked` (refused
   `unproved-range`) and under `CheckMode::Kernel` through
   `PackageDeclarations::check` (AC-6).
6. `s7` at each pair of range ends (AC-7).
7. `rm` at `a` = 9 and `b` = 4000000000000000000, and `rn`, `rf` and `rj` at
   every integer pair in their ranges, each under its division law (AC-8);
   run `rm` through a test translator that takes the `mod` interval from the
   operand interval ends.

Run the interpreter over every call. Tag the tests
`#[trace("TC-1384", "<AC id>")]`.

## Expected Results

- `f`, `m`, `d` and `s`'s addition record no kernel call; `g`, the
  3037000500 variant of `m`, `n`, `d0`, `r`, `t` and `u` record one kernel
  call per call.
- `s7` and `rn`, `rf` and `rj` record no kernel call; `rm` records one
  `mul` call and no `mod` call, and the ends-based translator fails it.
- `s` is refused `unproved-range` under `Linked`.
- Every result, cause and `Location` equals the interpreter's, including
  `d0`'s `Undefined`, `division-by-zero`, and `s`'s result-bound outcome.
