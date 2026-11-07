---
id: FR-505
title: "Admit bounded Rational value types and refuse inconsistent bounds"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-041
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-497
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-154
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-142
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-149
    type: depends_on
---
# FR-505: Admit bounded Rational value types and refuse inconsistent bounds

## Description

Native-model admission (FR-041-AC-2, deleted with `src/native_model`)
admitted a rational scalar only when every declared site of the scalar carried
the same numerator minimum, numerator maximum and maximum denominator, and it
refused a missing, duplicated, empty or inconsistent site. The spine admits a
domain-package field, parameter or result typed `ix://quire/native/Rational`
with no checked type, because no `typeRef` and no semantic-IR constraint
carries the bounds (FR-056, "A field, operation parameter or operation result
whose `typeRef` is ..."). The spine therefore admits no bounded exact rational
scalar, and the bounds-consistency refusal has no successor. This requirement
is that successor. It gives the model document a **Rational value type**, the
counterpart of FR-056's Text value type, so that every site typed by one value
type shares one bound by construction and a disagreement is a refusal. QSpec
FR-154 "Rational value types", FR-322 "Model-owned members" step 4 and FR-142
"Carried quantity" own the contract; this requirement states how QSL's intake
reader, checker and lowering realize it and invents none.

## Inputs

- A model document value-type definition (meaning
  `quire.meaning.model.value-type/v1`) whose `scalar` is `rational` and whose
  `constraints` either is empty or holds `minNumerator`, `maxNumerator`,
  `minDenominator` and `maxDenominator`, each once, in any order. Each
  constraint's `appliesTo` is the value type itself and its `operands` is an
  object whose only member `value` is either a JSON integer of magnitude at
  most `2^53` (`9007199254740992`), or a canonical signed-64 decimal integer
  string in `i64::MIN..=i64::MAX`: `0`, a positive digit sequence without a
  leading zero, or `-` followed by a nonzero digit and further digits.
- Optionally a `unit` member on the value type, a unit reference
  `ix://quire/unit/<name>` of the form a unit `typeRef` takes (QSpec FR-154):
  the legacy rational scalar carried an authored unit (dimensionless or one
  named unit), which this requirement keeps. A unit name resolves against
  the package's unit table, which holds the compiling unit's units and the
  model document's own units: when the compiling unit and the model document
  each declare a unit of one name, the compiling unit's unit of that name
  takes the name, whatever its dimension or scale, and the model document
  fills names the source lacks, as TC-419 step 8's source-owned unit nodes
  show: `ix://quire/unit/<name>` names the unit namespace, not the
  document's own unit, so a source unit `m` of `Time` types a document
  value type's `unit` `ix://quire/unit/m` over that `Time` unit and is not
  refused. Intake checks the value type's `unit`, and the checker resolves
  the `m` of `convert<Quantity<m>>` over its members, against that table,
  so a model document's unit is reached by a bare name only where the
  source declares no unit of that name. The alias-qualified spelling
  `Config::m`, `Config` being the alias that selects the document, names
  the document's own unit `m` (FR-091 "Dimension and unit declarations": `M::name` names a unit of the selected package) and resolves
  whether or not the source declares an `m`.
- A bare unit name in source that the compiling unit does not declare
  (a `Quantity<x>` type form, a quantity literal's unit or a
  `convert<Quantity<x>>` target) resolves to a selected domain package's
  unit of that name only when exactly one selected package declares a unit
  of that name. When two or more selected packages each declare one, the
  assembler SHALL refuse the name `ambiguous_declaration`/`ambiguous-name`
  (QSpec FR-272) at the name, through the assembly cause
  `AmbiguousUnitName { name, packages }`, whose `packages` are the
  identities of every declaring package in the order the unit's `model`
  declarations select them, and SHALL resolve no unit for it; it never
  refuses the name as `UnresolvedTypeName`, the cause of a name no unit
  holds, and never picks one package. The alias-qualified spellings `A::x`
  and `B::x` resolve to each package's unit.
- Fields, parameters, results, aliases, option `inner`, sequence `items`, map
  `values`, tuple positions and variant payloads whose `typeRef`, `target` or
  member names that value type.

## Outputs

- A member typed by a bounded Rational value type with no `unit` has the type
  `Rational[minNumerator, maxNumerator; minDenominator, maxDenominator]`, the
  form QSL source writes (QSpec FR-149 "Bounded rationals"), and lowers to the
  structural `bounded_domain`/`rational_range` node over the
  `scalar_type`/`rational` node, with bindings `numerator_min`,
  `numerator_max`, `denominator_min` and `denominator_max` (QSpec FR-322).
  The legacy `maximum_denominator` of `N` is the constraints `minDenominator` 1
  and `maxDenominator` `N`.
- A member typed by a bounded Rational value type with `unit` `u` has the
  carried quantity type `Quantity<u, Rational[n1, n2; d1, d2]>` (FR-497, QSpec
  FR-142 "Carried quantity") and lowers to FR-497's
  `scalar_type`/`carried_quantity` node over `u`'s unit node and that
  `rational_range` node, with `rounding` `exact`.
- A Rational value type with no constraints is the unbounded `Rational`: a
  member of it has no checked type, as a member typed by the bare native
  `ix://quire/native/Rational` has none.
- A refusal at the value type, naming the first defect (below).

## Behavior

- The intake reader SHALL bind a value type as Rational by its `scalar`,
  never by a `target` alias.
- The intake reader SHALL read a Rational value type's bounds from its own
  `constraints` only.
- The intake reader SHALL admit a Rational value type whose `constraints` is
  empty as the unbounded `Rational`, not refuse it.
- When a Rational value type has a non-empty `constraints`, the Rational
  reader SHALL refuse its first defect in this order, each at the value type node,
  so that exactly one refusal is reported. Rows 1 to 3 scan `constraints` in
  array order; rows 4 and 5 take the keywords in the order `minNumerator`,
  `maxNumerator`, `minDenominator`, `maxDenominator`:
  1. a constraint whose keyword is none of the four:
     `unsupported_construct`/`declaration-form` at `Constraint.keyword`,
     naming the keyword;
  2. a constraint whose `appliesTo` is not the value type's own identity
     (another node, an alias of the value type, or a native or unit
     reference): `invalid_model_binding`/`malformed-declaration` at
     `Constraint.appliesTo`;
  3. a keyword given a second time, the later constraint in array order:
     `invalid_model_binding`/`malformed-declaration` at `Constraint.keyword`,
     naming the keyword;
  4. a missing keyword: `invalid_model_binding`/`malformed-declaration` at
     `TypeDefinition.constraints`, naming the keyword;
  5. an `operands` that is not an object whose only member is `value`, or
     whose `value` is absent or is neither a JSON integer in
     `[-2^53, 2^53]` nor a canonical signed-64 decimal integer string
     (a fraction, an exponent spelling, `"3/4"`, `"01"`, `"+1"`, `"-0"`,
     `"9223372036854775808"` or `"-9223372036854775809"`):
     `invalid_model_binding`/`malformed-declaration` at
     that constraint's `Constraint.operands`, naming the keyword; no operand is
     rounded, widened or repaired;
  6. bounds outside `minNumerator <= maxNumerator` and
     `1 <= minDenominator <= maxDenominator`:
     `unsupported_construct`/`declaration-form` at
     `TypeDefinition.constraints`, naming the four bounds.
- A denominator bound therefore lies in `1 ..= i64::MAX` by row 6: a zero or
  negative `minDenominator` or `maxDenominator`, as an accepted integer or
  canonical string, passes row 5 and refuses
  `unsupported_construct`/`declaration-form` at row 6.
- Before declaration intake, the package's FR-272 canonical-number check
  SHALL refuse an integer number of magnitude above `2^53` as
  `noncanonical_wire`/`inexact-integer` at its `document_pointer`. Such a
  number never reaches row 5. The limit is inclusive and is not the narrower
  checked-node ordinal bound; strings preserve the full signed-64 range.
- The intake reader SHALL refuse a constraint carried by any other node whose
  `appliesTo` names a Rational value type, or an alias whose `target` chain
  reaches one, `invalid_model_binding`/`malformed-declaration` at the carrying
  node and its
  `Constraint.appliesTo`, whatever its keyword; this refusal is the carrying
  node's and leaves the value type's own report unchanged.
- The intake reader SHALL keep the value type's `unit`: absent means
  dimensionless. A `unit` that is not a string `ix://quire/unit/<name>` (a
  bare name such as `m`, a number, or a native reference such as
  `ix://quire/native/Rational`), and a `unit` on a value type whose `scalar`
  is not `rational`, refuse `invalid_model_binding`/`malformed-declaration` at
  `TypeDefinition.unit`; a well-formed `unit` naming no unit of the unit
  namespace refuses `missing_declaration`/`missing-name` at the value type
  node and `TypeDefinition.unit`. A `unit` defect is reported only when no
  constraint row above fails.
- The checker SHALL check a member of a unit-bearing Rational value type as
  the carried quantity `Quantity<u, Rational[..]>` of FR-497, whose mode is
  `exact`: it is not an operand of `=`, `!=`, `<` or any other quantity
  operation, which refuse `ill_typed`/`type-mismatch` at the expression as
  FR-497 "Type identity" states. Two such members are compared, and combined,
  through the exact conversion `convert<Quantity<u>>(e)` of each. Two Rational
  value types with equal bounds and different units, or one with a unit and
  one without, are two types; two with equal bounds and equal units, or equal
  bounds and no unit, are one type and lower to one node (the legacy rule that
  equal representations cannot merge different nominal units).
- The checker SHALL refuse a source `Rational[n1, n2; d1, d2]` with
  `n1 > n2` or outside `1 <= d1 <= d2` `ill_typed` at the type wherever it is
  written, the carrier of a `Quantity<u, C>` included (QSpec FR-149 "Bounded
  rationals"); this is the source-form counterpart of row 6.
- Every member typed by one Rational value type, directly or through an alias
  chain, SHALL have exactly that type's bounds and unit. A member cannot
  restate or narrow them: a field typed by it that carries its own non-empty
  `constraints` refuses `invalid_model_binding`/`malformed-declaration` at
  `Field.constraints`, a parameter the same at
  `Operation.params[i].constraints`, and an alias whose `target` chain reaches
  it and that carries a non-empty `constraints` refuses the same at the alias
  node and `TypeDefinition.constraints`. An alias with no constraint admits
  and denotes its target's type.
- FR-106 admission SHALL read a snapshot or invocation value at a member of
  a bounded Rational value type as FR-106 "Rational value" states: it reduces
  the supplied Rational object to its normalized form (greatest common divisor
  1, positive denominator) and admits it only if that normalized numerator and
  denominator lie inside the type's bounds, so an unreduced fraction admits as
  its normalized value. A normalized pair outside the bounds refuses
  `invalid_runtime_input`/`invalid-value` at the member path naming the
  declared type and the normalized pair (FR-106 check 7), a Float object
  refuses `invalid_runtime_input`/`wrong-value-kind` naming `kind` (FR-106
  check 6), and no value is
  rounded or converted. A value at a member of a unit-bearing Rational value
  type is its magnitude in `u`, decided by the same rule.
- A field, operation parameter or operation result typed by the bare native
  `ix://quire/native/Rational`, or by a Rational value type with no
  constraints, keeps FR-056's behavior (admitted with no checked type, every
  use refuses `ill_typed`/`operator-ineligible`).
- The package reader SHALL admit a bounded Rational value type as a tuple
  position `typeRef` and as a variant case `payloadType`, with the type and
  `rational_range` node a field of it has. It SHALL refuse a Rational value
  type with no constraints there as it refuses the native `Rational` there
  (QSpec FR-322-AC-181 and AC-182), at stage `package_read`: as a tuple
  position `invalid_model_binding`/`malformed-declaration` at the position,
  and as a variant payload `unsupported_construct`/`declaration-form` at
  `Variant.payloadType`. Intake's reference-form check admits both; the
  position restriction is the checked package's (QSpec FR-322).
  The legacy refusal of a whole model for a rational site without a role is
  not carried: an unbounded rational site is exactly the bare native, and
  FR-056-AC-43 owns it.
- Adding the Rational value type SHALL change no other declaration's node
  identity, ordering or binding.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-505-AC-1 | A package with value type `Ratio` (`scalar` `rational`; constraints `maxDenominator` 12, `minNumerator` -3, `minDenominator` 1 and `maxNumerator` 7, in that order) and a field, an operation parameter and an operation result typed `Ratio` admits; each of the three sites and a plain alias of `Ratio` has the type `Rational[-3, 7; 1, 12]`, the package keeps one original declaration for `Ratio`, and the four sites name one `bounded_domain`/`rational_range` node over the `scalar_type`/`rational` node with bindings `numerator_min` -3, `numerator_max` 7, `denominator_min` 1 and `denominator_max` 12. A second value type of equal bounds names that same node and one with `maxDenominator` 13 names another. `Ratio` with an empty `constraints` admits as the unbounded `Rational`, and a field of it has no checked type; JSON integer bounds `-9007199254740992` and `9007199254740992`, and canonical string bounds `"-9223372036854775808"` and `"9223372036854775807"`, admit with unchanged exact bounds. | Test (TC-1300) |
| FR-505-AC-2 | Over `Ratio`, each single defect refuses once at the `Ratio` node: a constraint `minLength` `unsupported_construct`/`declaration-form` at `Constraint.keyword` naming `minLength`; a `maxNumerator` whose `appliesTo` is another value type, or an alias of `Ratio`, `invalid_model_binding`/`malformed-declaration` at `Constraint.appliesTo`; `maxNumerator` given twice the same at the second `Constraint.keyword` naming `maxNumerator`; `minDenominator` removed the same at `TypeDefinition.constraints` naming `minDenominator`; `minNumerator` 8 against `maxNumerator` 7, `minDenominator` 0, `minDenominator` 13 against `maxDenominator` 12, `maxDenominator` 0 and `maxDenominator` -1 each `unsupported_construct`/`declaration-form` at `TypeDefinition.constraints` naming the bounds. | Test (TC-1300) |
| FR-505-AC-3 | Operands `1.5`, `1e2`, `"3/4"`, `"01"`, `"+1"`, `"-0"`, `"9223372036854775808"` and `"-9223372036854775809"` for `maxNumerator`, `0.5` for `maxDenominator`, a `maxNumerator` with no `operands.value`, and a `maxNumerator` whose `operands` carries a second member, each refuse `invalid_model_binding`/`malformed-declaration` at that constraint's `Constraint.operands` naming the keyword, with no bound rounded or widened. Integer numbers `9007199254740993`, `9007199254740994` and `-9007199254740994` refuse earlier as `noncanonical_wire`/`inexact-integer` at their exact document pointer, not row 5. | Test (TC-1300) |
| FR-505-AC-4 | A field of type `Ratio` that adds its own `maxDenominator` constraint refuses `invalid_model_binding`/`malformed-declaration` at `Field.constraints`; an operation parameter typed `Ratio` with its own constraint refuses the same at `Operation.params[0].constraints`; an alias of `Ratio` that carries a constraint refuses the same at the alias node and `TypeDefinition.constraints`; a constraint carried by another value type whose `appliesTo` is `Ratio` refuses the same at that value type's node and `Constraint.appliesTo`, while `Ratio` itself admits. Two sites typed by two value types that differ only in `maxDenominator` (12 and 13) stay two distinct types. | Test (TC-1300) |
| FR-505-AC-5 | A snapshot supplying `7/12`, `-3/1` and `0/1` to a `Ratio` field is admitted unchanged, and `2/4` (unreduced) is admitted as its normalized value `1/2` (FR-106-AC-55); `8/1`, `16/2` (normalized `8/1`), `1/13` and `-4/1` each refuse `invalid_runtime_input`/`invalid-value` at the member path naming `Rational[-3, 7; 1, 12]` and the normalized pair, the `float64` object of `0.5` refuses `invalid_runtime_input`/`wrong-value-kind` naming `kind` (FR-106-AC-56), and no value is rounded. With `Ratio` given `unit` `ix://quire/unit/m`, `1/4` is admitted as `1/4 m` and `1/13` refuses the same. | Test (TC-1300) |
| FR-505-AC-6 | A field typed bare `ix://quire/native/Rational`, and a field typed a Rational value type with no constraints, each admit and their use in a clause refuses `ill_typed`/`operator-ineligible` as FR-056-AC-43 states; adding `Ratio` to the package leaves every other declaration's node identity unchanged. | Test (TC-1300) |
| FR-505-AC-7 | With `dimension Length;`, `unit m : Length = rational(1, 1);` and `unit cm : Length = rational(1, 100);`, `Ratio` with `unit` `ix://quire/unit/m` types its members `Quantity<m, Rational[-3, 7; 1, 12]>`, lowered to the `scalar_type`/`carried_quantity` node over `m`'s unit node, AC-1's `rational_range` node and `rounding` `exact`; a second value type of equal bounds and unit `ix://quire/unit/m` names that same node, while `Ratio` with no unit and a value type of equal bounds with unit `ix://quire/unit/cm` name two other nodes. `unit` `ix://quire/unit/nosuch` refuses `missing_declaration`/`missing-name` at the value type node and `TypeDefinition.unit`; `unit` `m`, `unit` `1` and `unit` `ix://quire/native/Rational` on `Ratio`, and `unit` `ix://quire/unit/m` on a value type of `scalar` `string`, each refuse `invalid_model_binding`/`malformed-declaration` at `TypeDefinition.unit`; `Ratio` with `unit` `m` and `minNumerator` 8 reports only the bounds refusal. | Test (TC-1300) |
| FR-505-AC-8 | When two defects of `Ratio` are present only the earlier row is reported: `minLength` with `maxNumerator` given twice reports `minLength`; a foreign `appliesTo` with `maxNumerator` given twice reports the `appliesTo`; `minLength` with a foreign `appliesTo` reports `minLength`; `maxNumerator` given twice with `minDenominator` removed reports the repeated keyword; `minDenominator` removed with `maxNumerator` `1.5` reports the missing keyword; `maxNumerator` `1.5` with `minNumerator` 8 reports the operand; `maxDenominator` `0.5` (listed first) with `maxNumerator` `1.5` reports `maxNumerator`, in keyword order. | Test (TC-1300) |
| FR-505-AC-9 | Over AC-7's units declared in `Ratio`'s model document, selected under the alias `Config`, so that the bare `m` resolves against that document's own units, two fields `a` and `b` typed by `Ratio` with `unit` `ix://quire/unit/m`: the invariants `self.a = self.b`, `self.a < self.b` and `self.a + self.b = self.a` each refuse `ill_typed`/`type-mismatch` at the expression, while `convert<Quantity<m>>(self.a) = convert<Quantity<m>>(self.b)`, `convert<Quantity<m>>(self.a) < convert<Quantity<m>>(self.b)` and the alias-qualified `convert<Quantity<Config::m>>(self.a) = convert<Quantity<m>>(self.b)` check, `Config::m` and `m` naming the document's one `m` unit node. The source types `Rational[8, 7; 1, 12]` and `Rational[0, 7; 0, 12]` refuse `ill_typed` at the type as a function parameter type and as the carrier of `Quantity<m, ..>`. | Test (TC-1300) |
| FR-505-AC-10 | A tuple type `P` with position `p0` typed `Ratio` and a variant type `V` with case `c` whose `payloadType` is `Ratio` admit, and `p0` and `c`'s payload have the type `Rational[-3, 7; 1, 12]` and name AC-1's `rational_range` node; with `Ratio`'s `constraints` empty, `p0` refuses `invalid_model_binding`/`malformed-declaration` at the position and `c` refuses `unsupported_construct`/`declaration-form` at `Variant.payloadType`, each at stage `package_read`, while a field typed the unconstrained `Ratio` admits with no checked type. | Test (TC-1300) |
| FR-505-AC-11 | Over AC-9's model document (its own `dimension Length`, `m` and `cm`), with the compiling unit also declaring its own `dimension Length;` and `unit m : Length = rational(1, 1);`, `Ratio` with `unit` `ix://quire/unit/m` types its members over the compiling unit's `m` unit node, the one whose owner is a `SourceOwner`, not the model document's `m` node; with the source `m` removed the members are typed over the model document's `m` node, whose owner is a `ModelOwner`. With the source declaring `dimension Time;` and `unit m : Time = rational(1, 1);` in place of its `Length` and `m`, the package is admitted with no refusal, the members are typed over the source `m` of `Time`, and `convert<Quantity<cm>>(self.a) = 100 cm`, which checks without the source `m`, refuses `ill_typed`/`type-mismatch` at the conversion. | Test (TC-1300) |
| FR-505-AC-12 | With selected packages `acme/a` under alias `A` and `acme/b` under alias `B`, in that order, each declaring a base dimension and a unit `x`, and a compiling unit that declares no `x`: `function f using v(d: Quantity<x>): Boolean pure { d = d }` refuses once, `ambiguous_declaration`/`ambiguous-name` at `x`, with the cause `AmbiguousUnitName { name: "x", packages: ["acme/a", "acme/b"] }` and no `UnresolvedTypeName`, and no `PackageDeclarations`; the same function over `Quantity<A::x>` and over `Quantity<B::x>` assembles; with `acme/b` declaring `y` in place of `x`, `Quantity<x>` resolves to `acme/a`'s `x` and `Quantity<y>` to `acme/b`'s `y`; and with the compiling unit declaring its own `dimension L;` and `unit x : L = rational(1, 1);`, `Quantity<x>` resolves to the source `x` with no refusal. | Test (TC-1300) |

## Dependencies

- **Unit carrier:** a unit-bearing Rational value type is the carried quantity
  `Quantity<u, Rational[..]>` of FR-497, which admits a `Rational[..]` carrier
  with mode `exact` (QSpec FR-142 "Carried quantity"). Its members are
  stored, passed, returned and converted as FR-497 states; equality, ordering
  and arithmetic go through `convert<Quantity<u>>` and refuse
  `ill_typed`/`type-mismatch` on the carried members directly. A member of a
  Rational value type with no unit is an ordinary `Rational[..]` operand.
- **Upstream:** FR-056 (Text value type, intake order), FR-106 (value
  admission), FR-497 (carried quantities), FR-041-AC-2 (the legacy behavior
  this succeeds).
- **Contract (QSpec):** QSpec FR-154 "Rational value types" (FR-154-AC-42 to
  AC-47), FR-322 "Model-owned members" step 4 and FR-322-AC-380 to AC-382,
  FR-142 "Carried quantity" (FR-142-AC-31) and FR-149 "Bounded rationals"
  define the value type, its reader order, its node and its carrier; vectors
  TC-981 `RV-01` to `RV-12`. This requirement cites them by id.
