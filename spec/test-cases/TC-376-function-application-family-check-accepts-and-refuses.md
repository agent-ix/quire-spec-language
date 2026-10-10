---
id: TC-376
title: "Function application checking accepts a well-typed call and refuses wrong arity, an unknown name and a type mismatch"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: verifies
---
# TC-376: Function application checking accepts a well-typed call and refuses wrong arity, an unknown name and a type mismatch

## Description

Verify the behavior of `Value`'s function-application check
(`check::family::Application`, which `Typer::infer_form`'s `Call` arm calls):
checking an `Expression::Call` through the S3 typer against a real signature
admits a well-typed call as a typed call node, and refuses an ill-formed
call (wrong arity, an unknown callee name, a mismatched argument type) with
the crate's catalogued refusal cause. Scope: FR-065-AC-4.

## Test Procedure

1. Build a one-parameter `Boolean -> Boolean` signature named `f` and check
   `f(true)`.
2. Using the same signature, check `f(true, false)` (one argument too many).
3. With no declared function or type, check `nowhere()` (an undeclared
   name).
4. Using the same signature as step 1, check `f(1)` (an `Integer` argument
   against a declared `Boolean` parameter).

## Expected Results

- Step 1: the call is accepted; the produced node's value type is `Boolean`
  and its callee is signature index 0.
- Step 2: the call is refused with a type-mismatch cause (arity does not
  match).
- Step 3: the call is refused with a missing-name cause naming `nowhere`.
- Step 4: the call is refused with a type-mismatch cause.

