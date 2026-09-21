---
id: TC-174
title: "Checking and evaluation produce identical results before and after the split"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: verifies
---
# TC-174: Checking and evaluation produce identical results before and after the split

## Description

Verify that moving checking to `check` and rewiring evaluation over `check`'s
accessors changes no observable behavior: the same domain package, the same
arguments, object environment and meter produce the same `CheckedPackage` (or
the same refusal set) from checking, and the same `Evaluation` (or the same
`InputRefusal`) from evaluation, before and after the move. This test catches
a split whose module-shape criteria (TC-170 through TC-173) all pass but
whose accessor plumbing is wrong — for example, `check`'s accessor for a
function's `slots` accidentally returning a different function's `slots`,
or `CheckedPackage::call`'s admission step validating against the wrong
parameter list after the type's fields became accessor calls instead of
direct field reads. A shape-only review of the split would not catch this;
running the same fixture through both the pre-move and post-move code is
what does. Scope: FR-068-AC-5.

## Test Procedure

1. Select the existing checking and evaluation fixtures already used by
   `value::expression`'s own tests (the function-application and dispatch
   fixtures FR-065/FR-062's tests exercise), covering: a package that
   checks and evaluates successfully; a package `PackageDeclarations::check`
   refuses (at least one case per `CheckCause` variant already covered by an
   existing test); a `CheckedPackage::call`/`evaluate` invocation refused by
   `InputRefusal` (unknown function, arity mismatch, wrong value kind,
   dangling reference).
2. Run each fixture against the pre-move code (the commit immediately
   before this requirement's implementation) and record its full result:
   the `CheckedPackage` construction outcome (success, or the exact
   `CheckRefusal` set) and, for a successfully checked package, the
   `CheckedPackage::call`/`evaluate` result (the exact `Evaluation` or the
   exact `InputRefusal`).
3. Run the identical fixtures against the post-move code and record the
   same fields.
4. Compare the pre-move and post-move results field for field for every
   fixture.

## Expected Results

- Step 4: every fixture's pre-move and post-move results are identical,
  field for field — the same checked/refused outcome from checking, and the
  same evaluated/refused outcome from evaluation; any fixture whose
  post-move result differs from its pre-move result, in any field, fails
  this test and names the fixture and the differing field.
