---
id: TC-218
title: "Redefinition variance checking reports every failing axis, not only the first"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: verifies
---
# TC-218: Redefinition variance checking reports every failing axis, not only the first

## Description

Verify that checking a redefining operation reports every independently
failing variance axis (parameter type, result multiplicity, effect frame)
in one `Refused` outcome, rather than stopping at the first violation.
Scope: FR-082-AC-1.

Catches an implementation that short-circuits on the first axis failure
(the common, natural Rust shape for sequential `?`-chained checks) instead
of accumulating every axis's result before returning. A test that only
supplies one failing axis at a time cannot distinguish "stops at first
failure" from "checks every axis" — both report the single failure
correctly. Only a fixture violating three independent axes simultaneously
distinguishes them.

## Test Procedure

1. Declare an operation on a supertype with one typed parameter, a typed
   result and a declared effect frame that modifies field `x`.
2. Declare a redefinition on a subtype that: (a) narrows the parameter to an
   incompatible, non-contravariant type; (b) widens the result multiplicity
   beyond what the redefined result multiplicity admits; and (c) declares an
   effect frame that modifies field `y`, not reachable from `x` through any
   `redefines` edge — three independent axis violations in one redefinition.
3. Run redefinition checking over this pair.
4. Inspect the returned `Refused` outcome's list of failing axes.

## Expected Results

The outcome names all three axes — parameter type, result multiplicity and
effect — each with its own typed cause (`variance-parameter`,
`multiplicity-narrowing`, `effect-escape`). A mutant that returns after the
first failing axis reports only one of the three, failing this assertion
even though its single reported failure is itself correct.
