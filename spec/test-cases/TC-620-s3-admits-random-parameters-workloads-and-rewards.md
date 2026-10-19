---
id: TC-620
title: "S3 admits random parameters, workloads and rewards and refuses malformed ones"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: verifies
---
# TC-620: S3 admits random parameters, workloads and rewards and refuses malformed ones

## Description

Verify S3's checks of random parameters, workloads and rewards on ADR-024 §7.2's `Service` model and its malformed variants.

Scope: FR-185-AC-1 to FR-185-AC-4.

## Test Procedure

Fixtures: ADR-024 §7.2's `Service` model and the variants FR-185-AC-2 to FR-185-AC-4 list.

1. Check `Service` and read back `attempt`'s random parameters, the `duration` reward and the workload `Steady`.
2. Check each random-parameter variant of FR-185-AC-2, in a unit of at least 19,729 bytes, including the weights `rational(-1, 1)`, the bare weight `-1`, and the exact-value bounds of FR-185-AC-2: `decimal(1, 4294967295)`, `rational(n, n)` with `n` of 70,000 bits, `rational(m, 1)` with `m` of 65,536 and of 65,537 bits, `decimal(1, 19728)`, `decimal(1, 19729)`, `decimal(25, 3)` and `decimal(5, 0)`.
3. Check each workload variant of FR-185-AC-3.
4. Check each reward variant of FR-185-AC-4, and `reward cost on Server::request = 2`.

Tag the tests `#[trace("TC-620", "FR-185-AC-n")]`.

## Expected Results

- Step 1: supports and probabilities `9/10`, `7/100`, `3/100` and `49/50`, `1/50`; `duration: Quantity<Time>`; weight 1 per operation.
- Step 2: each variant refused with a diagnostic naming the parameter and carrying its FR-185-AC-2 cause; `rational(-1, 1)` refused `invalid-weight`, `decimal(1, 4294967295)` refused at S2 (`DecimalScaleBeyondSource`) and `rational(n, n)` (`n` of 70,000 bits) refused with the rational-bits limit outcome without expanding or reducing, `rational(m, 1)` at 65,536 bits and `decimal(1, 19728)` admitted, `rational(m, 1)` at 65,537 bits and `decimal(1, 19729)` refused with the limit outcome naming `s3.rational_bits`, `decimal(25, 3)` admitted as `1/40`, `decimal(5, 0)` admitted, the bare `-1` refused `invalid_syntax` at S1.
- Step 3: each variant refused naming the workload and the operation, with its FR-185-AC-3 cause.
- Step 4: the three variants refused naming the reward, with their FR-185-AC-4 causes (including `duplicate-reward`); `cost` checks.
