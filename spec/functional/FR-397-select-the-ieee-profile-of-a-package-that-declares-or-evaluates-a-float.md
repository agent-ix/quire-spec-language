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
  - target: ix://agent-ix/quire-specification/FR-148
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-201
    type: depends_on
---
# FR-397: Select the IEEE profile of a package that declares or evaluates a float

## Description

QSpec's lock catalog names the IEEE definition `quire.value.ieee754-2019-default/v1`
under the role `ieee_profile` (QSpec FR-148). The role is not header-selectable
(FR-110-AC-3). QSpec's package selection makes it a conditional slot of the
closed trigger `ieee_operation` (QSpec FR-201-AC-9), which a package raises when
it declares or evaluates a `float32` or `float64` type, value or operation
(QSpec `value-complete.md`). This requirement states how QSL source raises that
trigger, how the profile reaches the package, what each float source form
does, and what a comparison of floats does.

## Inputs

- A unit, with its supplied libraries, whose source may use `Float32[mode]` and
  `Float64[mode]` types (FR-092).
- The `DefinitionLock` catalog row of the role `ieee_profile`.

Complete-V1 source has no float constant, no conversion into a float and no
float comparison: the bit-level `float32(bits: ...)` and `float64(bits: ...)`
constructors are not admitted (S2 refuses them as
`UnrepresentedConstruct{FloatValue}`, FR-091). A float value in a body therefore
comes only from a declared parameter or field, or from an IEEE arithmetic
result whose operands ultimately come from declarations. Model float fields
and operations are refused at intake. Scanning resolved declarations therefore
covers every body IEEE application today; any future float source must extend
the trigger before its body is checked.

## Outputs

- For a package that declares or evaluates a float, the lock's
  `definition_selections` list the `ieee_profile` row once, as `{authority,
  identity}`, and each node of an IEEE operation carries the law `ieee_profile`
  with that `DefinitionRef` (FR-093).
- For a package that does neither, no `ieee_profile` selection and no such law.

## Behavior

- **The trigger.** A package raises `ieee_operation` when any unit of the
  package declares a `Float32[mode]` or `Float64[mode]` type (a parameter, a
  result or a field) or holds a checked IEEE arithmetic application over two
  `Float32` or two `Float64` operands or a checked conversion of a float to a
  `Rational` (FR-093's `Ieee(operator)` and `IeeeToRational`).
- **The selection.**
  - The checker SHALL record the `ieee_operation` trigger for a package that
    raises it.
  - The checker SHALL include the catalog row of the role `ieee_profile` in the
    package's definition selection when the trigger is recorded.
  - The checker SHALL NOT include that row for a package without the trigger.
  - The checker SHALL refuse a header `profile` that names the row's identity as
    `unknown_profile`/`wrong-selection-role` (FR-110-AC-3), and no other unit
    declaration supplies the row.
- **The profile in use.**
  - The checker SHALL build the unit's IEEE profile from the selected row.
  - The rounding mode of an operation is the mode the operand types carry
    (FR-092), so `Float64[nearest-even]` operands round to nearest even.
  - The checker SHALL write the law of an IEEE node as the selected row and
    never from a constant (FR-093).
- **Float-to-rational conversion.** The checker SHALL lower a
  `convert<Rational[...]>(e)` whose operand is a `Float32` or `Float64` to
  `quire.op.ieee.to_rational` (FR-093). Its value and loss are QSpec FR-148's.
- **Conversion into a float.**
  - The checker SHALL refuse a `convert` whose target is a `Float32` or `Float64`
    (from an exact type, or between the two widths) as
    `ill_typed`/`type-mismatch`.
- **Strict admission.** The V2 reader (`quire.checked-package/v2`) SHALL admit the
  package emitted for a unit whose IEEE operations are the arithmetic of a
  `Float32` or `Float64` function and the float-to-rational conversion.
- **Comparison.** Numeric equality, total ordering and bitwise identity are three
  distinct selected operations (QSpec FR-148), and no operator spelling chooses
  among them.
  - The checker SHALL refuse `=`, `!=`, `<`, `<=`, `>` and `>=` when either
    operand is `Float32` or `Float64` as `ill_typed`/`operator-ineligible` at
    the operator, including comparisons of unlike widths or with an exact type.
- **Arithmetic operand types.** The checker SHALL refuse arithmetic operands
  of unlike float widths, or a float mixed with an exact type, as
  `ill_typed`/`type-mismatch`.
- **Reserved intrinsics.** The reserved intrinsic identities
  `quire::value::ieee::sqrt`, `fma`, `numericEqual`, `totalOrder` and
  `bitIdentical` (QSpec FR-148) are admitted in no call in complete-V1 source.
  - The checker SHALL refuse a call to one as `missing_declaration`/`missing-name`
    naming the identity.
- **Absent profile at evaluation.** The evaluator SHALL end an IEEE node reached
  with no admitted `ieee_profile` in the internal failure
  `ieee-profile-unresolved` (FR-090).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-397-AC-1 | Under `profile v = "quire.value.complete/v1"`, `function add using v(a: Float64[nearest-even], b: Float64[nearest-even]): Float64[nearest-even] pure { a + b }` checks, the package's `definition_selections` list the `ieee_profile` row once, and the `quire.op.ieee.float64.add` node carries the law `ieee_profile` with that `DefinitionRef` and mode `rounding` `nearest-even`; the same with `Float32` gives `quire.op.ieee.float32.add`. | Test (TC-1091) |
| FR-397-AC-2 | `function keep using v(a: Float64[nearest-even]): Boolean pure { true }` declares a float and holds no IEEE operation, and its package selects the `ieee_profile` row once; `function plain using v(a: Int[0, 10], b: Int[0, 10]): Int[0, 20] pure { a + b }` selects none and no node carries the law. | Test (TC-1091) |
| FR-397-AC-3 | `function to_r using v(a: Float64[nearest-even]): Rational[-100, 100; 1, 16] pure { convert<Rational[-100, 100; 1, 16]>(a) }` lowers its conversion to `quire.op.ieee.to_rational` with the law `ieee_profile`, and the package selects the `ieee_profile` row. | Test (TC-1091) |
| FR-397-AC-4 | The V2 reader admits the packages emitted for the `add` functions of FR-397-AC-1 over `Float64` and over `Float32` and for `to_r` of FR-397-AC-3. | Test (TC-1091) |
| FR-397-AC-5 | A header `profile w = "quire.value.ieee754-2019-default/v1"` refuses `unknown_profile`/`wrong-selection-role` in a unit that holds a float function and in one that holds none. | Test (TC-1091) |
| FR-397-AC-6 | `a = b`, `a != b`, `a < b`, `a <= b`, `a > b` and `a >= b` refuse `ill_typed`/`operator-ineligible` at the operator when either parameter is `Float32[nearest-even]` or `Float64[nearest-even]`, including unlike widths and float/exact pairs in either order. Arithmetic `+`, `-`, `*` and `/` over unlike float widths or a float with an `Int` or `Rational` parameter refuses `ill_typed`/`type-mismatch`. | Test (TC-1091) |
| FR-397-AC-7 | A call `quire::value::ieee::numericEqual(a, b)` over two `Float64[nearest-even]` parameters, and one of each other reserved intrinsic, refuses `missing_declaration`/`missing-name` naming the identity. | Test (TC-1091) |
| FR-397-AC-8 | `convert<Float64[nearest-even]>(i)` over an `Int[0, 10]` parameter, `convert<Float64[nearest-even]>(f)` over a `Float32[nearest-even]` parameter and `convert<Float32[nearest-even]>(d)` over a `Float64[nearest-even]` parameter each refuse `ill_typed`/`type-mismatch`. | Test (TC-1091) |
| FR-397-AC-9 | The evaluator's `ieee-profile-unresolved` failure for an IEEE node reached with no admitted `ieee_profile` is exercised by FR-090-AC-15, through TC-917's `CheckedBody::fixture` construction of an `Ieee` and an `IeeeToRational` node over `population_function_package()`. | Test (TC-917) |

## Dependencies

- FR-093 (the IEEE rows and the law rule), FR-110 (header selections), FR-092
  (the float types), FR-091 (the S2 refusal of bit constructors), FR-090 (the
  internal failure), QSpec FR-148 and FR-201 (the profile and the closed
  triggers).
