---
id: TC-377
title: "Function declaration checking accepts a well-typed declaration, reports its calls, and refuses an ill-typed body"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: verifies
---
# TC-377: Function declaration checking accepts a well-typed declaration, reports its calls, and refuses an ill-typed body

## Description

Verify the behavior of `Value`'s function-declaration checking entry point
(`check::family::check_declaration_body`, which `ValueFunctionFamily::check`
calls): given a real declaration, it accepts a well-typed body, correctly
reports every call the body makes, and refuses a body whose type disagrees with the
declared result. This test verifies accept/refuse behavior, not the presence
or absence of any pre-migration symbol; see this test case's own Status
section for why.

## Test Procedure

1. Declare `f(x: Boolean) -> Boolean = x` and `g() -> Boolean = f(true)`.
   Check `g`'s body against both signatures.
2. Declare `g() -> Boolean = 1` (an `Integer` body against a declared
   `Boolean` result). Check `g`'s body.
3. Declare `v(o: Option[Integer]) -> Integer = value(o)` (a body that is
   well-typed but statically undefined: `value(o)` with no proved
   `present(o)` guarding it). Check `v`'s body.

## Expected Results

- Step 1: checking succeeds; the returned declaration reports exactly one
  call, whose callee is `f`'s signature index, and zero local evaluation
  slots (the declaration binds no `let`).
- Step 2: checking refuses with a type-mismatch cause.
- Step 3: checking refuses with a definedness cause naming the unproved
  `present` obligation, not a type-mismatch cause -- the body type-checks
  against the declared result and is refused only because that obligation
  is unproved.

## Status

Backed, all three steps:
`check_declaration_body_accepts_a_well_typed_declaration_and_reports_its_calls`,
`check_declaration_body_refuses_an_ill_typed_body` and
`check_declaration_body_refuses_an_undefined_body`
(`qsl-semantics/src/check/family.rs`, `checking_tests`), all tagged
`#[trace("TC-377")]`. This test case verifies FR-065's declaration-checking
behaviour generally; FR-065-AC-7 (TC-380) is the criterion for the contract
hook's verdict and FR-065-AC-5 (TC-164) is the criterion for call verdicts
across entry points.
