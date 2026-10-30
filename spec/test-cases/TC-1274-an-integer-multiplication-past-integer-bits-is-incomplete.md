---
id: TC-1274
title: "An integer multiplication past integer_bits is incomplete at its arithmetic charge"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
---
# TC-1274: An integer multiplication past integer_bits is incomplete at its arithmetic charge

## Description

Verify that S6a returns the kernel's `Incomplete` record, with the kernel's
amounts, when `2^64 x 2^64` is evaluated under `integer_bits` 128. This
catches an evaluator that refuses, wraps, recomputes the amounts, or
completes the multiplication.

Scope: FR-096-AC-23.

## Test Procedure

1. Check `a * b` with `a` and `b` of `Int[0, 18446744073709551616]`, and
   evaluate it at S6a for `a = b = 18446744073709551616` under
   `integer_bits` 128 and every other limit large enough not to stop first,
   recording the meter's admitted charges.

Tag the test `#[trace("TC-1274", "FR-096-AC-23")]`.

## Expected Results

- Step 1: `Outcome::Incomplete` with limit kind `integer_bits`, limit 128,
  consumed 65, denied amount 130 and charge point
  `integer-arithmetic.arithmetic`; not `Outcome::Refused`; no
  `integer-arithmetic.result-retain` charge admitted.
