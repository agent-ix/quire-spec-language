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
the whole i128 range and refuses one outside it with `IntegerOutsideI128`.

Scope: FR-091-AC-36, FR-091-AC-37.

## Test Procedure

Through the spine (`parse`, `select`, `check`) on one-function units:

1. Check `function u using v(x: Int[0, 18446744073709551615]): Boolean pure
   { x >= 0 }`, then the same with `x: Int[0, 9223372036854775808]`.
2. Check a parameter typed `Int[-170141183460469231731687303715884105728,
   170141183460469231731687303715884105727]` with the body
   `x <= 170141183460469231731687303715884105727`, then with
   `x >= -170141183460469231731687303715884105728`.
3. Check a parameter typed `Int[0, 170141183460469231731687303715884105728]`
   and one typed `Int[-170141183460469231731687303715884105729, 0]`.
4. Check the bodies `x < 170141183460469231731687303715884105728` and
   `x > -170141183460469231731687303715884105729` over `x: Int[0, 9]`.

## Expected Results

1. Both admit; each parameter's resolved type is `ValueType::Int` with
   exactly the written bounds.
2. Both admit; the bounds are `i128::MIN` and `i128::MAX` exactly.
3. Each refuses `IntegerOutsideI128`, code `ill_typed`/`type-mismatch`, at
   the type form's span: site `upper`, value
   `170141183460469231731687303715884105728`, limit `i128::MAX`; and site
   `lower`, value `-170141183460469231731687303715884105729`, limit
   `i128::MIN`. No checked package is returned.
4. Each refuses `IntegerOutsideI128` at the literal's span with site
   `literal`, limit `i128::MAX` and `i128::MIN` respectively, and no checked
   package is returned.
