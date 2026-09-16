---
id: TC-187
title: "Quantity and unit conversion"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/FR-142
    type: verifies
---

# TC-187: Quantity and unit conversion

## Description

Evaluate compatible exact/lossy conversions and incompatible or affine-unit operations.

## Test Procedure

Select language `ix:native`, edition `1-draft` revision `1-draft.2`, and the
`quire.value.complete/v1` definition at revision `1-draft.1`, using the exact
definition-file byte digests retained by
[`complete-value-lock.json`](../../proposals/quire-v1/definitions/complete-value-lock.json),
including `quire.value.compound-unit/v1`. Use distinct I04 semantic-node keys for length `L`, time `T`
and temperature `Theta`. Define canonical roots metre `m`, second `s` and kelvin `K`; centimetre `cm`
targeting `m` with scale `1/100`; inch `in` targeting `m` with scale `127/5000`;
degree Celsius `degC` targeting `K` with scale `1` and canonical offset `5463/20`; and
degree Fahrenheit `degF` targeting `K` with scale `5/9` and offset
`45967/180`. Execute:

| Vector | Operation | Expected value/disposition |
| --- | --- | --- |
| U01 | Convert `250 cm` to `m`; add to `1 m` | `5/2 m`; sum `7/2 m`, exact and dimension `L` |
| U02 | Normalize `(L*T)/T` and `L^0` | respectively `L` and the empty dimension; zero exponents are absent from each retained sorted map |
| U03 | Add `1 m` to `1 s` where `s` has time dimension `T` | refused as incompatible dimensions before arithmetic |
| U04 | Convert `0 degC` and `32 degF` to `K` | both equal exact `5463/20 K` |
| U05 | Add, subtract, multiply, divide or raise a `degC`/`degF` value to a power | refused because nonzero-offset points have no implicit difference-unit arithmetic |
| U06 | Admit a unit with zero scale | refused before any conversion value is exposed |
| U07 | Convert `1 in` to decimal metres at scale two | `exact`: refused; `nearest-even`: `(3,2) m` plus `{ exact_numerator: 127, exact_denominator: 5000, rounded_coefficient: 3, rounded_scale: 2, mode: nearest-even }` |
| U08 | Convert `-2 m`, `2 m`, `-3 m`, `3 m` into target Decimal coefficient domain `[-2,2]`, scale zero | `-2` and `2` succeed; one-below/one-over refuse atomically |
| U09 | Use a base-dimension name/shape equal to `L` but a different I04 semantic-node key | incompatible and refused despite structural resemblance |
| U09b | Declare a distinct unit key under `L` with scale one and convert it to `m` | conversion succeeds exactly while source and target unit identities remain distinct |
| U10 | Convert `100 cm` to `m` with `ScalarLimitsV1 { integer_bits: 7, decimal_digits: 0, scale_expansion: 0, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 1, value_occurrences: 1, work_units: 6, result_units: 1 }`; then deny `unit.result-retain` | the exact tuple returns `1 m`; the denied named final charge returns incomplete without a partial quantity |
| U11 | Mutate the `L` unit graph to have two targetless roots, no root, a target cycle, an unknown target or a target under time dimension `T` | semantic admission refuses each topology before a quantity value exists |
| U12 | Multiply `2 m * 3 s`, divide `6 m / 2 s`, square `2 m`, and divide `5 m / 5 m` | exact values `6`, `3`, `4`, `1` under canonical compound units `m*s`, `m*s^-1`, `m^2`, and the empty dimensionless unit respectively |
| U13 | Raise `2 m` to integer power `18446744073709551616` with `ScalarLimitsV1 { integer_bits: 64, decimal_digits: 0, scale_expansion: 0, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 1, work_units: 5, result_units: 1 }` | `incomplete { limit_kind: integer_bits, limit: 64, consumed: 2, next_charge: 18446744073709551617, charge_point: unit.rational-arithmetic }` before the power is computed; `next_charge` exceeds `u64::MAX` and is reported exactly |

For U01, U04 and U07, compare the complete exact affine composition through
the canonical unit, result unit identity, normalized dimension map, original
unit/value provenance and any FR-140 `DecimalLoss`.

Generate finite acyclic same-dimension unit graphs with one canonical root,
nonzero reduced rational scales, rational offsets and bounded quantity values.
The oracle composes exact affine maps through the root, applies FR-140 only at
an explicit decimal target, and normalizes compound dimensions/units. Shrink
edge count, rational magnitude, exponent count and value magnitude while
preserving the selected valid or invalid topology. Mutate every topology rule,
dimension key, zero scale, affine-operation restriction and applicable named
unit charge independently.

## Expected Results

Every vector returns the exact quantity or typed refused/incomplete disposition
in the table. No conversion is inferred from equal names or scales. A failure
publishes neither canonical nor target partial values, and exhaustion leaves
independent sibling conversions unchanged.
