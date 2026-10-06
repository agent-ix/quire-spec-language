---
id: TC-909
title: "Integer bounds and literals up to i128 check, and one beyond refuses at check"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-909: Integer bounds and literals up to i128 check, and one beyond refuses at check

## Description

Verify that check admits `Int[lo, hi]` bounds and integer literals across
the whole i128 range, folds a directly negated 2^127 into the one literal
`i128::MIN`, and refuses every other integer outside the range with
`IntegerOutsideI128`.

Scope: FR-091-AC-36, FR-091-AC-37, FR-091-AC-38.

## Test Procedure

Through the spine (`parse`, `select`, `check`, `package`) on one-function
units:

1. Check `function u using v(x: Int[0, 18446744073709551615]): Boolean pure
   { x >= 0 }`, then the same with `x: Int[0, 9223372036854775808]` and with
   `x: Int[0, 18446744073709551616]`.
2. Check a parameter typed `Int[-170141183460469231731687303715884105728,
   170141183460469231731687303715884105727]` with the body
   `x <= 170141183460469231731687303715884105727`.
3. Check a parameter typed `Int[0, 170141183460469231731687303715884105728]`
   and one typed `Int[-170141183460469231731687303715884105729, 0]`.
4. Over `x: Int[-170141183460469231731687303715884105728, 0]`, check, key and
   lower the body `x >= -170141183460469231731687303715884105728`.
5. Over the same parameter, check the body
   `x >= -170141183460469231731687303715884105727`.
6. Check the bodies `x < 170141183460469231731687303715884105728`,
   `x >= -(170141183460469231731687303715884105728)` and
   `x >= -170141183460469231731687303715884105729`.

## Expected Results

1. All three admit; each parameter's resolved type is `ValueType::Int` with
   exactly the written bounds.
2. It admits; the bounds are `i128::MIN` and `i128::MAX` exactly.
3. Each refuses `IntegerOutsideI128`, code `ill_typed`/`type-mismatch`, at
   the type form's span: site `upper`, value
   `170141183460469231731687303715884105728`, limit `i128::MAX`; and site
   `lower`, value `-170141183460469231731687303715884105729`, limit
   `i128::MIN`. No checked package is returned.
4. It checks. The right operand is one `Integer` literal `i128::MIN` with no
   `Negate` node; its literal node keys to FR-092 vector L7
   (`c3aa30b239d70dea497badfb4fd426fb7b57e441aeb22d2eb450a19ad6617c5b`), and
   the lowered package holds it as one literal.
5. It checks, with the right operand a `Negate` of the literal
   170141183460469231731687303715884105727, its form unchanged.
6. Each refuses `IntegerOutsideI128` at the literal's span with site
   `literal`, the literal's value as written (2^127, 2^127 and 2^127+1),
   limit `i128::MAX` and code `ill_typed`/`type-mismatch`; no checked package
   is returned.
