---
id: TC-192
title: "Integer division profiles"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/FR-147
    type: verifies
---

# TC-192: Integer division profiles

## Description

Evaluate signed vectors, zero divisors and missing finite-backend domains under each division law.

## Test Procedure

Select language `ix:native`, edition `1-draft` revision `1-draft.2`, and the
`quire.value.complete/v1` definition at revision `1-draft.1`. For `div`/`rem`,
select exactly one of the three revision-`1-draft.1` integer-division
definitions in FR-147. Retain the exact DefinitionRefs from
[`complete-value-lock.json`](../../proposals/quire-v1/definitions/complete-value-lock.json).
For `(a,b)` in
`(7,3)`, `(7,-3)`, `(-7,3)` and `(-7,-3)`, execute the paired operation and
compare `(q,r)` with this table:

| Profile | `(7,3)` | `(7,-3)` | `(-7,3)` | `(-7,-3)` |
| --- | --- | --- | --- | --- |
| truncating | `(2,1)` | `(-2,1)` | `(-2,-1)` | `(2,-1)` |
| floor | `(2,1)` | `(-3,-2)` | `(-3,2)` | `(2,-1)` |
| Euclidean | `(2,1)` | `(-2,1)` | `(-3,2)` | `(3,2)` |

For every cell, independently assert `a = b*q + r`, the profile's remainder
sign/range law and atomic exposure of both results. Then execute:

| Vector | Operation/domain | Expected value/disposition |
| --- | --- | --- |
| DIV-01 | Each profile with divisor zero | undefined; neither quotient nor remainder is present |
| DIV-02 | `mod(-7,3)` while each of the three `div`/`rem` definitions is selected | `2` every time because `mod` is independently Euclidean |
| DIV-03 | Mutate the checked definition closure so `mod` claims the truncating or floor definition | semantic admission returns `refused { code: invalid_package, cause: incompatible-definition }`; no expression is evaluated |
| DIV-04 | Mathematical `(-9223372036854775808) / -1` | exact `(9223372036854775808,0)` |
| DIV-05 | DIV-04 in signed-64 consumer domain | atomic domain refusal; neither result is exposed |
| DIV-06 | Signed-64 `MIN/1`, `MAX/1`, then one-below/one-over result membership | endpoints succeed; out-of-domain pair refuses |
| DIV-07 | At I13 negotiation, omit operand, intermediate and result bounds in three separate finite-consumer requests | each returns per-item `requires-bound`; evaluation is not invoked |
| DIV-08 | Run truncating `(7,3)` with `ScalarLimitsV1 { integer_bits: 3, decimal_digits: 0, scale_expansion: 0, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 2, work_units: 4, result_units: 2 }`; then deny `integer-division.result-pair` | the exact tuple returns `(2,1)`; the denied named final charge is exactly `incomplete { limit_kind: work_units, limit: 3, consumed: 3, next_charge: 1, charge_point: integer-division.result-pair }` with no partial pair or other member |
| DIV-09 | Omit the division definition, select two laws, or substitute wrong revision/digest bytes | semantic admission returns `refused { code: invalid_package }` before evaluation |
| DIV-10 | Run `mod(-7,3)` with `ScalarLimitsV1 { integer_bits: 3, decimal_digits: 0, scale_expansion: 0, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 2, work_units: 4, result_units: 1 }`; then deny each of `integer-modulus.operands`, `integer-modulus.arithmetic`, `integer-modulus.domain` and `integer-modulus.result-retain` in turn | the exact tuple returns `2` after exactly those four charges and no `integer-division.*` charge; the denials return `incomplete { limit_kind: work_units, limit: k, consumed: k, next_charge: 1, charge_point: <denied point> }` for `k` = 0, 1, 2 and 3 respectively, with no remainder |
| DIV-11 | Run truncating `7 div 0` with DIV-08's tuple and `mod(7,0)` with DIV-10's tuple, each with `work_units: 1`; then each with `work_units: 0` | `work_units: 1`: undefined after the operands charge, with no arithmetic charge; `work_units: 0`: `incomplete { limit_kind: work_units, limit: 0, consumed: 0, next_charge: 1, charge_point: integer-division.operands }` and `integer-modulus.operands` respectively |
| DIV-12 | Run `mod(-7,3)` for a bounded consumer result domain `Int[0,1]` with DIV-10's tuple except `result_units: 0`; then `work_units: 2` | refused after `integer-modulus.domain` because remainder `2` is outside `[0,1]`; `integer-modulus.result-retain` is not charged, so it is not incomplete. With `work_units: 2`: `incomplete { limit_kind: work_units, limit: 2, consumed: 2, next_charge: 1, charge_point: integer-modulus.domain }`. (A signed-64 domain cannot refuse the Euclidean remainder of signed-64 operands, so this bounded domain is narrower.) |
| DIV-13 | Run DIV-08 with `integer_bits: 2` and `work_units: 0` | both counters are short at `integer-division.operands`; the first in `ScalarLimitsV1` field order is reported: `incomplete { limit_kind: integer_bits, limit: 2, consumed: 0, next_charge: 3, charge_point: integer-division.operands }` |

Generate bounded signed operand pairs for all three laws, emphasizing zero,
unit divisors, sign quadrants and finite-domain endpoints. The oracle computes
the unique exact pair satisfying `a = b*q + r` plus the selected remainder law;
`mod` always uses the Euclidean remainder. Shrink operand magnitude while
preserving divisor-zero, sign quadrant, domain overflow or selected-law
distinction. Deny each applicable named integer-division charge independently
and require atomic incomplete output.

## Expected Results

Every signed vector produces the tabled pair and satisfies its law. Zero,
profile misuse, missing bounds, finite-domain overflow and exhaustion remain
distinct typed dispositions. No failing case exposes only one member of a pair,
uses host overflow or substitutes floating-point division; independent sibling
results remain intact.
