---
id: TC-760
title: "execute returns what CheckedPackage::call and evaluate return"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-279
    type: verifies
---
# TC-760: execute returns what CheckedPackage::call and evaluate return

## Description

Verify `execute` with the interpreter backend against S6a and FR-100's command.

Scope: FR-279-AC-1 to FR-279-AC-4.

## Test Procedure

1. Over FR-100-AC-4's unit, call `execute` and `CheckedPackage::call` for `lt(5, 3)`, `flag(1)`, `id(4)` and `id(12)`.
2. Over FR-109's ConfigVersion unit, call `execute` with a clause selection and `qsl_replay::spine::run_clause` with the same clause and input.
3. For FR-100-AC-1, AC-4 and AC-6's requests, compare the command's `outcome` member with the rendering of `execute`'s result.
4. Call `execute` choosing a supplied test `aot` backend whose `prepare` returns `PrepareFailure::Limit` naming `prepare.code_bytes`, then one whose `prepare` returns `PrepareFailure::Fault`.

Tag the tests `#[trace("TC-760", "<AC id>")]`.

## Expected Results

- Step 1: the results are equal pairwise, including `id(12)`'s `CallFailure::Input`.
- Step 2: the `Evaluation`s are equal.
- Step 3: each pair is equal.
- Step 4: `CallFailure::Limit` with the same `LimitExceeded`, then `CallFailure::Fault` with the same `InternalFault`.
