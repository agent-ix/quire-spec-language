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

Part A is backed by the existing test
`tests/model_conformance.rs::r04_an_arity_mismatch_refuses_without_checking_parameter_axes`
(`Refused` outcome, exactly one failure, `TypeMismatch` cause, `IllTyped`
code), which is Part A's real fixture and assertion.

Part B exercises FR-082-AC-5's other clause — that the result-type,
result-multiplicity and effect axes remain independently checked and
reported even when arity also fails. No existing test constructs this
combined fixture: `r04` deliberately keeps the result type and effect
frame conforming so it isolates the arity carve-out alone. Part B follows
from `check_operation_redefinition`'s own structure (the result/effect
checks are unconditional, outside the arity `if`/`else`), but until an
implementation PR adds a test asserting it, this clause's combined-failure
coverage is a known gap. Remaining work: #120.

Catches an implementation that does not carve out the parameter axes at
all — reporting spurious per-parameter-type or per-parameter-multiplicity
failures for parameters that have no counterpart across an unequal-length
list — a defect a test that only ever supplies equal-length parameter lists
(TC-218's fixture) cannot distinguish from correct arity handling. Part B
additionally catches an implementation that carves out the parameter axes
correctly but, by doing so, accidentally short-circuits the unrelated
result/effect checks too (for example, an early `return` guarding the
whole check rather than only the per-parameter loop).

## Test Procedure

**Part A (arity alone):**

1. Declare an operation on a supertype with one typed parameter, a typed
   result and a declared effect frame that modifies field `x`.
2. Declare a redefinition on a subtype with zero parameters (an arity
   mismatch), whose result type and effect frame both still conform.
3. Run redefinition checking over this pair.
4. Inspect the returned `Refused` outcome's list of failing axes.

**Part B (arity together with a second, independent axis failure):**

5. Declare an operation on a supertype with one typed parameter and a typed
   result with multiplicity `[0,1]`.
6. Declare a redefinition on a subtype with zero parameters (an arity
   mismatch) whose result multiplicity narrows to `[1,1]` without an
   established postcondition proving the narrowing (a second, independent
   failure on the result-multiplicity axis), while the effect frame still
   conforms.
7. Run redefinition checking over this pair.
8. Inspect the returned `Refused` outcome's list of failing axes.

## Expected Results

Step 4's outcome names exactly one failure: the arity axis, with a
type-mismatch cause and an `IllTyped` code. No per-parameter-type or
per-parameter-multiplicity axis appears. A mutant that also emits a
per-parameter axis failure despite the arity mismatch (for example, by
zipping the shorter parameter list against the longer one instead of
skipping per-parameter checks entirely) fails the single-failure assertion.

Step 8's outcome names exactly two failures: the arity axis and the
result-multiplicity axis, with no per-parameter axis among them. A mutant
that short-circuits on the arity failure and skips the still-independent
result-multiplicity check reports only one failure in step 8, failing the
two-failures assertion.
