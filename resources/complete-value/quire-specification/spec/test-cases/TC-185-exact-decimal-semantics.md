---
id: TC-185
title: "Exact decimal semantics"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/FR-140
    type: verifies
---

# TC-185: Exact decimal semantics

## Description

Evaluate exact, rounded and unrepresentable decimal vectors without binary-float intermediates.

## Test Procedure

Select language `ix:native`, edition `1-draft` revision `1-draft.2`, and the
`quire.value.complete/v1` definition at revision `1-draft.1`, using the exact
definition-file byte digests retained by
[`complete-value-lock.json`](../../proposals/quire-v1/definitions/complete-value-lock.json).
Represent each decimal as `(coefficient, scale)` and
run the following vectors through the real local Rust boundary without a binary
floating-point conversion:

| Vector | Operation and domain | Expected value/disposition |
| --- | --- | --- |
| D01 | Compare `(10,1)` with `(100,2)` | equal normalized values `(1,0)`; provenance retains both source pairs |
| D02 | `(125,2) + (75,2)` | exact `(2,0)` with no loss |
| D03 | Round `(25,1)` and `(-25,1)` to scale zero | `exact`: refused; `toward-zero`: `2,-2`; `toward-positive`: `3,-2`; `toward-negative`: `2,-3`; `nearest-even`: `2,-2`; `nearest-away`: `3,-3`; each success carries `{ exact_numerator: 5|-5, exact_denominator: 2, rounded_coefficient: <tabled>, rounded_scale: 0, mode: <selected> }` |
| D04 | Divide `(1,0)` by `(8,0)` into scale three | exact `(125,3)` |
| D05 | Divide `(1,0)` by `(3,0)` into scale two | `exact`: refused; `nearest-even`: `(33,2)` plus `{ exact_numerator: 1, exact_denominator: 3, rounded_coefficient: 33, rounded_scale: 2, mode: nearest-even }` |
| D06 | Divide by normalized zero | undefined with no decimal value or loss record |
| D07 | Round `(25,1)` using `nearest-away` into coefficient domain `[-2,2]`, scale zero | refused after rounding because coefficient `3` is outside the domain; no value |
| D08 | Admit normalized coefficients `-2` and `2`, then attempt `-3` and `3` in domain `[-2,2]` | endpoints succeed; one-below/one-over refuse |
| D09 | Run D05 with `ScalarLimitsV1 { integer_bits: 7, decimal_digits: 3, scale_expansion: 2, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 2, work_units: 5, result_units: 1 }`; then deny `decimal.result-retain` | the exact tuple admits operands, `100/3`, rounding to `33`, and retention; the denied named final charge returns incomplete after internal arithmetic/rounding but exposes no decimal or loss record |

For every successful rounded result, compare all typed loss fields:
`exact_numerator`, `exact_denominator`, `rounded_coefficient`, `rounded_scale`
and `mode`. Repeat D03 with each sign so directed rounding cannot be implemented
as an unsigned magnitude shortcut.

Generate bounded coefficient/scale pairs, target domains and all six rounding
spellings, including zeros, signs, ties, finite-decimal quotients and recurring
quotients. The oracle is exact rational arithmetic followed by FR-140's one
selected rounding and target admission. Shrink coefficient magnitude, scale
delta and target width while preserving whether the case is exact, a tie,
lossy, out of domain or zero-divisor. For each generated operation, deny every
applicable named decimal charge in turn and require incomplete with no value.

## Expected Results

Every vector returns the value or distinct undefined/refused/incomplete
disposition in the table. All successes retain pre-normalized representation
provenance. Refusal, undefined division and exhaustion contain no decimal value;
an exhausted vector does not alter independently evaluated siblings.
