---
id: TC-394
title: "Each Value expression construct maps to its Expression variant, with grouping from the CST"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-394: Each Value expression construct maps to its Expression variant, with grouping from the CST

## Description

Verify FR-091's expression-mapping table row by row, and that grouping and
associativity come from the CST's nesting.

This catches a missing or swapped row, and a production that rebuilds
precedence from token order.

Scope: FR-091-AC-3, FR-091-AC-39.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. For each row of FR-091's expression-mapping table, build an admissible
   unit whose one function body holds that construct over names `a`, `b`,
   `c`, as a table-driven test with one case per row.
2. Run S2 and read the function's body.
3. Separately run S2 on the bodies `a or b and c`, `a - b - c`,
   `a implies b implies c` and `(a + b) * c`.
4. Run S2 on the bodies `map(x in c: x)`, `collect(x in c: x)` and
   `allInstances<M::T>(p)`.
5. Run S2 on `a div b`, `a rem b`, `a mod b`, `1 + a div b * 2` and
   `if true then 1 else a mod b`, retaining each operator and operand span.
   Check each well-typed body under a resolved `quire.value.complete/v1`
   selection with exactly one of the truncating, floor or Euclidean
   integer-division definitions, and separately check `mod` without selecting
   a division law when no `div`/`rem` occurs. Use `(-7,3)` to distinguish
   the selected `div`/`rem` law and require `mod` to expose `2` under every
   selection. Check `a / b` with integer operands both without an expected
   rational type and with an expected `Rational[..]` type.

Tag the test `#[trace("FR-091-AC-3", "FR-091-AC-39", "TC-394")]`.

## Expected Results

- For every row, step 2 reads the listed variant, with operands in source
  order. `R { f: a, g: null }` gives field `g` as `FieldInitializer::Null`,
  and `(a)` gives `Name("a")`.
- Step 3 reads `Or(a, And(b, c))`, `Subtract(Subtract(a, b), c)`,
  `Implies(a, Implies(b, c))` and `Multiply(Add(a, b), c)`.
- Step 4 reads `Query` with `BinderQuery::Map` for both `map` and
  `collect`, and `AllInstances` whose target is a type form with
  qualified-name head `M::T`.
- Step 5 reads distinct `Binary{IntegerDiv, IntegerRem, IntegerMod}`
  forms with the CST's product precedence and original spans, including
  the dead branch. Check accepts them only under complete value; `div`/`rem`
  retain the selected law and `mod` remains Euclidean. `/` never denotes
  integer division; the expected rational type enables FR-149's exact
  integer-operand promotion for rational division.
