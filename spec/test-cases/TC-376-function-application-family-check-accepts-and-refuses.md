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

Verify the behavior of `Value`'s function-application family check
(`check::family::check_application`, the one call `infer_form`'s `Call` arm
makes) directly: given a real signature and a real argument list, it accepts
a well-typed call and produces a typed call node, and it refuses an
ill-formed call (wrong arity, an unknown callee name, a mismatched argument
type) with the crate's real, catalogued refusal cause. This test verifies
accept/refuse behavior, not the arm's code shape or the presence/absence of
any symbol; see this test case's own Status section for why.

## Test Procedure

1. Build a one-parameter `Boolean -> Boolean` signature named `f` and check
   `f(true)`.
2. Using the same signature, check `f(true, true)` (one argument too many).
3. Using the same signature, check `nowhere(true)` (an undeclared name).
4. Using the same signature, check `f(1)` (an `Integer` argument against a
   declared `Boolean` parameter).

## Expected Results

- Step 1: the call is accepted; the produced node's value type is `Boolean`
  and its call identity resolves to signature index 0.
- Step 2: the call is refused with a type-mismatch cause (arity does not
  match).
- Step 3: the call is refused with a missing-name cause naming `nowhere`.
- Step 4: the call is refused with a type-mismatch cause.

## Status

**Backed.** `check_application_accepts_a_well_typed_call`,
`check_application_refuses_wrong_arity` and
`check_application_refuses_an_unknown_name`
(`src/check/family.rs`, `checking_tests`), tagged
`#[trace("TC-376", "FR-065-AC-4")]`.

**Behavioral, not structural, per team-lead testing-policy ruling (QSL-148,
2026-09-21).** FR-065-AC-4 is worded as a code-shape test ("an AST or
line-count check against a fixed budget"), and TC-163's own step 6 verifies
that shape directly. That structural fact is true of the delivered code --
`infer_form`'s `Call` arm is exactly one call into
`super::family::check_application` and holds no other conditional, lookup
or loop -- but this test case does not re-verify it, on explicit
instruction: test what the family check accepts and refuses, not the arm's
structure or placement. TC-376 instead exercises `check_application` itself
(the function the arm's one call reaches) directly, showing that call
performs real, adjudicated checking -- not that the arm merely delegates
somewhere.
