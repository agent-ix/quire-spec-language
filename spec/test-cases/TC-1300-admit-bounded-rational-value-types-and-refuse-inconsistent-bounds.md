---
id: TC-1300
title: "Admit bounded Rational value types and refuse inconsistent bounds"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-505
    type: verifies
---
# TC-1300: Admit bounded Rational value types and refuse inconsistent bounds

## Description

Verify the Rational value type of FR-505: the shared `rational_range` node,
the unbounded empty-constraint case, the ordered reader refusals, alias,
parameter and foreign-constraint restatement, value admission, the unchanged
bare native, tuple positions and variant payloads, the unit-bearing carried quantity and its comparison through
`convert<Quantity<u>>`. Scope: FR-505-AC-1 to FR-505-AC-10. The QSpec vectors
are TC-981 `RV-01` to `RV-12`, referenced by row id and never copied.

## Test Procedure

Fixture `R0` is a domain package with object type `Gauge` whose field `level`,
operation `adjust` parameter `delta` and result are typed `Ratio`, where
`Ratio` has `scalar` `rational` and the constraints `maxDenominator` 12,
`minNumerator` -3, `minDenominator` 1 and `maxNumerator` 7, in that order,
each with `appliesTo` `Ratio`. The unit rows add `dimension Length;`,
`unit m : Length = rational(1, 1);` and `unit cm : Length = rational(1, 100);`.
Each row below mutates `R0` as stated and admits it.

| Row | Mutation of R0 | Expected |
| --- | --- | --- |
| 1 | none; then add plain alias `Level3` of `Ratio` typing a new field | admitted; `level`, `delta`, the result and the new field have type `Rational[-3, 7; 1, 12]`; one original declaration for `Ratio`; all four name one `bounded_domain`/`rational_range` node over `scalar_type`/`rational` with bindings `numerator_min` -3, `numerator_max` 7, `denominator_min` 1, `denominator_max` 12 |
| 2 | add `Ratio2` with `Ratio`'s bounds typing a new field; then `Ratio13` equal to `Ratio` but `maxDenominator` 13 | `Ratio2`'s field names row 1's node; `Ratio13`'s names another node; two distinct types |
| 3 | `Ratio`'s `constraints` empty | admitted as the unbounded `Rational`; `level` has no checked type |
| 4 | `minNumerator` -9223372036854775808 | admitted |
| 5 | add constraint `minLength` 0 to `Ratio` | `unsupported_construct`/`declaration-form` at `Constraint.keyword` naming `minLength` |
| 6 | `maxNumerator`'s `appliesTo` another value type; then an alias of `Ratio` | each `invalid_model_binding`/`malformed-declaration` at `Constraint.appliesTo` |
| 7 | `maxNumerator` given twice | `invalid_model_binding`/`malformed-declaration` at the second `Constraint.keyword` naming `maxNumerator` |
| 8 | remove `minDenominator` | `invalid_model_binding`/`malformed-declaration` at `TypeDefinition.constraints` naming `minDenominator` |
| 9 | `maxNumerator` operand `1.5`; `1e2`; `"3/4"`; 9223372036854775808; `maxDenominator` operand `0.5`; `maxNumerator` with no `operands.value`; `maxNumerator` `operands` with a second member | each `invalid_model_binding`/`malformed-declaration` at that `Constraint.operands` naming the keyword; no bound rounded or widened |
| 10 | `minNumerator` 8; `minDenominator` 0; `minDenominator` 13; `maxDenominator` 0; `maxDenominator` -1 | each `unsupported_construct`/`declaration-form` at `TypeDefinition.constraints` naming the bounds |
| 11 | `minLength` with `maxNumerator` twice; foreign `appliesTo` with `maxNumerator` twice; `minLength` with foreign `appliesTo`; `maxNumerator` twice with `minDenominator` removed; `minDenominator` removed with `maxNumerator` `1.5`; `maxNumerator` `1.5` with `minNumerator` 8; `maxDenominator` `0.5` listed first with `maxNumerator` `1.5` | one refusal each, of the earlier row: `minLength`; the `appliesTo`; `minLength`; the repeated keyword; the missing keyword; the operand; `maxNumerator` |
| 12 | `level` adds its own `maxDenominator` constraint; `delta` adds its own constraint | `invalid_model_binding`/`malformed-declaration` at `Field.constraints`; the same at `Operation.params[0].constraints` |
| 13 | add alias `Level2` of `Ratio` with a constraint | `invalid_model_binding`/`malformed-declaration` at the alias node and `TypeDefinition.constraints` |
| 14 | add value type `Other` carrying a constraint whose `appliesTo` is `Ratio` | `invalid_model_binding`/`malformed-declaration` at `Other` and `Constraint.appliesTo`; `Ratio` admitted |
| 15 | add a field typed bare `ix://quire/native/Rational` and one typed row 3's empty-constraint value type, then a clause that uses each | admitted; each clause refuses `ill_typed`/`operator-ineligible`; every other declaration's node identity equals row 1's |
| 16 | `Ratio` with `unit` `ix://quire/unit/m`; add a second equal-bounds value type with unit `ix://quire/unit/m`, `Ratio` without unit, and an equal-bounds value type with unit `ix://quire/unit/cm` | `level` has type `Quantity<m, Rational[-3, 7; 1, 12]>`, the `scalar_type`/`carried_quantity` node over `m`'s unit node, row 1's `rational_range` node and `rounding` `exact`; the second `m` type names that node; the no-unit and `cm` types name two other nodes |
| 17 | `unit` `ix://quire/unit/nosuch` | `missing_declaration`/`missing-name` at the value type node and `TypeDefinition.unit` |
| 18 | `unit` `m`; `unit` `1`; `unit` `ix://quire/native/Rational`; `unit` `ix://quire/unit/m` on a value type of `scalar` `string`; `unit` `m` with `minNumerator` 8 | each of the first four `invalid_model_binding`/`malformed-declaration` at `TypeDefinition.unit`; the last only the bounds refusal |
| 19 | row 16's `Ratio` typing fields `a` and `b`, with invariants `self.a = self.b`, `self.a < self.b`, `self.a + self.b = self.a`; then `convert<Quantity<m>>(self.a) = convert<Quantity<m>>(self.b)` and `convert<Quantity<m>>(self.a) < convert<Quantity<m>>(self.b)` | the first three each `ill_typed`/`type-mismatch` at the expression; the last two check |
| 20 | a source function parameter typed `Rational[8, 7; 1, 12]`; one typed `Rational[0, 7; 0, 12]`; the types `Quantity<m, Rational[8, 7; 1, 12]>` and `Quantity<m, Rational[0, 7; 0, 12]>` | each `ill_typed` at the `Rational[..]` type |
| 24 | add tuple type `P` with position `p0` typed `Ratio` and variant type `V` with case `c` whose `payloadType` is `Ratio` | admitted; `p0` and `c`'s payload type `Rational[-3, 7; 1, 12]`, naming row 1's node |
| 25 | row 24 with `Ratio`'s `constraints` empty | `p0` `invalid_model_binding`/`malformed-declaration` at the position; `c` `unsupported_construct`/`declaration-form` at `Variant.payloadType`; both at stage `package_read`; `level` admitted with no checked type |

Value rows, as `level` in a snapshot:

| Row | Package | Value | Expected |
| --- | --- | --- | --- |
| 21 | row 1 | `7/12`, `-3/1`, `0/1` | admitted |
| 22 | row 1 | `8/1`, `1/13`, `2/4`, `-4/1`, float `0.5` | each `invalid_runtime_input`/`invalid-value` naming the member path |
| 23 | row 16 | `1/4`; `1/13` | `1/4 m` admitted; `1/13` refuses as row 22 |

Tag the test `#[trace("FR-505-AC-1", "FR-505-AC-2", "FR-505-AC-3", "FR-505-AC-4", "FR-505-AC-5", "FR-505-AC-6", "FR-505-AC-7", "FR-505-AC-8", "FR-505-AC-9", "FR-505-AC-10", "TC-1300")]`.

## Expected Results

Each row's Expected column. Rows 5-14, 17-18 and 25 report exactly one refusal of
the value type (or of the carrying node, rows 12-14), at the named path; rows
21-23 change no value.
