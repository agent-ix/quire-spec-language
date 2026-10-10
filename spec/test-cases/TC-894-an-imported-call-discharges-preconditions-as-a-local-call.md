---
id: TC-894
title: "An imported call discharges its preconditions as a local call does"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: verifies
---
# TC-894: An imported call discharges its preconditions as a local call does

## Description

Verify that E3 checks an imported function call's arguments against the
callee's parameter types with the conversions and obligations of a local
call, discharged from the importing unit's facts, and that the callee's own
body obligations are discharged by the library's compile, never at the call
site. Scope: FR-099-AC-8.

## Test Procedure

Library `test/geometry` version `1` exports
`function f using v(x: Int[0, 9]): Boolean pure { x < 5 }`. The importing
unit imports it `as g` under its recomputed `package_id` and declares the
local `function f2 using v(x: Int[0, 9]): Boolean pure { x < 5 }`.

1. For each argument `e` in the list below, compile one importing unit whose
   function body is `g::f(e)` and one whose body is `f2(e)`:
   - `y`, with `y: Int[0, 9]`;
   - `3`;
   - `12`;
   - `y`, with `y: Int[0, 20]`;
   - `if y <= 9 then g::f(y) else false` (and `f2(y)` in its pair), with
     `y: Int[0, 20]`.
2. Change `f`'s body to `(10 div x) < 5`, update the import's digest to the
   new `package_id`, and compile the importing unit whose body is
   `g::f(y)` with `y: Int[0, 9]`.

## Expected Results

- Step 1: for each `e`, the `g::f` compile and the `f2` compile give the same
  verdict. Where one refuses, both refuse with the same code, cause and
  locus, the argument `e`.
- Step 2: the compile refuses `CompileRefusal::Dependency` with path
  `[test/geometry]`, carrying `undefined_expression`/`unproved-nonzero` at
  the divisor `x` in the library's source. No refusal is raised at the
  call `g::f(y)`, and no package is emitted.

