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
`convert<Quantity<u>>`, and the resolution of a bare unit name against the
source and the selected packages, a package unit being reached only
through its alias. Scope: FR-505-AC-1 to FR-505-AC-12. The QSpec vectors
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
| 4 | `minNumerator` `"-9223372036854775808"`; `maxNumerator` `"9223372036854775807"`; numeric `minNumerator` -9007199254740992; numeric `maxNumerator` 9007199254740992 | each admitted with exact unchanged bounds |
| 5 | add constraint `minLength` 0 to `Ratio` | `unsupported_construct`/`declaration-form` at `Constraint.keyword` naming `minLength` |
| 6 | `maxNumerator`'s `appliesTo` another value type; then an alias of `Ratio` | each `invalid_model_binding`/`malformed-declaration` at `Constraint.appliesTo` |
| 7 | `maxNumerator` given twice | `invalid_model_binding`/`malformed-declaration` at the second `Constraint.keyword` naming `maxNumerator` |
| 8 | remove `minDenominator` | `invalid_model_binding`/`malformed-declaration` at `TypeDefinition.constraints` naming `minDenominator` |
| 9 | `maxNumerator` operand `1.5`; `1e2`; `"3/4"`; `"01"`; `"+1"`; `"-0"`; `"9223372036854775808"`; `"-9223372036854775809"`; `maxDenominator` operand `0.5`; `maxNumerator` with no `operands.value`; `maxNumerator` `operands` with a second member | each `invalid_model_binding`/`malformed-declaration` at that `Constraint.operands` naming the keyword; no bound rounded or widened |
| 26 | numeric `minNumerator` 9007199254740993, 9007199254740994 and -9007199254740994 | each FR-272 `noncanonical_wire`/`inexact-integer` at the operand document pointer before intake; never an FR-154 operand refusal |
| 10 | `minNumerator` 8; `minDenominator` 0; `minDenominator` 13; `maxDenominator` 0; `maxDenominator` -1 | each `unsupported_construct`/`declaration-form` at `TypeDefinition.constraints` naming the bounds |
| 11 | `minLength` with `maxNumerator` twice; foreign `appliesTo` with `maxNumerator` twice; `minLength` with foreign `appliesTo`; `maxNumerator` twice with `minDenominator` removed; `minDenominator` removed with `maxNumerator` `1.5`; `maxNumerator` `1.5` with `minNumerator` 8; `maxDenominator` `0.5` listed first with `maxNumerator` `1.5` | one refusal each, of the earlier row: `minLength`; the `appliesTo`; `minLength`; the repeated keyword; the missing keyword; the operand; `maxNumerator` |
| 12 | `level` adds its own `maxDenominator` constraint; `delta` adds its own constraint | `invalid_model_binding`/`malformed-declaration` at `Field.constraints`; the same at `Operation.params[0].constraints` |
| 13 | add alias `Level2` of `Ratio` with a constraint | `invalid_model_binding`/`malformed-declaration` at the alias node and `TypeDefinition.constraints` |
| 14 | add value type `Other` carrying a constraint whose `appliesTo` is `Ratio` | `invalid_model_binding`/`malformed-declaration` at `Other` and `Constraint.appliesTo`; `Ratio` admitted |
| 15 | add a field typed bare `ix://quire/native/Rational` and one typed row 3's empty-constraint value type, then a clause that uses each | admitted; each clause refuses `ill_typed`/`operator-ineligible`; every other declaration's node identity equals row 1's |
| 16 | `Ratio` with `unit` `ix://quire/unit/m`; add a second equal-bounds value type with unit `ix://quire/unit/m`, `Ratio` without unit, and an equal-bounds value type with unit `ix://quire/unit/cm` | `level` has type `Quantity<m, Rational[-3, 7; 1, 12]>`, the `scalar_type`/`carried_quantity` node over `m`'s unit node, row 1's `rational_range` node and `rounding` `exact`; the second `m` type names that node; the no-unit and `cm` types name two other nodes |
| 17 | `unit` `ix://quire/unit/nosuch` | `missing_declaration`/`missing-name` at the value type node and `TypeDefinition.unit` |
| 18 | `unit` `m`; `unit` `1`; `unit` `ix://quire/native/Rational`; `unit` `ix://quire/unit/m` on a value type of `scalar` `string`; `unit` `m` with `minNumerator` 8 | each of the first four `invalid_model_binding`/`malformed-declaration` at `TypeDefinition.unit`; the last only the bounds refusal |
| 19 | row 16's `Ratio`, with the units declared in the model document that declares `Ratio`, selected under the alias `Config`, and none in the source, typing fields `a` and `b`, with invariants `self.a = self.b`, `self.a < self.b`, `self.a + self.b = self.a`; then `convert<Quantity<Config::m>>(self.a) = convert<Quantity<Config::m>>(self.b)`, `convert<Quantity<Config::m>>(self.a) < convert<Quantity<Config::m>>(self.b)` and `convert<Quantity<Config::cm>>(self.a) = 100 Config::cm`; then the bare `convert<Quantity<m>>(self.a) = convert<Quantity<m>>(self.b)` | the first three each `ill_typed`/`type-mismatch` at the expression; the next three check, `Config::m` naming the document's `m` unit node; the bare form refuses `missing_declaration`/`missing-name` at the first `m` with `UnresolvedUnitName { name: "m", packages: [<the document's identity>] }` |
| 20 | a source function parameter typed `Rational[8, 7; 1, 12]`; one typed `Rational[0, 7; 0, 12]`; the types `Quantity<m, Rational[8, 7; 1, 12]>` and `Quantity<m, Rational[0, 7; 0, 12]>` | each `ill_typed` at the `Rational[..]` type |
| 24 | add tuple type `P` with position `p0` typed `Ratio` and variant type `V` with case `c` whose `payloadType` is `Ratio` | admitted; `p0` and `c`'s payload type `Rational[-3, 7; 1, 12]`, naming row 1's node |
| 25 | row 24 with `Ratio`'s `constraints` empty | `p0` `invalid_model_binding`/`malformed-declaration` at the position; `c` `unsupported_construct`/`declaration-form` at `Variant.payloadType`; both at stage `package_read`; `level` admitted with no checked type |
| 26 | row 19's model document; the compiling unit also declares its own `dimension Length;` and `unit m : Length = rational(1, 1);`; then the same without the source `m` | `Ratio`'s members typed over the source `m` unit node (`SourceOwner`), not the model document's `m` node; without the source `m`, over the model document's `m` node (`ModelOwner`) |

Value rows, as `level` in a snapshot:

| Row | Package | Value | Expected |
| --- | --- | --- | --- |
| 21 | row 1 | `7/12`, `-3/1`, `0/1`; `2/4` | the first three admitted unchanged; `2/4` admitted as its normalized value `1/2` |
| 22 | row 1 | `8/1`, `16/2`, `1/13`, `-4/1`; `float64` object of `0.5` | the first four each `invalid_runtime_input`/`invalid-value` at the member path naming `Rational[-3, 7; 1, 12]` and the normalized pair (`16/2` as `8/1`); the float `invalid_runtime_input`/`wrong-value-kind` naming `kind` |
| 23 | row 16 | `1/4`; `1/13` | `1/4 m` admitted; `1/13` refuses as row 22 |

The unit-name rows below do not admit their fixture as the rows above do;
row 27 starts from row 19's model document and row 28 from its own two
packages.

| Row | Fixture | Expected |
| --- | --- | --- |
| 27 | row 19's model document; the compiling unit declares `dimension Time;` and `unit m : Time = rational(1, 1);`; the invariant `convert<Quantity<Config::cm>>(self.a) = 100 Config::cm`, first without those source declarations | without them the invariant checks; with them the package is admitted with no refusal, `Ratio`'s members are typed over the source `m` of `Time`, and the invariant refuses `ill_typed`/`type-mismatch` at the conversion |
| 28 | packages `acme/a` (alias `A`) and `acme/b` (alias `B`), selected in that order, each declaring a base dimension and a unit `x`; the compiling unit declares no `x`; `function f using v(d: Quantity<U>): Boolean pure { d = d }` for `U` = `x`, `A::x`, `B::x`; the literal `1 x`; `convert<Quantity<x>>(1 A::x)`; then `acme/b` declaring `y` in place of `x`, for `U` = `x` and `nosuch`; then the first packages with the compiling unit declaring `dimension L;` and `unit x : L = rational(1, 1);`, for `U` = `x`; then the first packages with `acme/a` declaring a value type typed `unit` `ix://quire/unit/x` | `x`, `1 x` and the `convert` target: one refusal each, `missing_declaration`/`missing-name` at `x`, cause `UnresolvedUnitName { name: "x", packages: ["acme/a", "acme/b"] }`, no `PackageDeclarations`; `A::x` and `B::x` assemble; with `y`, `x` refuses with `packages: ["acme/a"]` and `nosuch` with `packages: []`; with the source `x`, `x` resolves to the source unit with no refusal; the value type's members are typed over `acme/a`'s `x` with no refusal |

Tag the test `#[trace("FR-505-AC-1", "FR-505-AC-2", "FR-505-AC-3", "FR-505-AC-4", "FR-505-AC-5", "FR-505-AC-6", "FR-505-AC-7", "FR-505-AC-8", "FR-505-AC-9", "FR-505-AC-10", "FR-505-AC-11", "FR-505-AC-12", "TC-1300")]`.

## Expected Results

Each row's Expected column. Rows 5-14, 17-18 and 25 report exactly one refusal of
the value type (or of the carrying node, rows 12-14), at the named path; rows
21-23 round or convert no value, and row 21 retains `2/4` only as `1/2`.
