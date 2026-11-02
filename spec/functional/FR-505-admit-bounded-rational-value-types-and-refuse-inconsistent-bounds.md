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
  - target: ix://agent-ix/quire-specification/FR-154
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
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
type shares one bound by construction and a disagreement is a refusal.

## Inputs

- A model document value-type definition (meaning
  `quire.meaning.model.value-type/v1`) whose `scalar` is `rational` and whose
  `constraints` hold `minNumerator`, `maxNumerator`, `minDenominator` and
  `maxDenominator`, each once, in any order. Each constraint's `appliesTo` is
  the value type itself and its `operands.value` is a JSON integer.
- Optionally a `unit` member on the value type, as FR-154 reads it for a
  quantity: the legacy rational scalar carried an authored unit (dimensionless
  or one named unit), which this requirement keeps.
- Fields, parameters, results, aliases and option or sequence items whose
  `typeRef` or `target` names that value type.

## Outputs

- A field, parameter or result typed by the value type has the value type
  `Rational[minNumerator, maxNumerator; minDenominator, maxDenominator]`, the
  form QSL source writes. The legacy `maximum_denominator` of `N` is the
  constraints `minDenominator` 1 and `maxDenominator` `N`.
- A refusal at the value type, naming the first defect (below).

## Behavior

- The intake reader SHALL bind a value type as Rational by its `scalar` and
  its four constraints, never by a `target` alias. A `target` alias of a
  Rational value type carries no constraint; an alias that carries one refuses
  `invalid_model_binding`/`malformed-declaration` at the alias.
- The Rational reader SHALL refuse the first defect in this order, each at the
  value type naming what the row names:
  1. a constraint whose keyword is none of the four:
     `unsupported_construct`/`declaration-form`, naming the keyword;
  2. a keyword given twice, in constraint order:
     `invalid_model_binding`/`malformed-declaration`, naming the keyword;
  3. a missing keyword, checked in the order `minNumerator`, `maxNumerator`,
     `minDenominator`, `maxDenominator`:
     `invalid_model_binding`/`malformed-declaration`, naming the keyword;
  4. bounds outside `minNumerator <= maxNumerator` and
     `1 <= minDenominator <= maxDenominator`:
     `unsupported_construct`/`declaration-form`, naming the bounds.
- The Rational reader SHALL require signed 64-bit numerator bounds and
  denominator bounds in `1 ..= i64::MAX`; an operand outside its range, a
  float, an exponent spelling or a fraction spelling refuses at the reader of
  item 1's row as `invalid_model_binding`/`malformed-declaration` and is never
  rounded, widened or repaired (FR-041 "Behavior", inherited).
- The intake reader SHALL keep the value type's `unit`: absent means
  dimensionless, and a named unit that is not a unit of the unit namespace
  refuses `missing_declaration`/`missing-name` at the `unit` member. The
  checked type of a site is `Rational[...]` with that declared unit, and two
  value types equal in bounds and different in unit are two different types
  (the legacy rule that equal representations cannot merge different nominal
  units). Unit arithmetic and conversion over a Rational carrier are FR-497's;
  see Dependencies.
- Every site typed by one Rational value type SHALL have exactly that type's
  bounds. A site cannot restate or narrow them: a field of that type that
  carries its own `constraints` refuses `invalid_model_binding`/
  `malformed-declaration` at `Field.constraints` (FR-154's path).
- FR-106 admission SHALL admit a snapshot or invocation value at a Rational
  site only if it is a reduced fraction inside the type's domain; an unreduced
  fraction, a numerator or denominator outside the bounds or a float refuses
  `invalid_runtime_input`/`invalid-value` naming the member path (FR-106).
- A site typed by the bare native `ix://quire/native/Rational` keeps FR-056's
  behavior (admitted with no checked type, every use refuses
  `ill_typed`/`operator-ineligible`). The legacy refusal of a whole model for
  a rational site without a role is not carried: an unbounded rational site is
  exactly the bare native, and FR-056-AC-43 owns it.
- Adding the Rational value type SHALL change no other declaration's node
  identity, ordering or binding.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-505-AC-1 | A package with value type `Ratio` (constraints `minNumerator` -3, `maxNumerator` 7, `minDenominator` 1, `maxDenominator` 12) and a field, an operation parameter and an operation result typed `Ratio` admits, and each of the three sites has the type `Rational[-3, 7; 1, 12]`; the checked package carries one `Ratio` type node and the three sites reference it. | Test (TC-1300) |
| FR-505-AC-2 | Over `Ratio`, a constraint `minLength` refuses `unsupported_construct`/`declaration-form` naming `minLength`; `maxNumerator` given twice refuses `invalid_model_binding`/`malformed-declaration` naming `maxNumerator`; with `minDenominator` removed the refusal is `invalid_model_binding`/`malformed-declaration` naming `minDenominator`; `minNumerator` 8 against `maxNumerator` 7, `minDenominator` 0 and `minDenominator` 13 against `maxDenominator` 12 each refuse `unsupported_construct`/`declaration-form` naming the bounds. When two defects are present the earlier row of the order above is the one reported. | Test (TC-1300) |
| FR-505-AC-3 | Operands `1.5`, `1e2`, `"3/4"` and 9223372036854775808 for `maxNumerator`, and `0.5` for `maxDenominator`, refuse `invalid_model_binding`/`malformed-declaration` at the value type, with no bound rounded or widened. | Test (TC-1300) |
| FR-505-AC-4 | A field of type `Ratio` that adds its own `maxDenominator` constraint, and an alias of `Ratio` that carries a constraint, each refuse `invalid_model_binding`/`malformed-declaration` at the field's `Field.constraints` and at the alias; a plain alias of `Ratio` admits and has the type `Rational[-3, 7; 1, 12]`. Two sites typed by two value types that differ only in `maxDenominator` (12 and 13) stay two distinct types. | Test (TC-1300) |
| FR-505-AC-5 | A snapshot supplying `7/12`, `-3/1` and `0/1` to a `Ratio` field is admitted; `8/1`, `1/13`, `2/4` (unreduced), `-4/1` and the float `0.5` each refuse `invalid_runtime_input`/`invalid-value` naming the member path, and no value is reduced or rounded. | Test (TC-1300) |
| FR-505-AC-6 | A field typed bare `ix://quire/native/Rational` admits and its use in a clause refuses `ill_typed`/`operator-ineligible` as FR-056-AC-43 states; adding `Ratio` to the package leaves every other declaration's node identity unchanged. | Test (TC-1300) |
| FR-505-AC-7 | `Ratio` with `unit` `m` (metre) and `Ratio` with no unit give two distinct types for otherwise equal bounds; a field typed by the metre one has declared unit `m`; `unit` `nosuch` refuses `missing_declaration`/`missing-name` at the `unit` member; two value types with equal bounds and equal units are the same checked type. | Test (TC-1300) |

## Dependencies

- **Unit carrier:** FR-497 admits only `Int` and `Decimal` carriers in `Quantity<u, C>`; arithmetic and conversion of a Rational with a unit needs FR-497 (and QSpec FR-142) to admit a Rational carrier. Until then a unit-bearing Rational type is declared and compared, and any arithmetic on it refuses `ill_typed`/`operator-ineligible` as for a bare native Rational.

- **Upstream:** FR-056 (Text value type, intake order), FR-106 (value
  admission), FR-041-AC-2 (the legacy behavior this succeeds).
- **Contract change required (QSpec, not QSL):** QSpec FR-154 and the
  semantic-IR validator must admit a value-type definition with `scalar`
  `rational` and the four constraint keywords above, as it admits `string`
  with `minLength`, `maxLength` and `format` today (otherwise the validator
  refuses `CONSTRAINT_NOT_APPLICABLE`), and QSpec FR-322 "Model-owned members"
  step 4's declared-type table must name the Rational value type as a
  `Rational[..]` type. Until it does, the criteria cannot pass; this is
  the QSpec question filed with this batch.
