---
id: TC-380
title: "The function-declaration contract check refuses an ill-typed declaration and admits a well-typed one"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: verifies
---
# TC-380: The function-declaration contract check refuses an ill-typed declaration and admits a well-typed one

## Description

Verify that the `Value` family's contract `check` hook, called on a function
declaration, makes the typing verdict itself: an ill-typed declaration is
refused through the contract's refusal outcome, and a well-typed one is
admitted with its minted identity. Scope: FR-065-AC-7.

## Test Procedure

1. Build a typing context whose limits the declaration does not reach, with
   an empty diagnostic sink. Check `g() -> Boolean = 1` through `ValueFunctionFamily::check`.
2. With a fresh typing context, check `f() -> Boolean = true` through
   `ValueFunctionFamily::check`, and mint `f`'s identity independently.

## Expected Results

- Step 1: the outcome is `StageFailure::Refused`; its cause is
  `CheckCause::IllTyped(TypeMismatch)` (`ill_typed` / `type-mismatch`); the
  diagnostic sink holds no entry.
- Step 2: the outcome is the checked declaration; its identity equals the
  independently minted identity for `f`.

## Status

Covered in behavior, not yet traced.
`value_function_family_check_refuses_an_ill_typed_body` (step 1) and
`value_function_family_checks_through_the_contract` (step 2), both in
`src/value/expression/family.rs`, perform exactly these steps and pass.
Neither carries a `#[trace("TC-380", "FR-065-AC-7")]` tag yet.
