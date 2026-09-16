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
`45967/180`. Also define mass `M` with root `kg`; distinct derived dimension
nodes `Torque` and `Energy`, both with terms `{ L: 2, M: 1, T: -2 }` and roots
`N_m` and `J`; derived dimension `Area` with terms `{ L: 2 }` and root `m2`;
`u1` targeting `K` with scale `1` and offset `10`; `u2` targeting `u1` with
scale `1` and offset `-10`; `u3` targeting `degC` with scale `1` and offset
`0`; `rev` targeting `m` with scale `-1` and offset `0`; and `cm2` targeting `m2` with scale `1/10000` and offset `0`. In the vectors
below, `L(i,d,e,o,w,r)` abbreviates `ScalarLimitsV1 { integer_bits: i,
decimal_digits: d, scale_expansion: 0, text_input_bytes: 0, text_scalars: 0,
normalized_scalars: 0, unit_edges: e, value_occurrences: o, work_units: w,
result_units: r }`. Execute:

| Vector | Operation | Expected value/disposition |
| --- | --- | --- |
| U01 | Convert `250 cm` to `m`; add to `1 m` | `5/2 m`; sum `7/2 m`, exact and dimension `L` |
| U02 | Normalize `(L*T)/T` and `L^0` | respectively `L` and the empty dimension; zero exponents are absent from each retained sorted map |
| U03 | Add `1 m` to `1 s` where `s` has time dimension `T` | `refused { code: ill_typed }` for incompatible dimensions before arithmetic |
| U04 | Convert `0 degC` and `32 degF` to `K` | both equal exact `5463/20 K` |
| U05 | Add, subtract, multiply, divide or raise a `degC`/`degF` value to a power | `refused { code: ill_typed }` because nonzero-offset points have no implicit difference-unit arithmetic |
| U06 | Admit a unit with zero scale | refused before any conversion value is exposed |
| U07 | Convert `1 in` to decimal metres at scale two | `exact`: refused; `nearest-even`: `(3,2) m` plus `{ exact_numerator: 127, exact_denominator: 5000, rounded_coefficient: 3, rounded_scale: 2, mode: nearest-even }` |
| U08 | Convert `-2 m`, `2 m`, `-3 m`, `3 m` into target Decimal coefficient domain `[-2,2]`, scale zero | `-2` and `2` succeed; one-below/one-over refuse atomically |
| U09 | Use a base-dimension name/shape equal to `L` but a different I04 semantic-node key | `refused { code: ill_typed }` as incompatible despite structural resemblance |
| U09b | Declare a distinct unit key under `L` with scale one and convert it to `m` | conversion succeeds exactly while source and target unit identities remain distinct |
| U10 | Convert `100 cm` to `m` with `ScalarLimitsV1 { integer_bits: 8, decimal_digits: 0, scale_expansion: 0, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 1, value_occurrences: 1, work_units: 6, result_units: 1 }`; then deny `unit.result-retain` | the exact tuple returns `1 m`, with the largest amount `max(bits(100) + bits(1), bits(1) + bits(100)) = 8` at the multiply event; the denied named final charge returns incomplete without a partial quantity |
| U11 | Mutate the `L` unit graph to have two targetless roots, no root, a target cycle, an unknown target or a target under time dimension `T` | semantic admission refuses each topology before a quantity value exists |
| U12 | Multiply `2 m * 3 s`, divide `6 m / 2 s`, square `2 m`, and divide `5 m / 5 m` | exact values `6`, `3`, `4`, `1` under canonical compound units `m*s`, `m*s^-1`, `m^2`, and the empty dimensionless unit respectively |
| U13 | Raise `2 m` to integer power `18446744073709551616` with `ScalarLimitsV1 { integer_bits: 64, decimal_digits: 0, scale_expansion: 0, text_input_bytes: 0, text_scalars: 0, normalized_scalars: 0, unit_edges: 0, value_occurrences: 1, work_units: 5, result_units: 1 }` | `incomplete { limit_kind: integer_bits, limit: 64, consumed: 2, next_charge: 36893488147419103232, charge_point: unit.rational-arithmetic }` before the power is computed, since the amount is `abs(n) × maxparts(2) = 2^64 × 2`; `next_charge` exceeds `u64::MAX` and is reported exactly |
| U14 | Under `L(0,0,0,0,0,0)`, add `1 m + 1 s`, add `1 degC + 1 s`, add `1 degC + 1 degC`, divide `1 degC / 0 degC`, and add `1 cm + 1 m` | each returns `refused { code: ill_typed }` with no charge; the static order among incompatible dimensions, affine arithmetic and distinct units is informative only, while `1 degC / 0 degC` is refused statically as affine arithmetic rather than reaching the runtime zero divisor |
| U15 | Convert `1 in` to `cm` under `L(15,0,2,1,9,1)`, then under `L(7,0,1,1,9,1)` | exact `127/50 cm` after one read, two edges, four rational events (amounts 14, 15, 15 and 14, the add of offset `0/1` to `127/5000` being `max(bits(127) + bits(1), bits(0) + bits(5000)) + 1 = 15`), target admission and retention; then `incomplete { limit_kind: unit_edges, limit: 1, consumed: 1, next_charge: 2, charge_point: unit.edge }`, because both edges are charged before the first rational event |
| U16 | Evaluate U07 under `L(15,5,1,1,6,1)`; repeat `exact` and then `nearest-even` under `L(15,5,1,1,4,1)` | `exact` refuses after one read, one edge and two rational events (four work units), before `unit.target-domain`; `nearest-even` returns U07's value after `unit.target-domain` with `integer_bits = sbits(127,2) = 14` and `decimal_digits = sdigits(127,2) = 5`; under the second tuple `exact` is still refused, not incomplete, while `nearest-even` returns `incomplete { limit_kind: work_units, limit: 4, consumed: 4, next_charge: 1, charge_point: unit.target-domain }` |
| U17 | Convert `3 m` into U08's target under `L(2,1,0,1,2,0)`, then under `L(2,1,0,1,1,0)` | refused by membership after `unit.target-domain` and before `unit.result-retain`; then `incomplete { limit_kind: work_units, limit: 1, consumed: 1, next_charge: 1, charge_point: unit.target-domain }` |
| U18 | Convert `1 m` into target `Decimal[0,1;0,4294967295;exact]` under `L(18446744073709551615,64,0,1,2,1)` | `incomplete { limit_kind: decimal_digits, limit: 64, consumed: 0, next_charge: 4294967296, charge_point: unit.target-domain }`; the amount is `sdigits(1,4294967295) = 4294967296`, and the retained coefficient `10^4294967295` is never materialized |
| U19 | Add `2 m + 3 m` under `L(4,0,0,2,5,1)`, then under `L(4,0,0,2,4,1)` | `5 m` after the add event with `max(bits(2) + bits(1), bits(3) + bits(1)) + 1 = 4`; then `incomplete { limit_kind: work_units, limit: 4, consumed: 4, next_charge: 1, charge_point: unit.result-retain }`: the rational target charges `unit.target-domain` with one work unit and no size amount |
| U20 | Under `L(0,0,0,0,0,0)`, add `1 u1 + 2 u1` and multiply `1 u3 * 1 m`; add `1 u2 + 2 u2` under `L(4,0,0,2,5,1)` | `u1` and `u3` are affine by composed offset and return `refused { code: ill_typed }` with no charge; `u2`'s offsets compose to zero, so it is not affine and the sum is exact `3 u2` with no edge |
| U21 | Raise `0 m` to integer power `0` under `L(1,0,0,1,4,1)`; raise `0 m` to `-1` under `L(1,0,0,1,1,0)`, then under `L(1,0,0,1,0,0)` | exact dimensionless `1` after read, power event, target admission and retention; undefined after one `unit.identity-read`; then `incomplete { limit_kind: work_units, limit: 0, consumed: 0, next_charge: 1, charge_point: unit.identity-read }` |
| U22 | Convert `1 N_m` to `J` under `L(0,0,0,0,0,0)`; convert compound `4 m^2` to `m2` under `L(3,0,0,1,3,1)`; convert compound `4 m^2` to `cm2` under `L(17,0,1,1,6,1)`, then under `L(16,0,1,1,6,1)` | `refused { code: ill_typed }` with no charge, because the dimension nodes differ despite equal base-dimension maps; exact `4 m2` after one read, target admission and retention with no edge; exact `40000 cm2` after one read, one reverse edge, its two rational events, target admission and retention; then `incomplete { limit_kind: integer_bits, limit: 16, consumed: 5, next_charge: 17, charge_point: unit.rational-arithmetic }` at the divide event, whose amount is `bits(4) + bits(10000) = 17` after the subtract event's 5 |
| U23 | Order `1 degC < 2 degC` under `L(15,0,2,2,9,1)`, then under `L(15,0,2,2,8,1)`; order `1 rev < 2 rev` and test `1 degC == 1 degC`; under `L(0,0,0,0,0,0)`, order `0 degC < 32 degF` and test `0 degC == 32 degF` | `true` after two reads, two edges (left then right), four rational events (the offset add `1 + 5463/20` has amount `max(bits(1) + bits(20), bits(5463) + bits(1)) + 1 = 15`) and retention, with no `unit.target-domain` or `equality.*` charge; then `incomplete { limit_kind: work_units, limit: 8, consumed: 8, next_charge: 1, charge_point: unit.result-retain }`; `false`, because root values are `-1` and `-2`; `true`; both distinct-unit comparisons return `refused { code: ill_typed }` with no charge |
| U24 | Add `1 cm + 2 cm` under `L(4,0,0,2,5,1)` | exact `3 cm` with no `unit.edge`: two reads, one addition event in `cm`, target admission and retention |
| U25 | Convert `1 N_m` to compound `kg*m^2*s^-2` under `L(1,0,0,1,3,1)`, then convert that result to `J` under the same tuple | both explicit steps are admitted by equal base-dimension maps and each makes one read, target admission and retention with no edge; the result is exact `1 J` |
| U26 | Convert `1 u2` to `u1` under `L(6,0,3,1,12,1)`, then under `L(6,0,2,1,12,1)` | exact `-9 u1` after one read, three edges (`u2` to `u1`, `u1` to `K`, `K` to `u1`), six rational events, target admission and retention; then `incomplete { limit_kind: unit_edges, limit: 2, consumed: 2, next_charge: 3, charge_point: unit.edge }`, because no common-ancestor shortcut applies |
| U27 | Divide `1 cm / 0 cm` under `L(1,0,0,2,2,0)`, then under `L(1,0,0,2,1,0)` | undefined after both `unit.identity-read`s and before any `unit.edge`; then `incomplete { limit_kind: work_units, limit: 1, consumed: 1, next_charge: 1, charge_point: unit.identity-read }` |
| U28 | Multiply `1 cm * 1 in` under `L(20,0,2,2,11,1)`, then under `L(13,0,2,2,11,1)`, then under `L(19,0,2,2,11,1)` | exact `127/500000 m^2` after two reads, the `cm` edge then the `in` edge, the `cm` events then the `in` events, the multiplication event (amounts 8, 9, 14, 15 and `max(bits(1) + bits(127), bits(100) + bits(5000)) = 20`), target admission and retention; then `incomplete { limit_kind: integer_bits, limit: 13, consumed: 9, next_charge: 14, charge_point: unit.rational-arithmetic }` at the right operand's first event; then `incomplete { limit_kind: integer_bits, limit: 19, consumed: 15, next_charge: 20, charge_point: unit.rational-arithmetic }` at the multiplication event |
| U29 | Convert `1 in` into an integer target with domain `[-2,2]` under `exact` and then `nearest-even`, each under `L(15,0,1,1,4,1)`; convert `3 m` into that integer target under `L(2,0,0,1,2,0)`, then under `L(2,0,0,1,1,0)` | `exact` returns `refused` after one read, one edge and two rational events, before `unit.target-domain`; `nearest-even` rounds to `0` and returns `incomplete { limit_kind: work_units, limit: 4, consumed: 4, next_charge: 1, charge_point: unit.target-domain }`; `3 m` is refused by integer-domain membership after `unit.target-domain` and before `unit.result-retain`; then `incomplete { limit_kind: work_units, limit: 1, consumed: 1, next_charge: 1, charge_point: unit.target-domain }` |

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
