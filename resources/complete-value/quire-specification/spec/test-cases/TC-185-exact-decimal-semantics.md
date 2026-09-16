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
| D05 | Divide `(1,0)` by `(3,0)` into scale two | `exact`: refused; `nearest-even`: `(33,2)` plus `{ exact_numerator: 1, exact_denominator: 3, rounded_coefficient: 33, rounded_scale: 2, mode: nearest-even }`; the loss carries the mathematical value `1/3`, not the coefficient-space intermediate `100/3` |
| D06 | Divide by normalized zero | undefined with no decimal value or loss record |
| D07 | Round `(25,1)` using `nearest-away` into coefficient domain `[-2,2]`, scale zero | refused after rounding because coefficient `3` is outside the domain; no value |
| D08 | Admit normalized coefficients `-2` and `2`, then attempt `-3` and `3` in domain `[-2,2]` | endpoints succeed; one-below/one-over refuse |
| D09 | Run D05 with `ScalarLimitsV1 { integer_bits: 8, decimal_digits: 3, scale_expansion: 2, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 2, work_units: 5, result_units: 1 }`; then deny `decimal.result-retain` | the exact tuple admits operands, the dividend expanded by shift 2 (`sbits(1,2) = bits(1) + bits(100) = 8`, `sdigits(1,2) = 3`), `decimal.arithmetic` (`max(8, bits(3)) = 8`), rounding `100/3` to `33` (sized as `decimal.arithmetic`, from the aligned operands: `integer_bits = max(8, bits(3)) = 8`, `decimal_digits = max(3, digits(3)) = 3`), and retention; the denied named final charge returns exactly `incomplete { limit_kind: work_units, limit: 4, consumed: 4, next_charge: 1, charge_point: decimal.result-retain }` after internal arithmetic/rounding and exposes no decimal, loss record or other member |
| D10 | Type-check `Decimal[3,2;0,0]`, `Decimal[0,9;2,1]` and `Decimal[0,9;0,4294967296]`; then `Decimal[2,2;1,1]` and `Decimal[0,9;0,4294967295]` | the first three each return `refused { code: ill_typed }`; the last two are well formed |
| D11 | Admit `(1,0)`, `(11,1)`, `(1000,3)` and `(9999,2)` into `Decimal[0,10000;2,2]`; `(100,0)` and `(9999,2)` into `Decimal[0,9999;2,2]`; `(1,3)` into `Decimal[-9,9;0,2]` | `1` (`c* = 100`), `1.1` (`c* = 110`), pre-normalized `(1000,3)` (value `1`, `c* = 100`) and `99.99` (`c* = 9999`) are members; `100` refuses in `[0,9999]` because `c* = 10000` exceeds `hi` only after lifting to scale 2, while `99.99` is a member; `0.001` refuses because `s* = 3 > 2`; equal values always have equal membership |
| D12 | Divide `(1,0)` by `(3,0)` into `Decimal[-1000,1000;0,2;nearest-even]`, then into `Decimal[-10,10;0,2;nearest-even]` | first: `(33,2)` with loss `1/3`; second: rounded `(33,2)` refuses on coefficient and is never re-rounded to `(3,1)` |
| D13 | Multiply `(150,2)` by `(2,0)` into `Decimal[-9,9;0,0;toward-zero]` and into `Decimal[-9,9;0,0;exact]` with `ScalarLimitsV1 { integer_bits: 10, decimal_digits: 4, scale_expansion: 0, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 2, work_units: 4, result_units: 1 }`; then set `work_units: 3`; then restore `work_units: 4` and set `integer_bits: 9` | discarded digits `00` are not a rounding step: both modes return `(3,0)` with no loss and no `decimal.rounding` charge; with `work_units: 3` both return `incomplete { limit_kind: work_units, limit: 3, consumed: 3, next_charge: 1, charge_point: decimal.result-retain }`; with `integer_bits: 9` both return `incomplete { limit_kind: integer_bits, limit: 9, consumed: 8, next_charge: 10, charge_point: decimal.arithmetic }` because the multiplication amount is `bits(150) + bits(2) = 10` (and `digits(150) + digits(2) = 4`) on the retained operand `(150,2)`, not `bits(15) + bits(2) = 6` on normalized `(15,1)` |
| D14 | Run D05 `exact` into `Decimal[-1000,1000;0,2;exact]` with D09's tuple except `work_units: 3` | `refused` after `decimal.arithmetic` with no `decimal.rounding` charge; not incomplete. With `work_units: 2`: `incomplete { limit_kind: work_units, limit: 2, consumed: 2, next_charge: 1, charge_point: decimal.arithmetic }` |
| D15 | Divide `(1,0)` by `(0,3)` into `Decimal[-1000,1000;0,2;nearest-even]` with D09's tuple except `work_units: 1`; then `work_units: 0` | `work_units: 1`: undefined after `decimal.operands`, with no scale-expansion charge; `work_units: 0`: `incomplete { limit_kind: work_units, limit: 0, consumed: 0, next_charge: 1, charge_point: decimal.operands }` |
| D16 | Run D07 with `ScalarLimitsV1 { integer_bits: 5, decimal_digits: 2, scale_expansion: 0, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 1, work_units: 4, result_units: 0 }` | refused on coefficient `3` after `decimal.rounding`; `decimal.result-retain` is not charged, so `result_units: 0` does not make it incomplete. With `work_units: 3`: `incomplete { limit_kind: work_units, limit: 3, consumed: 3, next_charge: 1, charge_point: decimal.rounding }` |
| D17 | Divide `(100,2)` by `(1,0)` into `Decimal[-1000,1000;0,2]` with `ScalarLimitsV1 { integer_bits: 7, decimal_digits: 3, scale_expansion: 1, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 2, work_units: 4, result_units: 1 }` | exact value `1` retained as `(100,2)`; the retained shift `abs(2 + 0 - 2)` is zero, so `scale_expansion: 1` suffices (normalized operands would need shift 2) |
| D18 | Divide `(1234,3)` by `(2,0)` into `Decimal[-1000,1000;0,1;nearest-even]` with `ScalarLimitsV1 { integer_bits: 11, decimal_digits: 4, scale_expansion: 2, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 2, work_units: 5, result_units: 1 }`; then `scale_expansion: 1` | divisor side expands: `D = 2 × 10^2 = 200`, `N = 1234`, reduced intermediate `617/100`; result `(6,1)` with `{ exact_numerator: 617, exact_denominator: 1000, rounded_coefficient: 6, rounded_scale: 1, mode: nearest-even }`; with `scale_expansion: 1`: `incomplete { limit_kind: scale_expansion, limit: 1, consumed: 0, next_charge: 2, charge_point: decimal.scale-expansion }` |
| D19 | Admit `(0,0)` and `(1,0)` into `Decimal[0,10;4294967295,4294967295]` under `ScalarLimitsV1` with every limit zero | `0` is a member (`c* = 0`); `1` is refused because `c* = 10^4294967295` has 4294967296 digits and exceeds `hi`; neither decision materializes `c*`, charges a counter or returns incomplete |
| D20 | Order `(15,1) < (2,0)` for operands of `Decimal[0,100;0,1]` with `ScalarLimitsV1 { integer_bits: 6, decimal_digits: 2, scale_expansion: 1, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 2, work_units: 3, result_units: 1 }`; then with `integer_bits: 5` | true after `ordering.operands` (`integer_bits` 4, `decimal_digits` 2, `value_occurrences` 2), `ordering.arithmetic` aligned to scale 1 (`scale_expansion` 1, `sbits(15,0) = 4` and the scaled coefficient `sbits(2,1) = bits(2) + bits(10) = 6`: `integer_bits` 6; `sdigits(2,1) = 2`: `decimal_digits` 2) and `ordering.result-retain`: three work units and one result unit; then `incomplete { limit_kind: integer_bits, limit: 5, consumed: 4, next_charge: 6, charge_point: ordering.arithmetic }` |
| D21 | Order `(100,2) < (2,0)` for operands of `Decimal[0,100;0,2]` retained as written, with `ScalarLimitsV1 { integer_bits: 9, decimal_digits: 3, scale_expansion: 2, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 2, work_units: 3, result_units: 1 }`; then with `integer_bits: 8` | true after `ordering.operands` on the retained coefficients 100 and 2 (`integer_bits = max(7, 2) = 7`, `decimal_digits = 3`, `value_occurrences = 2`), `ordering.arithmetic` aligned to scale `max(2, 0) = 2` (`scale_expansion = 2`, `sbits(100,0) = 7` and the scaled coefficient `sbits(2,2) = bits(2) + bits(100) = 9`: `integer_bits = 9`; `decimal_digits = max(3, sdigits(2,2)) = 3`) and `ordering.result-retain`: three work units and one result unit. Normalizing `(100,2)` to `(1,0)` would wrongly give `integer_bits` 2 and `scale_expansion` 0. Then `incomplete { limit_kind: integer_bits, limit: 8, consumed: 7, next_charge: 9, charge_point: ordering.arithmetic }` |
| D22 | Multiply `(1,0)` by `(1,0)` into `Decimal[0,1;0,1000;exact]` with `ScalarLimitsV1 { integer_bits: 3323, decimal_digits: 1001, scale_expansion: 1000, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 2, work_units: 4, result_units: 1 }`; then with `integer_bits: 3322`; then with `scale_expansion: 999` | exact value `1` retained as `(10^1000, 1000)` after `decimal.operands` (`integer_bits` 1, `decimal_digits` 1, `value_occurrences` 2), `decimal.scale-expansion` (shift 0), `decimal.arithmetic` (`bits(1) + bits(1) = 2`, `digits(1) + digits(1) = 2`), no rounding step, membership (`s* = 0`, `c* = 1`) and `decimal.result-retain` computed analytically for the target-scale upscale `k = 1000` of the materialized product `1` (`scale_expansion` 1000, `integer_bits = sbits(1,1000) = bits(1) + bits(10^1000) = 1 + 3322 = 3323`, `decimal_digits = sdigits(1,1000) = 1001`): four work units and one result unit; `incomplete { limit_kind: integer_bits, limit: 3322, consumed: 2, next_charge: 3323, charge_point: decimal.result-retain }`; `incomplete { limit_kind: scale_expansion, limit: 999, consumed: 0, next_charge: 1000, charge_point: decimal.result-retain }`; neither incomplete run materializes `10^1000` or exposes a value |
| D23 | Multiply `(1,0)` by `(1,0)` into `Decimal[0,1;0,4294967295;exact]` with `ScalarLimitsV1 { integer_bits: 18446744073709551615, decimal_digits: 64, scale_expansion: 4294967295, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 2, work_units: 4, result_units: 1 }` | `incomplete { limit_kind: decimal_digits, limit: 64, consumed: 2, next_charge: 4294967296, charge_point: decimal.result-retain }` after three work units; the amounts are `sdigits(1,4294967295) = 4294967296` and `sbits(1,4294967295) = 1 + bits(10^4294967295) = 14267572525`, derived without materializing the retained coefficient `10^4294967295`, which is never materialized |

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
