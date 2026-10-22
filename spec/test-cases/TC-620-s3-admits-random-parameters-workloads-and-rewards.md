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

Scope: FR-185-AC-1 to FR-185-AC-9.

## Test Procedure

Fixtures: the dimension `Time`, the unit `s` (root) and the unit `ms` (target `s`, scale `1/1000`), ADR-024 §7.2's `Service` model (with `d` typed `Millis = Int[1, 12]`), the variants FR-185-AC-2 to FR-185-AC-4 list, a second model `Other`, and the coprime-weight distributions of FR-185-AC-6.

1. Check `Service` and read back `attempt`'s random parameters, the `duration` reward and the workload `Steady`.
2. Check each random-parameter variant of FR-185-AC-2, including the weights `rational(-1, 1)`, the bare weight `-1`, and the exact-value bounds of FR-185-AC-2: `decimal(1, 4294967295)`, `rational(n, n)` with `n` of 70,000 bits, `rational(m, 1)` with `m` of 65,536 and of 65,537 bits, `decimal(1, 19728)`, `decimal(1, 19729)`, `decimal(25, 3)` and `decimal(5, 0)`.
3. Check each workload variant of FR-185-AC-3.
4. Check each reward variant of FR-185-AC-4, and `reward cost on Server::request = 2`.
5. Declare a second workload `Steady` on `Service`, and one on `Other` (FR-185-AC-5).
6. Check the two coprime-weight distributions of FR-185-AC-6, and one with a check cancelled before it starts.
7. Emit `Service` (FR-185-AC-7).
8. Check `Server::attempt` with `d: Quantity<ms>` and its supports, then with `1 s` and `true` (FR-185-AC-8, with `Time`, `s` and `ms` declared; it reads QSpec's unit `typeRef` and quantity literal form, FR-322-AC-79 and FR-322-AC-80).
9. Check the reference-typed parameter variant of FR-185-AC-9 with an object of `Server` as the support value, then the variant with the parameter typed `Int[1, 12]` and the integer support value `3`.

Tag the tests `#[trace("TC-620", "FR-185-AC-n")]`.

## Expected Results

- Step 1: supports and probabilities `9/10`, `7/100`, `3/100` and `49/50`, `1/50`; `duration: Millis`; weight 1 per operation.
- Step 2: each variant refused with a diagnostic naming the parameter and carrying its FR-185-AC-2 cause; `rational(-1, 1)` refused `invalid-weight`, `decimal(1, 4294967295)` and `rational(n, n)` (`n` of 70,000 bits) each refused with a `rational-bits-exceeded` limit refusal (`LimitExceeded`) without expanding or reducing, `rational(m, 1)` at 65,536 bits and `decimal(1, 19728)` admitted, `rational(m, 1)` at 65,537 bits and `decimal(1, 19729)` refused with the limit refusal, `decimal(25, 3)` admitted as `1/40`, `decimal(5, 0)` admitted, the bare `-1` refused `invalid_syntax` at S1.
- Step 3: each variant refused naming the workload and the operation, with its FR-185-AC-3 cause.
- Step 4: the three variants refused naming the reward, with their FR-185-AC-4 causes (including `duplicate-reward`); `cost` checks.
- Step 5: each second `Steady` refused `conflicting_declaration`/`duplicate-declaration` with both declarations related.
- Step 6: the 65,536-bit sum admitted; the 65,537-bit sum refused with the limit refusal naming the parameter; the check with a tripped handle ends cancelled with the handle requested and a reported limit that is not the 65,536-bit ceiling.
- Step 7: S3 checks, S4 refuses `unsupported_construct`/`not-yet-implemented` naming the first random parameter, `random-parameter` and the profile.
- Step 8: with `Time`, `s` and `ms` declared, the quantity supports check; `1 s` and `true` refused `value-outside-type`.
- Step 9: the reference support value refused `value-outside-type` naming the parameter; the `Int[1, 12]` variant with the integer support value checks.
