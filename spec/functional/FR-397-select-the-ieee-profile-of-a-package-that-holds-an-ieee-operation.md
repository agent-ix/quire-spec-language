---
id: FR-397
title: "Select the IEEE profile of a package that holds an IEEE operation"
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
  - target: ix://agent-ix/quire-specification/FR-148
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-201
    type: depends_on
---
# FR-397: Select the IEEE profile of a package that holds an IEEE operation

## Description

A unit that declares `Float32[mode]` or `Float64[mode]` values and operates on
them needs the IEEE profile definition the lock catalog names under the role
`ieee_profile` (`quire.value.ieee754-2019-default/v1`, QSpec FR-148). A header
`profile` cannot select it (FR-110-AC-3: the role is not header-selectable), and
QSpec's package selection makes it a conditional slot of the closed trigger
`ieee_operation` (QSpec FR-201-AC-9). This requirement states what raises that
trigger in QSL source, how the profile reaches the package, and what a
comparison of floats does.

## Inputs

- A unit, with its supplied libraries, whose checked bodies may hold floating-point
  values (FR-092's `Float32[mode]` and `Float64[mode]` types; the bit-level
  `float32(bits: ...)` and `float64(bits: ...)` constructors).
- The `DefinitionLock` catalog row of the role `ieee_profile`.

## Outputs

- For a package that holds an IEEE operation, the lock's `definition_selections`
  list the `ieee_profile` row once, as `{authority, identity}`, and each node of
  an IEEE operation carries the law `ieee_profile` with that `DefinitionRef`
  (FR-093).
- For a package that holds none, no `ieee_profile` selection and no such law.

## Behavior

- **The trigger.** An *IEEE operation* is a checked `Ieee(operator)` arithmetic
  application over two `Float32` or two `Float64` operands, or a checked
  conversion of a `Float32` or `Float64` operand to a `Rational` (FR-093's
  `IeeeToRational`), in any unit of the package. A declaration, a parameter, a
  return type or a `float32`/`float64` bit constructor alone is not an IEEE
  operation.
- **The selection.**
  - The checker SHALL record the `ieee_operation` trigger for a package that
    holds an IEEE operation.
  - The package's definition selection SHALL include the catalog row of the
    role `ieee_profile` when the trigger is recorded.
  - The package's definition selection SHALL NOT include that row for a package
    without the trigger.
  - A header `profile` naming its identity SHALL still refuse
    `unknown_profile`/`wrong-selection-role` (FR-110-AC-3), and no other unit
    declaration supplies the row.
- **The profile in use.**
  - The checker SHALL build the unit's IEEE profile from the selected row.
  - The rounding mode of an operation is the mode the operand types carry
    (FR-092), so `Float64[nearest-even]` operands round to nearest even.
  - The law of an IEEE node SHALL be the selected row and no constant
    (FR-093).
- **Float-to-rational conversion.** A `convert<Rational[...]>(e)` whose operand is
  a `Float32` or `Float64` lowers to `quire.op.ieee.to_rational` (FR-093); its
  value and loss are QSpec FR-148's.
- **Comparison.** Numeric equality, total ordering and bitwise identity are
  three distinct selected operations (QSpec FR-148) and no operator spelling
  chooses among them, so none is lowered.
  - The checker SHALL refuse `=`, `!=`, `<`, `<=`, `>` and `>=` over `Float32`
    or `Float64` operands as `ill_typed`/`operator-ineligible` at the operator.
  - The checker SHALL refuse operands of unlike widths, or a float mixed with an
    exact type, as `ill_typed` (QSpec FR-148).
- **Absent profile at evaluation.** The evaluator SHALL end an IEEE node reached
  with no admitted `ieee_profile` in the internal failure
  `ieee-profile-unresolved` (FR-090).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-397-AC-1 | Under `profile v = "quire.value.complete/v1"`, `function add using v(a: Float64[nearest-even], b: Float64[nearest-even]): Float64[nearest-even] pure { a + b }` checks, the package's `definition_selections` list the `ieee_profile` row once, and the `quire.op.ieee.float64.add` node carries the law `ieee_profile` with that `DefinitionRef` and mode `rounding` `nearest-even`; the same with `Float32` gives `quire.op.ieee.float32.add`. | Test (TC-1091) |
| FR-397-AC-2 | `function keep using v(a: Float64[nearest-even]): Boolean pure { true }` checks and its package selects no `ieee_profile`, its `definition_selections` hold no such row and no node carries the law; `function one using v(): Float64[nearest-even] pure { float64(bits: 0x3ff0000000000000) }` also selects none. | Test (TC-1091) |
| FR-397-AC-3 | `function to_r using v(a: Float64[nearest-even]): Rational[-100, 100; 1, 16] pure { convert<Rational[-100, 100; 1, 16]>(a) }` lowers its conversion to `quire.op.ieee.to_rational` with the law `ieee_profile` and the package selects the `ieee_profile` row. | Test (TC-1091) |
| FR-397-AC-4 | A header `profile w = "quire.value.ieee754-2019-default/v1"` still refuses `unknown_profile`/`wrong-selection-role`, in a unit that holds an IEEE operation as in one that holds none. | Test (TC-1091) |
| FR-397-AC-5 | `a = b`, `a != b`, `a < b`, `a <= b`, `a > b` and `a >= b` over two `Float64[nearest-even]` parameters each refuse `ill_typed`/`operator-ineligible` at the operator, and `a + b` over a `Float32` and a `Float64` refuses `ill_typed`. | Test (TC-1091) |
| FR-397-AC-6 | A package with an IEEE operation, built for an evaluator input with no admitted `ieee_profile`, ends in the internal failure `ieee-profile-unresolved`. | Test (TC-1091) |

## Dependencies

- FR-093 (the IEEE rows and the law rule), FR-110 (header selections), FR-092
  (the float types), FR-090 (the internal failure), QSpec FR-148 and FR-201
  (the profile and the closed triggers).
