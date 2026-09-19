---
id: TC-194
title: "Complete equality matrix"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/FR-149
    type: verifies
---

# TC-194: Complete equality matrix

## Description

Execute positive, negative and cross-type comparisons for every complete-V1 value kind.

## Test Procedure

Select language `ix:native`, edition `1-draft` revision `1-draft.2`, and the
`quire.value.complete/v1`, `quire.value.text.unicode-17.0.0/v1` and
`quire.value.ieee754-2019-default/v1` definitions at revision `1-draft.1`, using
their exact DefinitionRefs from
[`complete-value-lock.json`](../../proposals/quire-v1/definitions/complete-value-lock.json).
Build fixtures with distinct I04 semantic-node keys
for declarations that intentionally share names or shapes. Execute every row
through the real local Rust boundary:

Every `ill-typed` expectation below means a type-checking
`refused { code: ill_typed }`; it is not a peer evaluator outcome.

Every equality is spelled `=` or `!=`. Each operand is a parameter of the
named declared type, or a scalar literal whose type the other operand supplies
under FR-144 literal typing.

| Vector | Operands and selected relation | Expected result/disposition |
| --- | --- | --- |
| E01 | Boolean `true,true`; `true,false` | true; false |
| E02 | Integer pairs `(1,1)` and `(1,2)`; then `Int[0,2]` value `1` and `Rational[0,2;1,1]` value `1/1` with `convert<Rational[0,2;1,1]>` on the left operand, and without conversion | true; false; true; ill-typed |
| E03 | Decimal pairs `(10,1)`/`(100,2)` and `(10,1)`/`(11,1)`; then `Decimal[0,100;0,2]` value `(10,1)` and `Rational[0,100;1,100]` value `1/1` with `convert<Rational[0,100;1,100]>` on the left operand | true; false; true with both source types/provenance unchanged |
| E04 | `Rational[0,1;1,3]` value `1/3` under `convert<Decimal[0,100;2,2;nearest-even]>` compared with decimal `(33,2)` | ill-typed, not false, because the source denominator bound exceeds one |
| E05 | `100 cm` and `1 m` after explicit canonical-unit conversion; then `1 m` and `1 s` | true; incompatible dimensions are ill-typed |
| E06 | U+00E9 and `U+0065 U+0301` under `nfc`; `a` and `b` under `nfc`; same payload typed as `nfc` and `binary-utf8` | true; false; different profiles are ill-typed |
| E07 | `enum-A::READY` with itself and with `enum-A::DONE`, then with `enum-B::READY` | true; false; different declaration node is ill-typed without an explicit common type |
| E08 | parameters of the same `Option<Integer>`: two `none` values; two present values holding 1; `none` versus a present value holding 1 (the grammar has no present constructor, so present values are supplied as parameters) | true; true; false |
| E09 | same declared nullable field containing `null`; then its absent state versus explicit `null` | true; false, with no absence/null coercion |
| E10 | records of declaration `record-A` with equal fields, one unequal nested field, then equal-shaped `record-B` | true; false; different declaration is ill-typed |
| E11 | tuples of declaration `tuple-A` with equal positions, one unequal position, then equal-shaped `tuple-B` | true; false; different declaration is ill-typed |
| E12 | sequences `[1,2]`, `[1,2]`, `[2,1]` | first pair true; permutation false |
| E13 | sets inserted as `[1,2]` and `[2,1]`, then `[1,3]` | permutation true; changed membership false |
| E14 | bags `[1,1,2]`, `[2,1,1]`, `[1,2,2]` | permutation true; changed multiplicity false |
| E15 | ordered sets `[1,2,1]`, `[1,2]`, `[2,1]` | first-occurrence normalization makes first pair true; changed order false |
| E16 | two `Reference<M::Obj>` values with the same universe and object identity but different loaded state and observation; then different object identities with equal fields; then equal object labels from different universes | true; false; `refused { code: foreign_reference }` after `equality.plan-form` (two work units) and before `equality.plan`, with no further charge |
| E17 | binary32 `+0` and `-0` under numeric equality, total-order equivalence and bit identity | true; false; false |
| E18 | qNaN `0x7fc00001` with itself under numeric equality, total-order equivalence and bit identity | false; true; true |
| E19 | binary32 `1.0` and binary64 `1.0` without conversion, then after explicit exact conversion to binary64 | ill-typed; numeric equality true while both source widths remain unchanged |
| E20 | Evaluate two outer-record constructor expressions where a required nested field expression returns undefined, refused or incomplete; then, for `record Two { a: Integer; b: Integer; }`, evaluate the left operand `Two { b: eB, a: eA }` where `eA` is refused and `eB` would charge and be incomplete | the same disposition propagates before either outer value exists, and no Boolean is produced; then `eA`'s refusal propagates because field `a` is first in declaration order, `eB` and the right operand are never evaluated and no charge is made |
| E21 | Two equal values of `record List { head: Integer; tail: List?; }` containing integers 1–8 with the last `tail` absent (`occ = 16` each), once as duplicated trees and once with shared immutable DAG tails, under `ScalarLimitsV1 { integer_bits: 4, decimal_digits: 0, scale_expansion: 0, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 17, work_units: 51, result_units: 1 }`; then deny the 17th `equality.pair` and separately deny `equality.result-retain` | both admitted representations return true after `equality.plan-form` (32 work units), `equality.plan`, exactly 17 pairs and `equality.result-retain`: 51 work units; either denied named charge is incomplete with no Boolean |
| E22 | Under unlimited limits, with `record Holder { r: Reference<M::Obj>; }` (keyed by its reference identity triple) and `h1`, `h2`, `h3` holding distinct objects of one universe: compare values of `Set<Holder>[0,2]` holding `{h1,h2}` with `{h2,h1}`, `{h1,h2}` and `{h1,h3}`; compare values of `Bag<Holder>[0,3]` holding `h1,h1,h2` with `h2,h1,h1`, `h1,h2,h1`, `h1,h2,h2` and `h2,h2,h2`; then `Set<Holder>[0,2]` `{h1,h2}` with `{h1}` | true, true and false, each with `1 + 2 × 2 = 5` rank-matched pairs, `occ = 5` per operand and `10 + 5 + 2 = 17` work units in both operand orders; true, true, false and false, each with `1 + 3 × 2 = 7` pairs, `occ = 7` per operand and `14 + 7 + 2 = 23` work units in both operand orders; false with one pair and `8 + 1 + 2 = 11` work units, because member counts differ. Every count is identical across insertion and hash orders |
| E23 | Run E21's duplicated-tree comparison with E21's tuple except `work_units: 50` | `equality.plan-form` consumes 32 work units; the `equality.plan` reservation of `17 + 2 = 19` is then unavailable before any further counter changes: exactly `incomplete { limit_kind: work_units, limit: 50, consumed: 32, next_charge: 19, charge_point: equality.plan }`, with no pair event, Boolean or other member |
| E24 | Compare records `{ d: 1 cm }` and `{ d: 1 cm }` of one declared record type (`occ = 2` each) under `ScalarLimitsV1 { integer_bits: 0, decimal_digits: 0, scale_expansion: 0, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 2, work_units: 8, result_units: 1 }`; then set `work_units: 7`; then `work_units: 3` | true after exactly `equality.plan-form` (four work units), `equality.plan` (two planned pairs), `equality.pair` for `$` and for `$.d`, and `equality.result-retain`: eight work units and one result unit, with no `unit.*` read, edge or event; then `incomplete { limit_kind: work_units, limit: 7, consumed: 4, next_charge: 4, charge_point: equality.plan }`; then `incomplete { limit_kind: work_units, limit: 3, consumed: 0, next_charge: 4, charge_point: equality.plan-form }` |
| E25 | Under `ScalarLimitsV1 { integer_bits: 0, decimal_digits: 0, scale_expansion: 0, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 1, work_units: 5, result_units: 1 }`, compare Boolean `true = true`, integer `7 != 7` of `Integer`, decimal `a = b` for parameters `a`, `b` of `Decimal[0,100;1,2]` holding `decimal(10, 1)` and `decimal(100, 2)` (two literal operands would have no expected type) and E16's equal references; then repeat the Boolean comparison with `work_units: 4` and with `work_units: 1` | true, false, true and true, each after exactly `equality.plan-form` (two work units), `equality.plan`, one `equality.pair` and `equality.result-retain`, with no other family charge; then `incomplete { limit_kind: work_units, limit: 4, consumed: 2, next_charge: 3, charge_point: equality.plan }`; then `incomplete { limit_kind: work_units, limit: 1, consumed: 0, next_charge: 2, charge_point: equality.plan-form }` |
| E26 | With left operand `convert<T>(e)`: `Int[0,2]` 1 to `Decimal[0,200;2,2]` versus `decimal(100, 2)`; `Decimal[0,9;0,0]` 3 to `Int[0,9]` versus `Int[0,9]` 3; `Rational[0,5;1,1]` 4 to `Int[0,5]` versus `Int[0,5]` 4; `Decimal[0,9;0,1]` `decimal(1, 0)` to `Int[0,9]` versus `Int[0,9]` 1; `Int[0,300]` 1 to `Decimal[0,200;2,2]` versus `decimal(100, 2)`; `Integer` 1 to `Rational[0,2;1,1]` versus `Rational[0,2;1,1]` `rational(1, 1)`; `Decimal[0,100;1,2]` `decimal(10, 1)` to `Decimal[0,100;0,2]` versus `decimal(1, 0)`; `Decimal[0,100;0,2]` `decimal(1, 0)` to `Decimal[0,100;1,2]` versus `decimal(10, 1)`; then `let x = convert<Decimal[0,100;2,2;nearest-even]>(r) in x = d` for E04's `r` and `d` | true; true; true; ill-typed; ill-typed; ill-typed; true; ill-typed; true, with the conversion's `DecimalLoss` retained. Each ill-typed result is a static refusal with no charge even though the value fits |
| E27 | Pair counts under unlimited limits: `Option<Integer>` `none`/`none`, the present value 1 / the present value 2, `none` / the present value 1; `record R { a: Integer; b: Integer?; }` `R { a: 1 }`/`R { a: 1 }`, `R { a: 1, b: null }`/`R { a: 1 }`, `R { a: 1, b: 2 }`/`R { a: 2, b: 2 }`; `Sequence<Integer>[0,3]` `sequence[1, 2]`/`sequence[1, 2, 3]` and `sequence[1, 2]`/`sequence[1, 3]`; `Set<Integer>[0,3]` `set[1, 2]`/`set[2, 1]` and `set[1, 2]`/`set[1, 2, 3]`; `Bag<Integer>[0,3]` `bag[1, 1, 2]`/`bag[1, 2, 2]`, all operands supplied as parameters of those types | as (Boolean, pairs, work units): `(true,1,5)`, `(false,2,8)`, `(false,1,6)`; `(true,3,9)`, `(false,3,9)`, `(false,3,11)`, with no early exit and `occ` 0 for `absent` and `null` slots; `(false,1,10)`, `(false,3,11)`; `(true,3,11)`, `(false,1,10)`; `(false,4,14)` from rank pairs `(1,1)`, `(1,2)` and `(2,2)`. Work units are `occ(left) + occ(right)` plus the pair count plus two |
| E28 | `=` on binary32 `float32(bits: 0x3f800000)` with itself, and on two values of `record F { x: Float32; }`; then declare `Set<Float64>[0,2]` | each is `refused { code: ill_typed, cause: operator-ineligible }` before any charge |
| E29 | Under `ScalarLimitsV1 { integer_bits: 8, decimal_digits: 3, scale_expansion: 2, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 1, work_units: 9, result_units: 2 }`, evaluate E26's first comparison, `convert<Decimal[0,200;2,2]>(i) = decimal(100, 2)` for `i` of `Int[0,2]` holding 1; then with the same tuple except `work_units: 8` | true after `decimal.operands` (`integer_bits` 1, `decimal_digits` 1), `decimal.scale-expansion` (`scale_expansion` 2, `integer_bits = sbits(1,2) = bits(1) + bits(100) = 8`, `decimal_digits = sdigits(1,2) = 3`), `decimal.arithmetic`, `decimal.result-retain`, then the five-unit one-pair equality: nine work units and two result units; then `incomplete { limit_kind: work_units, limit: 8, consumed: 6, next_charge: 3, charge_point: equality.plan }`, because the four decimal charges and `equality.plan-form` consume six work units and `equality.plan` requires capacity for its pair and two more units |
| E30 | For parameters `f`, `g` of `Float64` holding `float64(bits: 0x3ff0000000000000)` and `float64(bits: 0x4000000000000000)`: `f < g`, `f <= g`, `f > g`, `f >= g`; then `quire::value::ieee::totalOrder(f, g)` | each ordering operator is `refused { code: ill_typed, cause: operator-ineligible }` before any charge; `totalOrder` returns true |

For total-order equivalence in E17/E18, evaluate `totalOrder(a,b)` and
`totalOrder(b,a)` and require both; do not alias it to numeric equality. For
E02-E06 and E19, snapshot both source values before comparison and prove that
explicit conversion creates only comparison values. For E10-E16, include an
equal, unequal and wrong-declaration/kind mutation so every recursive and
collection relation is exercised rather than inferred from one generic helper.
For every value-kind row E01-E19, also compare its left operand with a value of
a disjoint kind without conversion and require `ill_typed`, so the cross-type
matrix is measured rather than inferred from numeric examples.

Generate bounded values for every matrix row with same-type equal and unequal
pairs plus disjoint-kind pairs. Numeric generators include normalized and
non-normalized equal representations and explicit lossless/lossy conversion
requests; text includes same-profile equal/unequal values; enums include the
same declaration with equal/different members and different declarations;
collections vary order, uniqueness and multiplicity; references vary identity
and loaded state. The oracle is the selected row relation plus the exact common-
type precondition. Shrink by semantic structure (magnitude, scalar count,
field/element count, recursion depth) while preserving the row and conversion
precondition. For recursive values, generate duplicated-tree and shared-DAG
representations of the same occurrence tree and an illegal containment back-edge;
the first two have identical equality/accounting, while the back-edge refuses
construction under FR-143 before equality.

For E21, the exact depth-first pair paths are `$`, then
`$.head`, `$.tail`, `$.tail.head`, continuing alternately through
`$.tail^7`, `$.tail^7.head`, and the terminal `$.tail^8`; that is eight record
pairs, eight integer-head pairs and one terminal equal pair of two absent `tail`
slots. `equality.plan-form`
adds `16 + 16 = 32` work units, `equality.plan` adds one, the 17 pair events add
17 and result retention adds one.

## Expected Results

Every matrix row returns its tabled Boolean or typed
ill-typed/refused/incomplete disposition. No implicit or lossy conversion,
matching local name, equal record shape, equal referenced state, collection
iteration order or absence/null substitution creates equality. Nested failure
and exhaustion propagate without a false Boolean, no comparison mutates either
operand, and independent sibling comparisons retain their results.
