---
id: TC-239
title: "An arity mismatch refuses without checking per-parameter axes, while result and effect axes are still checked"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: verifies
---
# TC-239: An arity mismatch refuses without checking per-parameter axes, while result and effect axes are still checked

## Description

Verify that a redefining operation whose parameter count differs from the
redefined operation's refuses with exactly one arity failure and checks no
per-parameter type or multiplicity axis. Scope: FR-082-AC-5's arity-carve-out
clause.

Backed by the existing test
`tests/model_conformance.rs::r04_an_arity_mismatch_refuses_without_checking_parameter_axes`
(`Refused` outcome, exactly one failure, `TypeMismatch` cause, `IllTyped`
code), which is this test case's real fixture and assertion. FR-082-AC-5's
other clause — that the result-type, result-multiplicity and effect axes
remain independently checked and reported even when arity also fails — is
not exercised by a single combined fixture in either this test case or the
codebase today; it follows from `check_operation_redefinition`'s own
structure (the result/effect checks are unconditional, outside the arity
`if`/`else`), but no test constructs a redefinition that fails arity and a
later axis simultaneously and asserts both failures appear.

Catches an implementation that does not carve out the parameter axes at
all — reporting spurious per-parameter-type or per-parameter-multiplicity
failures for parameters that have no counterpart across an unequal-length
list — a defect a test that only ever supplies equal-length parameter lists
(TC-218's fixture) cannot distinguish from correct arity handling.

## Test Procedure

1. Declare an operation on a supertype with one typed parameter, a typed
   result and a declared effect frame that modifies field `x`.
2. Declare a redefinition on a subtype with zero parameters (an arity
   mismatch), whose result type and effect frame both still conform.
3. Run redefinition checking over this pair.
4. Inspect the returned `Refused` outcome's list of failing axes.

## Expected Results

The outcome names exactly one failure: the arity axis, with a type-mismatch
cause and an `IllTyped` code. No per-parameter-type or per-parameter-
multiplicity axis appears. A mutant that also emits a per-parameter axis
failure despite the arity mismatch (for example, by zipping the shorter
parameter list against the longer one instead of skipping per-parameter
checks entirely) fails the single-failure assertion.
