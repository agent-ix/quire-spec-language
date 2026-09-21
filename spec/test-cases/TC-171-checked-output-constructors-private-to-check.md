---
id: TC-171
title: "CheckedPackage/CheckedExpression/CheckedFunction constructors are private to check"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: verifies
---
# TC-171: CheckedPackage/CheckedExpression/CheckedFunction constructors are private to check

## Description

Verify that `CheckedPackage`, `CheckedExpression` and `CheckedFunction` are
defined exactly once, in `check`, with constructors private to `check`, so
that `value::expression` (and every other module) can reach a value of these
types only through `check`'s own checking entry points and public accessors
— the same boundary ADR-011 §4 already verifies for the S4 type in `package`
by a `compile_fail` test. This test catches a split where the module
boundary is cosmetic: files moved into `src/check/`, but the type's fields
or a constructor left reachable (`pub(crate)`, or a field visible to an
ancestor module) so `value::expression` still builds the type directly
instead of going through `check`'s API. Scope: FR-068-AC-2.

## Test Procedure

1. In a `compile_fail` unit test placed inside the crate, in
   `value::expression` itself — not in `tests/`, an external integration-test
   crate — attempt to construct a `CheckedPackage` value by naming its fields
   directly (a struct literal), not through any of `check`'s checking entry
   points. A `tests/` integration test is not an acceptable substitute here:
   `CheckedPackage`'s, `CheckedExpression`'s and `CheckedFunction`'s fields
   are already private to their defining module today (module-private, not
   even `pub(crate)` — see `value/expression/mod.rs:58-79` on the pre-move
   baseline), so a struct-literal construction attempt from `tests/` already
   fails to compile before this requirement's implementation, for a reason
   unrelated to the move (private fields are invisible to an external
   integration-test crate regardless of which module defines them). A test
   placed in `tests/` therefore passes identically on the pre-move and
   post-move tree and does not discriminate the before/after flip this test
   exists to demonstrate; only a test placed inside the crate, in
   `value::expression` — a descendant of the type's defining module before
   the move, a sibling after it — flips from compiling to failing to
   compile across the move, which is what this test must show.
2. Repeat step 1 for `CheckedExpression` and for `CheckedFunction`.
3. Compile the test harness and record the compiler's diagnostic for each of
   the three attempts.
4. Separately, confirm that `value::expression`'s `CheckedPackage::call`,
   `CheckedPackage::evaluate` and the argument-admission logic (today's
   `validate`) compile and run correctly using only accessors `check`
   exposes — not by reaching a private field through a module-path trick
   (for example, `check` re-exporting a field as `pub` rather than exposing
   a real accessor method).

## Expected Results

- Steps 1-3: each of the three struct-literal construction attempts fails to
  compile with a privacy error (E0451 or equivalent) naming the type's
  private field or constructor; a build that succeeds fails this test, since
  it demonstrates the type is constructible from outside `check`.
- Step 4: `value::expression`'s checking-adjacent methods compile and run
  using `check`'s accessor methods only; no field of `CheckedPackage`,
  `CheckedExpression` or `CheckedFunction` is `pub` at the type definition
  itself.
