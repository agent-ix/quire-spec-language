---
id: FR-397
title: "Select the IEEE profile of a package that declares or evaluates a float"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-148
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-201
    type: depends_on
---
# FR-397: Select the IEEE profile of a package that declares or evaluates a float

## Description

The QSpec definition prose names the IEEE definition `quire.value.ieee754-2019-default/v1`
under the role `ieee_profile` (QSpec FR-148). The role is not header-selectable
(FR-110-AC-3). This requirement states how the profile reaches the law of an
IEEE node. It also states every float
source form: the bit constructors that write a constant, the arithmetic, the
comparison operators, the `quire::value::ieee::*` intrinsic calls and the
`convert` forms into and out of a float, each with its typing, its lowering to
FR-322 terms (FR-093) and its evaluation under QSpec FR-148.

## Inputs

- A unit, with its supplied libraries, whose source may use `Float32[mode]` and
  `Float64[mode]` types (FR-092) and the float forms of this requirement.
- The layer table's definition of the role `ieee_profile`.

A float value in a body comes from a declared parameter or field, from a bit
constructor, from an IEEE arithmetic or intrinsic result, or from a conversion
into a float. Model float fields are refused at intake (FR-091).

## Outputs

- Each node of an IEEE operation carries the law `ieee_profile`, whose
  `definition` is the `DefinitionRef` `{authority, identity}` of the QSpec
  definition prose for that role (FR-110's layer table; FR-093), with no
  selection rows.
- A package with no IEEE operation holds no such law.

## Behavior

- **The role.** The checker SHALL refuse a header `profile` that names the
  row's identity as `unknown_profile`/`wrong-selection-role` (FR-110-AC-3), and
  no other unit declaration supplies the row.
- **The profile in use.**
  - The checker SHALL build the unit's IEEE profile from the layer table's row of the role.
  - The rounding mode of an operation is the mode the operand types carry
    (FR-092), so `Float64[nearest-even]` operands round to nearest even and a
    bare `Float64` operand rounds strict `exact` (QSpec FR-148).
  - The checker SHALL write the law of an IEEE node as the layer table's row of the role
    and never from a constant (FR-093).
- **Float constants.** A float constant is written by a bit constructor,
  `float32(bits: 0xHHHHHHHH)` or `float64(bits: 0xHHHHHHHHHHHHHHHH)`, whose
  eight or sixteen hexadecimal digits are the exact IEEE 754-2019 bit pattern
  (QSpec `shared-grammar.md`). Every bit pattern is admitted: either sign of
  zero, the infinities, and every quiet and signaling NaN with its payload.
  Human decimal text is written by converting a `Decimal` or `Rational` value
  (below), never as a literal.
  - The checker SHALL type a `float32(bits: ..)` constructor as `Float32[m]` and
    a `float64(bits: ..)` constructor as `Float64[m]`, where `m` is the rounding
    mode of the expected type of its position when that expected type is a
    float of the constructor's width, and `exact` otherwise. The expected type
    of an operand of an arithmetic or comparison operator, or of an argument of
    an intrinsic call, is the type of its sibling operand when that is a float
    of the same width, and otherwise the expected type of the whole
    application; the expected type of the operand of unary `-` is the expected
    type of the whole application. The expected type of a call argument is its parameter's type,
    of a function body its result type, and of a record field its field type.
  - The checker SHALL refuse a bit constructor at a position whose expected type
    is a float of the other width, or is not a float, as
    `ill_typed`/`type-mismatch`. A constructor never changes width to fit its
    slot.
  - The checker SHALL lower a bit constructor to one `value`/`literal` node
    whose body is `literal{type, value_kind, value}` with `value_kind`
    `float32_bits` or `float64_bits`, `type` the float type node of the
    constructor's checked type, and `value` the eight or sixteen digits as
    lowercase hexadecimal without the `0x` prefix (FR-093).
  - The evaluator SHALL evaluate a bit constructor to the float value of exactly
    those bits, whatever they encode; it sets no flag and does not quiet a
    signaling NaN.
  - A bit constructor with a digit count other than eight (`float32`) or
    sixteen (`float64`), an uppercase digit, a missing `0x`, a missing `bits`
    label or a non-hexadecimal digit is refused by S1 as
    `invalid_syntax`/`invalid-token` at the constructor, before S2.
- **Arithmetic.** `+`, `-`, `*` and `/` over two operands of one float type
  (one width and one rounding mode) have that type, round once under that
  mode and lower to `quire.op.ieee.float32.*` or `quire.op.ieee.float64.*`
  (FR-093). Division by a float zero carries no definedness obligation: QSpec
  FR-148 returns the signed infinity or the canonical quiet NaN with the
  `divide_by_zero` or `invalid` flag.
  - The checker SHALL refuse arithmetic operands of unlike float widths, of
    unlike rounding modes, or a float mixed with an exact type (`Int[..]`,
    `Integer`, `Rational[..]`, `Decimal[..]`), as `ill_typed`/`type-mismatch`.
- **Negation.** Unary `-` over a float of type `Float{w}[m]` has that type and
  is IEEE `negate` (QSpec FR-148): it flips the sign bit and rounds nothing, so
  it is exact under every rounding mode including strict `exact`, sets no flag,
  keeps a NaN's payload and quiet bit and never refuses. It lowers to one
  `quire.op.ieee.float{w}.negate` node (FR-093).
  - The checker SHALL type the application as its operand's type and lower it
    with the law `ieee_profile` and no mode.
  - The evaluator SHALL evaluate it to the operand's bits with the sign bit
    flipped, whatever they encode, including a signaling NaN, which stays
    signaling.
  - The checker SHALL refuse unary `-` over a `Boolean`, `Text` or other operand
    that is neither a float, an exact number nor a quantity as
    `ill_typed`/`type-mismatch` at the operand, as it does for every operator.
- **Comparison.** The six comparison operators over two operands of one float
  type are the IEEE quiet comparisons, and each lowers to a Boolean
  composition of QSpec FR-148's `numericEqual` and `totalOrder` operations
  (`quire.op.ieee.numeric_equal`, `quire.op.ieee.total_order`) and
  `quire.op.boolean.*` nodes; no operator uses `bitIdentical`. Write
  `N(x)` for `numericEqual(x, x)`, which is false exactly when `x` is a NaN.
  - `a = b` SHALL be `numericEqual(a, b)`: false when either operand is a NaN,
    and true for `+0` and `-0`.
  - `a != b` SHALL be `not(numericEqual(a, b))`: true when either operand is a
    NaN.
  - `a < b` SHALL be `and(and(N(a), N(b)), and(totalOrder(a, b),
    not(numericEqual(a, b))))` and `a <= b` SHALL be `and(and(N(a), N(b)),
    or(totalOrder(a, b), numericEqual(a, b)))`. Both are false when either
    operand is a NaN, and `-0 < +0` is false while `-0 <= +0` is true.
  - `a > b` SHALL be the composition of `b < a` and `a >= b` the composition of
    `b <= a`, with the operands exchanged and their nodes shared.
  - The checker SHALL refuse a comparison whose operands are of unlike float
    widths, of unlike rounding modes, or a float and an exact type, as
    `ill_typed`/`type-mismatch` at the operator.
  - `=` and `!=` also apply to an option, record, tuple, union or collection
    whose type reaches a float at any depth (QSpec FR-149 "IEEE relations").
    They lower to `quire.op.structural.eq` and `.ne` (FR-093), whose leaves
    list every float leaf with the law `ieee_profile` and mode `null` beside
    the text leaves. A float leaf of an option, record, tuple, union payload or
    sequence compares by `numericEqual`, so `+0` equals `-0` and a value that
    holds a NaN is unequal to every value including itself. A float read out
    of a structure (`r.x`, `value(o)`) is compared as a float.
  - `=` and `!=` over a set, bag or ordered set whose element type reaches a
    float also lower to `quire.op.structural.eq` and `.ne`, with the float leaf
    at the path `["inner"]` (QSpec FR-322's leaf walk), the law `ieee_profile`
    and mode `null`. Two rules apply together. Set members match by bit
    identity: the collections are compared by QSpec FR-144's membership
    relation, which matches a float member by `bitIdentical`, so `set[qnan]`
    equals `set[qnan]`, `set[+0]` is unequal to `set[-0]`, and two NaNs of
    different payload are different members. Leaves compare by
    `numericEqual`: a float leaf of an option, record, tuple, union payload or
    sequence, and a float read out of a collection, compares by
    `numericEqual` as above. A sequence is not a set, so a sequence holding a
    NaN stays unequal to itself.
  - `numericEqual`, `totalOrder` and `bitIdentical` are also written as calls
    (Intrinsics), so a source chooses the relation by spelling the call.
- **Intrinsics.** The reserved intrinsic identities
  `quire::value::ieee::sqrt`, `fma`, `numericEqual`, `totalOrder` and
  `bitIdentical` (QSpec FR-148, `shared-grammar.md`) are the ordinary
  qualified-call production. The checker SHALL resolve a call headed by one of
  them to the intrinsic and type and lower it as follows (FR-093).
  - `sqrt(x)` over a `Float{w}[m]` has type `Float{w}[m]` and lowers to
    `quire.op.ieee.float{w}.sqrt` with mode `rounding` `m`.
  - `fma(a, b, c)` over three operands of one float type `Float{w}[m]` has
    that type and lowers to `quire.op.ieee.float{w}.fma` with mode `rounding`
    `m`; it rounds once after the unrounded product and sum.
  - `numericEqual(a, b)`, `totalOrder(a, b)` and `bitIdentical(a, b)` over two
    operands of one float type have type `Boolean` and lower to
    `quire.op.ieee.numeric_equal`, `quire.op.ieee.total_order` and
    `quire.op.ieee.bit_identical` with no mode. `totalOrder(a, b)` is true when
    `a` precedes or equals `b` in QSpec FR-148's total order, and
    `bitIdentical(a, b)` is true exactly when the bits are the same.
  - The checker SHALL lower each intrinsic node with the law `ieee_profile`
    and the operands in call order.
  - The checker SHALL refuse a call to an intrinsic with an argument count other
    than its arity, with an argument that is not a float, with arguments of
    unlike widths or rounding modes, or with a float and an exact type, as
    `ill_typed`/`type-mismatch` at the argument or, for the count, at the call.
  - The checker SHALL refuse a name `quire::value::ieee::<name>` that also
    resolves to a declaration of the unit or of a supplied library as
    `invalid_package`/`conflicting-definition`, naming the identity; a user
    declaration never replaces or shadows an intrinsic.
  - A qualified name `quire::value::ieee::<name>` for any other `<name>`
    refuses `missing_declaration`/`missing-name`.
- **Conversion into a float.** `convert<Float32[m]>(e)` and
  `convert<Float64[m]>(e)` are explicit; the target's mode `m` is the policy.
  - Over an `Integer`, `Int[..]`, `Rational[..]` or `Decimal[..]` operand, the
    checker SHALL lower the conversion to `quire.op.ieee.from_exact` with the
    law `ieee_profile`, mode `rounding` `m` and member `type_argument` of the
    target's node. The evaluator SHALL round the operand's exact value once
    under `m`; under strict `exact` an inexact, overflowing or tiny-inexact
    result is `refused { code: ieee_not_exact, cause: rounding-required }` with
    its `nearest-even` would-be flags and no bits (QSpec FR-148). A decimal
    operand is sized by its retained coefficient and scale.
  - Over a `Float64` operand and a `Float32` target, whatever the operand's
    rounding mode, the checker SHALL lower the conversion to
    `quire.op.ieee.to_float32` with mode `rounding` `m`; over a `Float32`
    operand and a `Float64` target, to `quire.op.ieee.to_float64` with no mode
    (every binary32 value is a binary64 value). Both carry the law
    `ieee_profile` and member `type_argument` of the target's node. A finite
    value rounds once under `m`, a zero and an infinity keep their sign, and a
    NaN keeps its sign and integer payload below the quiet bit, gains the quiet
    bit and sets `invalid` when signaling; a NaN payload not smaller than the
    target's quiet bit is `refused { code: ieee_nan_payload_not_representable,
    cause: payload-exceeds-target }` (QSpec FR-148).
  - A `convert` whose operand type already equals the target type is the
    operand itself and builds no node.
  - Over a float operand of the target's width whose rounding mode differs
    from the target's, the checker SHALL type the conversion with the target
    type and build no node: the rounding mode is part of the type (QSpec
    FR-148), the width is unchanged, and every value of the width is
    representable, so the value is the operand's bits exactly. The conversion
    rounds nothing, sets no flag, charges nothing and is admitted under every
    target mode including strict `exact`; a zero, an infinity and a NaN keep
    their bits, and a signaling NaN stays signaling. Operators applied to the
    result use the target's mode. (Lead default, not an owner answer; Peter may
    veto: QSpec FR-148 names the mode as part of the type and width conversion
    as a rounding of the exact value to the target width, which is the
    identity at the same width; it states no separate same-width operation.)
  - The checker SHALL refuse a `convert` into a float from a quantity, `Text`,
    `Boolean` or any other operand that is neither an exact number nor a
    float, as `ill_typed`/`type-mismatch`.
- **Conversion out of a float.** The checker SHALL lower a
  `convert<Rational[...]>(e)` whose operand is a `Float32` or `Float64` to
  `quire.op.ieee.to_rational` (FR-093). Its value, loss and refusals are
  QSpec FR-148's: a NaN or infinity is `Undefined`, and a value outside the
  named domain is `refused { code: ieee_rational_out_of_domain }`.
  - The checker SHALL refuse a `convert` of a float to `Decimal[..]`,
    `Integer` or `Int[..]` as `ill_typed`/`type-mismatch` with no charge
    (QSpec FR-148 defines no such conversion); an IEEE value reaches exact
    arithmetic only as `Rational[..]`.
- **Strict admission.** The V2 reader (`quire.checked-package/v2`) SHALL admit the
  package emitted for a unit whose float forms are any of those above.
- **Clauses.** A state clause may hold the float forms above. Its
  evaluation yields the completed Boolean, `Undefined` or `Refused` outcome
  that QSpec FR-148 gives the operation (FR-107, FR-100).
- **Absent profile at evaluation.** The evaluator SHALL end an IEEE node reached
  with no admitted `ieee_profile` in the internal failure
  `ieee-profile-unresolved` (FR-090).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-397-AC-1 | Under `profile v = "quire.value.complete/v1"`, `function add using v(a: Float64[nearest-even], b: Float64[nearest-even]): Float64[nearest-even] pure { a + b }` checks, and the `quire.op.ieee.float64.add` node carries the law `ieee_profile` with that `DefinitionRef` and mode `rounding` `nearest-even`; the same with `Float32` gives `quire.op.ieee.float32.add`. | Test (TC-1091) |
| FR-397-AC-2 | **RETIRED (the package-level selection of the IEEE profile is withdrawn).** The criterion asserted the selection rows only; no replacement. | Test (TC-1091), retired |
| FR-397-AC-3 | `function to_r using v(a: Float64[nearest-even]): Rational[-100, 100; 1, 16] pure { convert<Rational[-100, 100; 1, 16]>(a) }` lowers its conversion to `quire.op.ieee.to_rational` with the law `ieee_profile`. | Test (TC-1091) |
| FR-397-AC-4 | The V2 reader admits the packages emitted for the `add` functions of FR-397-AC-1 over `Float64` and over `Float32`, for `to_r` of FR-397-AC-3, and for one function of each form of FR-397-AC-8, FR-397-AC-10, FR-397-AC-12, FR-397-AC-15, FR-397-AC-17, FR-397-AC-21 and FR-397-AC-22. | Test (TC-1091) |
| FR-397-AC-5 | A header `profile w = "quire.value.ieee754-2019-default/v1"` refuses `unknown_profile`/`wrong-selection-role` in a unit that holds a float function and in one that holds none. | Test (TC-1091) |
| FR-397-AC-6 | `+`, `-`, `*` and `/` over unlike float widths, over unlike rounding modes (`Float64[nearest-even]` and `Float64[toward-zero]`), or over a float and an `Int[..]`, `Rational[..]` or `Decimal[..]` parameter refuse `ill_typed`/`type-mismatch`; `a = b`, `a != b`, `a < b`, `a <= b`, `a > b` and `a >= b` over the same pairs, in either order, refuse `ill_typed`/`type-mismatch` at the operator. | Test (TC-1091) |
| FR-397-AC-7 | `quire::value::ieee::numericEqual(a, b)`, `totalOrder(a, b)` and `bitIdentical(a, b)` over two `Float64[nearest-even]` parameters have type `Boolean` and lower to `quire.op.ieee.numeric_equal`, `quire.op.ieee.total_order` and `quire.op.ieee.bit_identical` with the law `ieee_profile`, no mode, and the arguments in call order; over two `Float32[nearest-even]` parameters the same three nodes check. | Test (TC-1091) |
| FR-397-AC-8 | `convert<Float64[nearest-even]>(i)` over an `Int[0, 10]` parameter, over a `Rational[-10, 10; 1, 10]` parameter and over a `Decimal[0, 100; 0, 3]` parameter lowers to `quire.op.ieee.from_exact` with the law `ieee_profile`, mode `rounding` `nearest-even` and member `type_argument` of the target's node. `convert<Float32[toward-zero]>(f)` over a `Float64[nearest-even]` parameter lowers to `quire.op.ieee.to_float32` with mode `rounding` `toward-zero`, and `convert<Float64[exact]>(f)` over a `Float32[nearest-even]` parameter to `quire.op.ieee.to_float64` with no mode. `convert<Float64[toward-zero]>(f)` over a `Float64[nearest-even]` parameter is typed `Float64[toward-zero]` and builds no node. | Test (TC-1091) |
| FR-397-AC-9 | The evaluator's `ieee-profile-unresolved` failure for an IEEE node reached with no admitted `ieee_profile` is exercised by FR-090-AC-15, through TC-917's `CheckedBody::fixture` construction of an `Ieee` and an `IeeeToRational` node over `population_function_package()`. | Test (TC-917) |
| FR-397-AC-10 | `function one using v(): Float64[nearest-even] pure { float64(bits: 0x3ff0000000000000) }` checks with the constructor typed `Float64[nearest-even]` from the result type, lowers to one `literal` node of `value_kind` `float64_bits` and `value` `3ff0000000000000`, and evaluates to the bits `0x3ff0000000000000`. `float32(bits: 0x3f800000)` over a `Float32[toward-zero]` result gives `value_kind` `float32_bits` and `value` `3f800000`. A constructor with no float expected type (`float64(bits: 0x0000000000000000) = float64(bits: 0x8000000000000000)`) is typed `Float64[exact]`. A constructor whose bits are a signaling NaN (`0x7ff0000000000001`), a negative quiet NaN with payload (`0xfff8000000000005`), `-0` and `+inf` evaluates to exactly its bits with no flag. | Test (TC-1091) |
| FR-397-AC-11 | `float64(bits: 0x3ff0)`, `float64(bits: 0x3FF0000000000000)`, `float32(bits: 3f800000)`, `float32(bits: 0x3f80000g)` and `float64(0x3ff0000000000000)` refuse `invalid_syntax`/`invalid-token` at the constructor. `float32(bits: 0x3f800000)` where a `Float64[nearest-even]` is expected, and `float64(bits: 0x3ff0000000000000)` where an `Int[0, 10]` is expected, refuse `ill_typed`/`type-mismatch`. | Test (TC-1091) |
| FR-397-AC-12 | Over `a: Float64[nearest-even]` and `b: Float64[nearest-even]`, `a = b` lowers to one `quire.op.ieee.numeric_equal` node; `a != b` to `quire.op.boolean.not` over it; `a < b` and `a <= b` to the compositions of the FR-397 Comparison bullets with `N(a)` and `N(b)` as `numeric_equal(a, a)` and `numeric_equal(b, b)`; `a > b` to the node of `b < a` and `a >= b` to the node of `b <= a`; no comparison node is `bit_identical`, and every `numeric_equal` and `total_order` node carries the law `ieee_profile`. The same over `Float32`. | Test (TC-1091) |
| FR-397-AC-13 | With the constants `p0` = `float64(bits: 0x0000000000000000)`, `n0` = `0x8000000000000000`, `one` = `0x3ff0000000000000`, `pinf` = `0x7ff0000000000000`, `ninf` = `0xfff0000000000000`, `qnan` = `0x7ff8000000000000` and `snan` = `0x7ff0000000000001`: `n0 = p0` is `true`, `n0 < p0` is `false`, `n0 <= p0` is `true`, `p0 >= n0` is `true`, `p0 > n0` is `false`, `one < pinf` is `true`, `ninf < one` is `true`, `qnan = qnan` is `false`, `qnan != qnan` is `true`, `qnan != one` is `true`, `qnan < one`, `qnan <= one`, `one > qnan`, `one >= qnan`, `snan < one` and `snan = snan` are `false`, and `pinf = pinf` and `pinf <= pinf` are `true`. The same vectors over `float32(bits: ..)` constants (`0x00000000`, `0x80000000`, `0x3f800000`, `0x7f800000`, `0xff800000`, `0x7fc00000`, `0x7f800001`) give the same results. | Test (TC-1091) |
| FR-397-AC-14 | Over `r: R` with `record R { x: Float64[nearest-even] }`, over `Option<Float64[nearest-even]>` parameters, over `Sequence<Float64[nearest-even]>` parameters and over a tuple holding a float, `=` and `!=` check with type `Boolean`; the unary `-` over `a: Float64[nearest-even]` checks with type `Float64[nearest-even]`; `r.x = r.x` checks. `-i` over an `Int[0, 10]` parameter keeps its exact-number lowering, and `-b` over a `Boolean` parameter refuses `ill_typed`/`type-mismatch` at the operand. | Test (TC-1091) |
| FR-397-AC-15 | `quire::value::ieee::sqrt(a)` over `a: Float64[nearest-even]` lowers to `quire.op.ieee.float64.sqrt` with mode `rounding` `nearest-even` and the law `ieee_profile`, and `quire::value::ieee::fma(a, b, c)` over three `Float64[nearest-even]` parameters to `quire.op.ieee.float64.fma`; the same over `Float32` gives the `float32` identities. `sqrt(float64(bits: 0x4000000000000000))` over a `Float64[nearest-even]` result evaluates to `0x3ff6a09e667f3bcd` and over `Float64[toward-zero]` to `0x3ff6a09e667f3bcc`; over `Float64[exact]` it is `refused { code: ieee_not_exact, cause: rounding-required }`. `sqrt(float64(bits: 0x4010000000000000))` over `Float64[exact]` is `0x4000000000000000`, `sqrt` of `0x8000000000000000` is `0x8000000000000000` and `sqrt` of `0xbff0000000000000` is `0x7ff8000000000000` with the `invalid` flag. `fma(0x3ff0000000000001, 0x3fefffffffffffff, 0xbff0000000000000)` over `Float64[nearest-even]` is `0x3c9ffffffffffffe`, while the same multiply followed by an add is `0x0000000000000000`. | Test (TC-1091) |
| FR-397-AC-16 | `quire::value::ieee::sqrt(a, b)`, `sqrt()`, `fma(a, b)` and `numericEqual(a)` refuse `ill_typed`/`type-mismatch` at the call; `sqrt(i)` over an `Int[0, 10]` parameter, `totalOrder(f32, f64)` over a `Float32` and a `Float64`, `numericEqual(a, b)` over `Float64[nearest-even]` and `Float64[toward-zero]` parameters and `fma(a, b, i)` refuse `ill_typed`/`type-mismatch` at the argument. `quire::value::ieee::cbrt(a)` refuses `missing_declaration`/`missing-name`. A supplied library that exports a function declaration under the qualified identity `quire::value::ieee::sqrt` makes the call `quire::value::ieee::sqrt(a)` refuse `invalid_package`/`conflicting-definition` naming `quire::value::ieee::sqrt`. `totalOrder(float64(bits: 0x8000000000000000), float64(bits: 0x0000000000000000))` is `true` and with the operands exchanged `false`; `totalOrder(0x7ff0000000000000, 0x7ff8000000000000)` is `true` and with the operands exchanged `false`; `totalOrder(0x7ff0000000000001, 0x7ff8000000000000)` is `true`; `totalOrder(0xfff8000000000000, 0xfff0000000000000)` is `true`; `bitIdentical(0x8000000000000000, 0x0000000000000000)` is `false` while `numericEqual` of the same pair is `true`; `bitIdentical(0x7ff8000000000000, 0x7ff8000000000000)` is `true` while `numericEqual` of the same pair is `false`. | Test (TC-1091) |
| FR-397-AC-17 | Over `Float64[nearest-even]` results: `convert` of the `Int[0, 20]` parameter value 3 gives `0x4008000000000000`; of the `Rational[-10, 10; 1, 10]` value 1/10 gives `0x3fb999999999999a`; of the `Decimal[0, 100; 0, 3]` value `0.1` gives `0x3fb999999999999a`; of the `Integer` value 9007199254740993 gives `0x4340000000000000`, under `Float64[toward-positive]` gives `0x4340000000000001`, and under `Float64[exact]` is `refused { code: ieee_not_exact, cause: rounding-required }`. Over `Float32` results the `Rational[-10, 10; 1, 10]` value 1/3 gives `0x3eaaaaab` under `nearest-even` and `0x3eaaaaaa` under `toward-zero`. `convert<Float32[nearest-even]>` of `0x3ff0000000000001` gives `0x3f800000`, of `0x7ff8000000000001` gives `0x7fc00001`, and of `0x7ff8000040000000` is `refused { code: ieee_nan_payload_not_representable, cause: payload-exceeds-target }`; under `Float32[exact]` the first is `refused { code: ieee_not_exact }`. `convert<Float64[exact]>` of `0x3f800000` gives `0x3ff0000000000000`, of `0x80000000` gives `0x8000000000000000`, of `0x7f800000` gives `0x7ff0000000000000`, and of `0x7fc00001` gives `0x7ff8000000000001`. | Test (TC-1091) |
| FR-397-AC-18 | `convert<Decimal[0, 100; 0, 3]>(a)`, `convert<Integer>(a)` and `convert<Int[0, 10]>(a)` over `a: Float64[nearest-even]` refuse `ill_typed`/`type-mismatch`. `convert<Float64[nearest-even]>(q)` over a `Quantity<ms>` parameter and over a `Boolean` parameter refuse `ill_typed`/`type-mismatch`; `convert<Float64[nearest-even]>(f)` over a `Float64[nearest-even]` parameter checks and builds no conversion node. `convert<Float64[toward-zero]>(f)` and `convert<Float64[exact]>(f)` over a `Float64[nearest-even]` parameter check with the target type and build no node, and evaluate to the operand's exact bits for `0x3ff0000000000001`, `0x8000000000000000`, `0xfff0000000000000`, `0x7ff0000000000001` (still signaling, no flag) and `0xfff8000000000005`; `convert<Float32[exact]>(f)` over a `Float32[toward-zero]` parameter does the same, and `convert<Float64[toward-zero]>(f) + b` over `b: Float64[toward-zero]` lowers to one `quire.op.ieee.float64.add` with mode `rounding` `toward-zero`. `convert<Rational[-100, 100; 1, 16]>(a)` of `0x7ff0000000000000` evaluates `Undefined`, of `0x8000000000000000` gives the rational 0, of `0x4059000000000001` is `refused { code: ieee_rational_out_of_domain }`. | Test (TC-1091) |
| FR-397-AC-19 | `function same using v(): Boolean pure { float64(bits: 0x3ff0000000000000) = float64(bits: 0x3ff0000000000000) }`, a function whose only float form is a `quire::value::ieee::bitIdentical` call over two `Float64` constructors, and a function whose only float form is `convert<Float64[nearest-even]>(i)` over an `Int[0, 10]` parameter each declare no float type, and each carries the law on its IEEE node. | Test (TC-1091) |
| FR-397-AC-20 | A state clause holding `float64(bits: 0x7ff8000000000000) != float64(bits: 0x7ff8000000000000)` evaluates to `Completed(true)`, one holding `quire::value::ieee::sqrt` under `Float64[exact]` of a constant that is not a perfect square, compared to a constant, evaluates to the kernel `Refused` `ieee_not_exact` outcome, and the clause set containing it has outcome `Undecided`. | Test (TC-1091) |
| FR-397-AC-21 | `function neg using v(a: Float64[nearest-even]): Float64[nearest-even] pure { -a }` lowers to one `quire.op.ieee.float64.negate` node with the law `ieee_profile`, no mode and the node `Float64[nearest-even]` as its type; the same over `Float32[toward-zero]` gives `quire.op.ieee.float32.negate`. `-float64(bits: 0x3ff0000000000000)` over a `Float64[nearest-even]` result is `0xbff0000000000000` with the constructor typed `Float64[nearest-even]`; `-float64(bits: 0x0000000000000000)` is `0x8000000000000000`, `-float64(bits: 0x8000000000000000)` is `0x0000000000000000`, `-float64(bits: 0x7ff0000000000001)` is `0xfff0000000000001` with no flag, `-float64(bits: 0xfff8000000000005)` is `0x7ff8000000000005` and `-float32(bits: 0x7f800000)` is `0xff800000`; the same under `Float64[exact]` gives the same bits and no refusal, and `-(-float64(bits: 0x7ff0000000000001))` is `0x7ff0000000000001`. `-a` where a `Float64[toward-zero]` is expected over a `Float64[nearest-even]` parameter refuses `ill_typed`/`type-mismatch`. A state clause holding `-float64(bits: 0x7ff8000000000000) != -float64(bits: 0x7ff8000000000000)` evaluates to `Completed(true)`. | Test (TC-1091) |
| FR-397-AC-22 | Over `record R { x: Float64[nearest-even]; y: Integer }`, `Option<Float64[nearest-even]>`, `union U { V(Float64[nearest-even]), N }`, `Sequence<Float64[nearest-even]>[0, 2]` and a tuple `(Float32[exact], Integer)`, `=` lowers to one `quire.op.structural.eq` node and `!=` to `quire.op.structural.ne`, each with leaves `operand:0` listing the float leaf with the law `ieee_profile` and mode `null` at the path QSpec FR-322 gives (`["field:x"]`, `["inner"]`, `["member:V", "position:0"]`, `["inner"]` and `["position:0"]`). With `p0` `0x0000000000000000`, `n0` `0x8000000000000000`, `one` `0x3ff0000000000000`, `qnan` `0x7ff8000000000000` and `qnan2` `0x7ff8000000000001`: `R { x: p0, y: 1 } = R { x: n0, y: 1 }` is true, `R { x: qnan, y: 1 } = R { x: qnan, y: 1 }` is false and its `!=` is true, `R { x: p0, y: 1 } = R { x: p0, y: 2 }` is false, `U::V(p0) = U::V(n0)` is true, `U::V(p0) = U::N` is false, the present `qnan` against itself is false, `sequence[p0, one] = sequence[n0, one]` is true, `sequence[qnan] = sequence[qnan]` is false, and the tuple of `Float32` `0x80000000` and `1` equals the tuple of `Float32` `0x00000000` and `1`. A record holding a float and a text field lists both leaves in field order. | Test (TC-1091) |
| FR-397-AC-23 | Over `Set<Float64[nearest-even]>[0, 2]` parameters, `=` lowers to one `quire.op.structural.eq` node and `!=` to `quire.op.structural.ne`, each with leaves `operand:0` listing the float leaf at the path `["inner"]` with the law `ieee_profile` and mode `null`. Set members match by bit identity and leaves compare by `numericEqual`: with `p0`, `n0`, `qnan` and `qnan2` of FR-397-AC-22, `set[qnan] = set[qnan]` is true, `set[p0] = set[n0]` is false and its `!=` is true, `set[p0, n0] = set[n0, p0]` is true, and `set[qnan] = set[qnan2]` is false; over `Bag<Float64[nearest-even]>[0, 2]`, `bag[qnan] = bag[qnan]` is true and `bag[p0] = bag[n0]` is false; `sequence[qnan] = sequence[qnan]` over `Sequence<Float64[nearest-even]>[0, 2]` stays false. | Test (TC-1091) |
| FR-397-AC-24 | Source forms (intrinsic names unqualified after `quire::value::ieee::` as in FR-397-AC-7) for the QSpec TC-193 rows that need the callable relations, the fused multiply-add and the ordering operators (QSL-1388; the callable surface is intended-present, not admission-only). With `HEADER` as in FR-397-AC-1 and `p0` `float32(bits: 0x00000000)`, `n0` `0x80000000`: F01 `p0 = n0` is `true`, `quire::value::ieee::bitIdentical(n0, p0)` is `false`, `totalOrder(n0, p0)` is `true` and `totalOrder(p0, n0)` is `false`. F02 over `0x7fc00001` and `0x7fc00002`: `a = a` is `false`, `bitIdentical(a, a)` is `true`, `totalOrder(a, b)` is `true` and `totalOrder(b, a)` is `false`. F02b: `totalOrder(0xffc00002, 0xffc00001)` and `totalOrder(0x7f800001, 0x7fc00001)` are `true`, each with the operands exchanged `false`. F04b over `Float32[nearest-even]` results: `float32(bits: 0x7fc00012) + float32(bits: 0xff800021)` and `fma(float32(bits: 0x3f800000), float32(bits: 0x7fc00012), float32(bits: 0xff800021))` each evaluate to `0x7fc00012` with the `invalid` flag. F05 over `0xff800000`, `0xff7fffff`, `0x7f7fffff` and `0x7f800000`: each ascending pair (`0xff800000` to `0xff7fffff`, `0xff7fffff` to `0x7f7fffff`, `0x7f7fffff` to `0x7f800000`) satisfies `<`, `<=` and `totalOrder` and fails `numericEqual`, the exchanged pair fails `<` and `<=`, `bitIdentical(0x7f800000, 0x7f800000)` is `true` and `bitIdentical(0x7f800000, 0xff800000)` is `false`. F11 `fma(float32(bits: 0x3f800001), float32(bits: 0x3f7ffffe), float32(bits: 0xbf800000))` over a `Float32[nearest-even]` result is `0xa8800000`, while the separately rounded `a * b + c` is `0x00000000`. F20: `numericEqual`, `totalOrder` and `bitIdentical` over a `Float32` and a `Float64`, `a + b` over the same pair, `a + i` over a `Float32[nearest-even]` and an `Int[0, 10]` parameter, and `convert<Decimal[0, 10; 0, 0; nearest-even]>(a)` and `convert<Int[0, 1]>(a)` over a `Float32[nearest-even]` parameter each refuse `ill_typed`/`type-mismatch` at check, so no charge is made. F34 is FR-397-AC-13 at both widths. | Test (TC-1091) |

## Dependencies

- FR-093 (the IEEE rows and the law rule), FR-110 (header selections), FR-092
  (the float types), FR-091 (the S1 and S2 forms of the bit constructor), FR-090
  (the internal failure), FR-107 and FR-100 (clause and refusal outcomes),
  QSpec FR-148 (the profile) and QSpec
  FR-322 (the IEEE operation identities and the float literal term).
