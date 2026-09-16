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
| DIV-08 | Run truncating `(7,3)` with `ScalarLimitsV1 { integer_bits: 3, decimal_digits: 0, scale_expansion: 0, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 2, work_units: 4, result_units: 2 }`; then deny `integer-division.result-pair` | the exact tuple returns `(2,1)`; the denied named final charge is incomplete with no partial pair |
| DIV-09 | Omit the division definition, select two laws, or substitute wrong revision/digest bytes | semantic admission returns `refused { code: invalid_package }` before evaluation |

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
