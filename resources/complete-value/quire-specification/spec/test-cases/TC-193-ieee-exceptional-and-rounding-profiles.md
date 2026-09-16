---
id: TC-193
title: "IEEE exceptional and rounding profiles"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/FR-148
    type: verifies
---

# TC-193: IEEE exceptional and rounding profiles

## Description

Evaluate NaN, infinity, signed zero, width and rounding-mode discriminator vectors.

## Test Procedure

Select language `ix:native`, edition `1-draft` revision `1-draft.2`, and the
`quire.value.complete/v1` and `quire.value.ieee754-2019-default/v1` definitions
at revision `1-draft.1`, using their exact DefinitionRefs in
[`complete-value-lock.json`](../../proposals/quire-v1/definitions/complete-value-lock.json).
Invoke only the qualified intrinsics fixed
by FR-148 and execute these comparison/propagation vectors through the real
local Rust boundary:

| Vector | Inputs/operation | Expected result |
| --- | --- | --- |
| F01 | binary32 `+0=0x00000000`, `-0=0x80000000` | numeric equal; not bit-identical; `totalOrder(-0,+0)` |
| F02 | qNaNs `0x7fc00001`, `0x7fc00002` | each is not numerically equal to itself; each is bit-identical to itself; payload 1 precedes payload 2 under `totalOrder` |
| F02b | negative qNaNs `0xffc00002`, `0xffc00001` and positive signaling/quiet pair `0x7f800001`, `0x7fc00001` | negative payload 2 precedes negative payload 1; positive signaling precedes positive quiet under `totalOrder` |
| F03 | add qNaN `0xffc00021` to qNaN `0x7fc00012` | left bits `0xffc00021` are retained, no flag |
| F04 | add sNaN `0x7f800021` to qNaN `0x7fc00012` | `0x7fc00021`, flag `{invalid}` |
| F04b | add qNaN `0x7fc00012` to sNaN `0xff800021`; then `fma(0x3f800000, 0x7fc00012, 0xff800021)` | add returns `0x7fc00012, {invalid}`; FMA returns `0x7fc00012, {invalid}` because the leftmost NaN is the second operand and the later third operand is signaling |
| F05 | compare binary32 positive/negative infinity and finite extrema | numeric and `totalOrder` results follow value order; bit identity follows width plus bits |
| F06 | compare binary32 `0x3f800000` with binary64 `0x3ff0000000000000` | ill-typed until an explicit width conversion is selected |

Execute the following arithmetic bit/flag vectors. An omitted flag set means
the empty set:

| Vector | Width, operation and mode | Expected bits and flags |
| --- | --- | --- |
| F07 | binary32 `0x3f800000 + 0x33800000` (`1 + 2^-24`) | nearest-even/toward-zero/toward-negative: `0x3f800000`; nearest-away/toward-positive: `0x3f800001`; every rounded result flags `{inexact}` |
| F08 | binary32 `0xbf800000 + 0xb3800000` (`-1 - 2^-24`) | nearest-even/toward-zero/toward-positive: `0xbf800000`; nearest-away/toward-negative: `0xbf800001`; every rounded result flags `{inexact}` |
| F09 | F07 under strict `exact`, then `0x3f800000 + 0x34000000` (`1 + 2^-23`) under `exact` | first refuses with would-be `{inexact}` and no bits; second returns `0x3f800001` with no flags |
| F10 | binary64 `0x3ff0000000000000 + 0x3ca0000000000000` (`1 + 2^-53`) | nearest-even/toward-zero/toward-negative: `0x3ff0000000000000`; nearest-away/toward-positive: `0x3ff0000000000001`; `{inexact}` |
| F11 | binary32 FMA `a=0x3f800001`, `b=0x3f7ffffe`, `c=0xbf800000`, nearest-even | fused result `0xa8800000`; separately rounded multiply then add gives `0x00000000` |
| F12 | `0x00000000 * 0x7f800000`; `sqrt(0xbf800000)`, under each selected direction and strict `exact` | canonical `0x7fc00000`, flag `{invalid}` for each |
| F13 | finite `+1`/`-1` divided by `+0`/`-0`, under each selected direction and strict `exact` | respectively `0x7f800000`, `0xff800000`, `0xff800000`, `0x7f800000`; flag `{divide_by_zero}` |
| F14 | `0x7f7fffff + 0x7f7fffff`, nearest-even then strict `exact` | nearest-even returns `0x7f800000` with `{overflow,inexact}`; `exact` refuses with those would-be flags and no bits |
| F15 | `0x00800000 * 0x3f000000`, nearest-even | exact subnormal `0x00400000`, no flags |
| F16 | `0x00000001 * 0x3f000000`, nearest-even | `0x00000000`, flags `{underflow,inexact}` |
| F17 | binary32 `numericEqual(0x7fc00001, 0x3f800000)` with `ScalarLimitsV1 { integer_bits: 32, decimal_digits: 0, scale_expansion: 0, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 2, work_units: 2, result_units: 1 }` | false; only `ieee.operands` and `ieee.result-retain` apply; denying either is incomplete without a Boolean |
| F18 | binary32 `0x00000000 * 0x7f800000` with the F17 tuple | `0x7fc00000`, `{invalid}`; this classified exceptional path has no exact-intermediate/round charge; denying result retention is incomplete without bits/flags |
| F19 | binary32 `sqrt(0x40000000)` (sqrt 2), nearest-even, with `ScalarLimitsV1 { integer_bits: 32, decimal_digits: 0, scale_expansion: 0, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 1, work_units: 4, result_units: 1 }` | `0x3fb504f3`, `{inexact}`; denying `ieee.exact-intermediate`, `ieee.round` or `ieee.result-retain` is incomplete without bits/flags, although the exact real is irrational |

In F20-F31, `I(i,o,w,r)` abbreviates `ScalarLimitsV1 { integer_bits: i,
decimal_digits: 0, scale_expansion: 0, text_input_bytes: 0, text_scalars: 0,
normalized_scalars: 0, unit_edges: 0, value_occurrences: o, work_units: w,
result_units: r }`.

| Vector | Width, operation, mode and limits | Expected result |
| --- | --- | --- |
| F20 | Under `I(0,0,0,0)`, apply `numericEqual`, `totalOrder` and `bitIdentical` to binary32 `0x3f800000` and binary64 `0x3ff0000000000000`; add those two operands; add binary32 `0x3f800000` to exact integer `1`; convert binary32 `0x3f800000` directly to `Decimal[0,10;0,0]` and to `Int[0,1]` | each returns `refused { code: ill_typed }` with no charge |
| F21 | binary32 `0x00000000 + 0x00000000` and `0x80000000 * 0x3f800000`, nearest-even, under `I(32,2,4,1)`; then under `I(32,2,3,1)` | `0x00000000` and `0x80000000`, no flags, after all four IEEE charges; then `incomplete { limit_kind: work_units, limit: 3, consumed: 3, next_charge: 1, charge_point: ieee.result-retain }` for each, because zero operands take the finite path |
| F22 | F09's first operation under strict `exact` and under nearest-even, each under `I(32,2,3,1)`; then strict `exact` under `I(32,2,2,1)` | `exact` returns the F09 refusal after `ieee.round` and before `ieee.result-retain`; nearest-even returns `incomplete { limit_kind: work_units, limit: 3, consumed: 3, next_charge: 1, charge_point: ieee.result-retain }`; then `exact` returns `incomplete { limit_kind: work_units, limit: 2, consumed: 2, next_charge: 1, charge_point: ieee.round }` |
| F23 | binary32 `0x7f7fffff + 0x73000000` (maximum finite plus half its unit in the last place) under nearest-even, toward-zero and strict `exact` | nearest-even returns `0x7f800000` with `{overflow,inexact}`; toward-zero returns `0x7f7fffff` with `{inexact}`; `exact` refuses with would-be flags `{overflow,inexact}`, the `nearest-even` flags, and no would-be bits |
| F24 | Width-convert binary32 `0x3f800000` to binary64, nearest-even, under `I(64,1,4,1)`, then `I(32,1,4,1)`; width-convert binary64 `0x3ff0000000000000` to binary32 under `I(32,1,4,1)`; convert exact `1/3` to binary32, nearest-even, under `I(32,1,4,1)`, then `I(2,1,4,1)`; convert binary32 `0x3f000000` to `Rational[0,1;1,2]` under `I(32,1,3,1)`; convert binary32 `0x7fc00000` to `Rational[0,1;1,2]` under `I(32,1,2,1)` | `0x3ff0000000000000`, no flags; then `incomplete { limit_kind: integer_bits, limit: 32, consumed: 32, next_charge: 64, charge_point: ieee.exact-intermediate }`; `incomplete { limit_kind: integer_bits, limit: 32, consumed: 0, next_charge: 64, charge_point: ieee.operands }`; `0x3eaaaaab` with `{inexact}`; then `incomplete { limit_kind: integer_bits, limit: 2, consumed: 2, next_charge: 32, charge_point: ieee.exact-intermediate }`; exact `1/2` after `ieee.operands`, `ieee.exact-intermediate` (`maxparts` 2) and `ieee.result-retain`; undefined after `ieee.operands` only |
| F25 | Width-convert binary64 `0x7ff0000000000001` and `0xfff8000000000003` to binary32 under `I(64,1,2,1)`; binary32 `0x7fc00001` to binary64 under `I(32,1,2,1)`; binary64 `0x7ff8000000400000` to binary32 under `I(64,1,1,0)`, then under `I(64,1,0,0)`; signaling binary64 `0x7ff0000000400000` to binary32 under `I(64,1,1,0)` | `0x7fc00001` with `{invalid}` and `0xffc00003` with no flag, keeping sign and payload; `0x7ff8000000000001` with no flag; `refused { code: ieee_nan_payload_not_representable }` with no flags after `ieee.operands` and before `ieee.result-retain`, because payload `2^22` does not fit binary32; then `incomplete { limit_kind: work_units, limit: 0, consumed: 0, next_charge: 1, charge_point: ieee.operands }`; the signaling source returns the same refusal with no `invalid` flag, because the NaN is not consumed |
| F26 | Width-convert binary32 `0x80000000` to binary64, nearest-even, under `I(64,1,4,1)`, then `I(64,1,3,1)`; add binary32 `0x80000000 + 0x80000000` under each selected direction and strict `exact`; add binary32 `0x3f800000 + 0xbf800000` under each selected direction and strict `exact`; subtract binary32 `0x80000000 - 0x00000000`, `0x00000000 - 0x00000000` and `0x3f800000 - 0x3f800000` under each selected direction and strict `exact` | `0x8000000000000000`, no flags; then `incomplete { limit_kind: work_units, limit: 3, consumed: 3, next_charge: 1, charge_point: ieee.result-retain }`; `0x80000000`, no flags, in every mode; `0x80000000` under `toward-negative` and `0x00000000` under every other direction and `exact`, no flags; binary32 `0x80000000 - 0x00000000` gives `0x80000000` in every mode; `0x00000000 - 0x00000000` and `0x3f800000 - 0x3f800000` each give `0x80000000` under `toward-negative` and `0x00000000` under every other direction and `exact`, no flags |
| F27 | Convert binary64 `0x0000000000000001` (`2^-1074`) to `Rational[0,1;1,D]`, where `D` is the decimal literal of `2^1074`, under `I(1075,1,3,1)`, then `I(1074,1,3,1)`; convert binary32 `0x80000000` to `Rational[0,0;1,1]` under `I(32,1,3,1)`, then `I(32,1,2,1)`; convert binary32 `0x7f800000` to `Rational[0,1;1,2]` under `I(32,1,1,0)`, then `I(32,1,0,0)`; convert binary32 `0x3f000000` (`1/2`) to `Rational[0,1;1,1]` under `I(32,1,2,0)`, then `I(32,1,1,0)` | exact `1/2^1074`; then `incomplete { limit_kind: integer_bits, limit: 1074, consumed: 64, next_charge: 1075, charge_point: ieee.exact-intermediate }`; exact `0`; then `incomplete { limit_kind: work_units, limit: 2, consumed: 2, next_charge: 1, charge_point: ieee.result-retain }`; undefined after `ieee.operands` only; then `incomplete { limit_kind: work_units, limit: 0, consumed: 0, next_charge: 1, charge_point: ieee.operands }`; `1/2` returns `refused { code: ieee_rational_out_of_domain }` at evaluation because denominator `2` exceeds `1`, after `ieee.exact-intermediate` and before `ieee.result-retain` (no result unit needed); then `incomplete { limit_kind: work_units, limit: 1, consumed: 1, next_charge: 1, charge_point: ieee.exact-intermediate }` |
| F28 | Convert decimal `1.00` (coefficient `100`, scale `2`) to binary32, nearest-even, under `I(32,1,4,1)`, then `I(6,1,4,1)` | `0x3f800000`, no flags; then `incomplete { limit_kind: integer_bits, limit: 6, consumed: 0, next_charge: 7, charge_point: ieee.operands }`, because the retained representation measures `max(bits(100), bits(10^2)) = 7`, not the reduced value `1` |
| F29 | binary32 `0x7f7fffff + 0x72800000` (maximum finite plus a quarter of its last-place unit), then `0x007fffff * 0x3f800001` (maximum subnormal times `1 + 2^-23`, exactly `2^-126 - 2^-172`), each under nearest-even and strict `exact` | `0x7f7fffff` with `{inexact}` and no overflow; `exact` refuses with would-be `{inexact}`; `0x00800000` with `{inexact}` and no underflow, because the rounded result is the minimum normal; `exact` refuses with would-be `{inexact}` |
| F30 | binary32 `sqrt(0x80000000)`, nearest-even, under `I(32,1,4,1)`, then `I(32,1,3,1)` | `0x80000000`, no flags, after four work units; then `incomplete { limit_kind: work_units, limit: 3, consumed: 3, next_charge: 1, charge_point: ieee.result-retain }` |
| F31 | Width-convert to binary32: binary64 `0xfff0000000000000` under `I(64,1,2,1)`, then `I(64,1,1,1)`; `0x3fb999999999999a` (binary64 0.1) under nearest-even with `I(64,1,4,1)`, under strict `exact` with `I(64,1,3,1)`, then `I(64,1,2,1)`; `0x47efffffe0000000` under nearest-even; `0x47effffff0000000` under nearest-even with `I(64,1,4,1)`, then `I(64,1,3,1)`, and under toward-zero; `0x0000000000000001` under nearest-even with `I(64,1,4,1)`, then `I(64,1,3,1)` | `0xff800000`, no flags, after `ieee.operands` and `ieee.result-retain` only; then `incomplete { limit_kind: work_units, limit: 1, consumed: 1, next_charge: 1, charge_point: ieee.result-retain }`; `0x3dcccccd` with `{inexact}`; `exact` refuses with would-be `{inexact}` after `ieee.round`; then `incomplete { limit_kind: work_units, limit: 2, consumed: 2, next_charge: 1, charge_point: ieee.round }`; `0x7f7fffff` with no flags, because the source is exactly the binary32 maximum; `0x7f800000` with `{overflow,inexact}`, a tie with the maximum's odd significand rounding to `2^128`; then `incomplete { limit_kind: work_units, limit: 3, consumed: 3, next_charge: 1, charge_point: ieee.result-retain }`; toward-zero returns `0x7f7fffff` with `{inexact}`; `0x00000000` with `{underflow,inexact}`; then `incomplete { limit_kind: work_units, limit: 3, consumed: 3, next_charge: 1, charge_point: ieee.result-retain }` |

At semantic admission, separately omit the IEEE definition, select two IEEE
definitions, substitute a different revision/digest, and attempt to bind a user
declaration to one of the reserved qualified intrinsic identities. Each returns
`refused { code: invalid_package }` before evaluation; the reserved-identity
binding returns exactly `refused { code: invalid_package, cause:
conflicting-definition }`. At I13 negotiation, an unavailable
width/intrinsic/policy returns per-item `unsupported`, while a missing required
finite proof returns per-item `requires-bound`; neither changes package
admission or invokes a substitute evaluator.

Run F10 with `ScalarLimitsV1 { integer_bits: 64, decimal_digits: 0,
scale_expansion: 0, text_input_bytes: 0, text_scalars: 0,
normalized_scalars: 0, unit_edges: 0, value_occurrences: 2, work_units: 4,
result_units: 1 }`, then use the NFR-071 seam to deny `ieee.result-retain`;
the exact tuple returns the tabled bits, while the denied named final charge
returns incomplete without bits. Run F13 immediately before an exact no-flag
operation to prove flags are operation-local rather than sticky global state.

Generate the Cartesian class matrix for both widths, all five IEEE directions
plus strict `exact`, and add/subtract/multiply/divide/square-root/FMA: signs,
positive/negative zero, minimum/maximum subnormal, minimum normal, maximum
finite, infinities, and signaling/quiet NaNs in every operand position. The
oracle is FR-148's exact-real operation on dyadic operands followed by its one selected rounding,
NaN and flag rules. Shrink finite operands toward zero, powers of two and format
boundaries; shrink NaNs by payload then sign while preserving signaling class.
Require signed-zero rules, exact-subnormal behavior, directed overflow, invalid
infinity combinations, finite/infinity results and NaN propagation for every
applicable class, rather than extrapolating them from F01-F16.

## Expected Results

Every vector returns the exact width, bits, fresh flag set, comparison or typed
refused type-checking result, evaluator incomplete result or I13 disposition in
the tables. Provenance retains the
selected width, rounding/strictness policy and IEEE definition. NaN selection,
strict `exact`, unsupported profiles and exhaustion never emit an alternative
rounded payload, and independent sibling operations retain their own flags and
results.
