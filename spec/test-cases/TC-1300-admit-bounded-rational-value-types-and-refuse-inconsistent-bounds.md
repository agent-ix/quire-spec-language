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

Verify the Rational value type of FR-505: shared bounds across sites, the
ordered reader refusals, alias and site restatement, value admission and the
unchanged bare native. Scope: FR-505-AC-1 to FR-505-AC-7.

## Test Procedure

Fixture `R0` is a domain package with object type `Gauge` whose field `level`,
operation `adjust` parameter `delta` and result are typed `Ratio`, where
`Ratio` has `minNumerator` -3, `maxNumerator` 7, `minDenominator` 1,
`maxDenominator` 12. Each row below mutates `R0` as stated and admits it.

| Row | Mutation of R0 | Expected |
| --- | --- | --- |
| 1 | none | admitted; `level`, `delta` and the result have type `Rational[-3, 7; 1, 12]`; one `Ratio` type node, three references |
| 2 | add constraint `minLength` 0 to `Ratio` | `unsupported_construct`/`declaration-form` naming `minLength` |
| 3 | `maxNumerator` given twice | `invalid_model_binding`/`malformed-declaration` naming `maxNumerator` |
| 4 | remove `minDenominator` | `invalid_model_binding`/`malformed-declaration` naming `minDenominator` |
| 5 | `minNumerator` 8 | `unsupported_construct`/`declaration-form` naming the bounds |
| 6 | `minDenominator` 0 | `unsupported_construct`/`declaration-form` naming the bounds |
| 7 | `minDenominator` 13 | `unsupported_construct`/`declaration-form` naming the bounds |
| 8 | `minLength` plus `minNumerator` 8 | the row 2 refusal only |
| 9 | `maxNumerator` operand `1.5` | `invalid_model_binding`/`malformed-declaration` at `Ratio` |
| 10 | `maxNumerator` operand `1e2` | same as row 9 |
| 11 | `maxNumerator` operand 9223372036854775808 | same as row 9 |
| 12 | `maxDenominator` operand `0.5` | same as row 9 |
| 13 | `level` adds its own `maxDenominator` constraint | `invalid_model_binding`/`malformed-declaration` at `Field.constraints` |
| 14 | add alias `Level2` of `Ratio` with a constraint | `invalid_model_binding`/`malformed-declaration` at the alias |
| 15 | add plain alias `Level3` of `Ratio` typing a new field | admitted; type `Rational[-3, 7; 1, 12]` |
| 16 | add `Ratio13` equal to `Ratio` but `maxDenominator` 13, typing a new field | admitted; two distinct types |
| 17 | add a field typed bare `ix://quire/native/Rational`, then a clause that uses it | admitted; the clause refuses `ill_typed`/`operator-ineligible`; every other declaration's node identity equals row 1's |

Value rows, over row 1's package, as `level` in a snapshot:

| Row | Value | Expected |
| --- | --- | --- |
| 18 | `7/12`, `-3/1`, `0/1` | admitted |
| 20 | `Ratio` with `unit` `m` beside `Ratio` without; `unit` `nosuch`; two equal-bounds equal-unit types | two distinct types, field has declared unit `m`; `missing_declaration`/`missing-name` at `unit`; one type |
| 19 | `8/1`, `1/13`, `2/4`, `-4/1`, float `0.5` | each `invalid_runtime_input`/`invalid-value` naming the member path |

Tag the test `#[trace("FR-505-AC-1", "FR-505-AC-2", "FR-505-AC-3", "FR-505-AC-4", "FR-505-AC-5", "FR-505-AC-6", "FR-505-AC-7", "TC-1300")]`.

## Expected Results

Each row's Expected column. Rows 2-12 and 13-14 report exactly one refusal, at
the named node; rows 18-19 change no value.
