---
id: FR-148
title: "Evaluate explicit IEEE floating-point profiles"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-010
    type: implements
  - target: ix://agent-ix/quire-specification/AD-005
    type: references
  - target: ix://agent-ix/quire-specification/FR-044
    type: references
---

# FR-148: Evaluate explicit IEEE floating-point profiles

## Description

When a source selects an IEEE floating-point profile, the evaluator SHALL use
its declared width, rounding mode and exceptional-value comparison policy.

## Inputs

Binary width, rounding mode, bit-pattern values and arithmetic/comparison
operations.

## Outputs

A completed profile bit-pattern/flag result or Boolean comparison, or an
undefined, refused or incomplete evaluator outcome. I13 `unsupported` and
`requires-bound` remain separate provider-negotiation dispositions.

## Behavior

NaN payloads, infinities and signed zero remain representable. Numeric equality,
total ordering and bitwise identity are distinct selected operations. Each
arithmetic step rounds once under the declared mode. Cross-width or exact-number
conversion is explicit and reports loss. The checked package selects
`quire.value.ieee754-2019-default/v1`; absence, a different version or multiple
IEEE policy selections refuses semantic admission.

## IEEE profile

Complete V1 uses the IEEE 754-2019 binary32 and binary64 interchange formats.
Values may be constructed from exact bits, including every NaN payload and sign.
The required operations are addition, subtraction, multiplication, division,
square root and fused multiply-add under `nearest-even`, `nearest-away`,
`toward-zero`, `toward-positive` and `toward-negative` rounding. Fused
multiply-add performs one rounding after the unrounded product and sum.

The grammar's `exact` spelling, including an omitted optional rounding spelling,
is a strict language policy rather than a sixth IEEE rounding direction. The
evaluator computes the exact real intermediate and succeeds only when the
selected width represents it without discarded information. An inexact, overflow
or tiny-and-inexact result is refused with its would-be flags and no rounded
substitute. The refusal carries only the would-be flag set, never would-be bits,
and those flags are the flags the same operation would return under
`nearest-even`, so overflow and tininess follow that direction. Inexactness at
an IEEE width is decided by the fixed-width rounding step, so a strict `exact`
refusal follows `ieee.round` and precedes `ieee.result-retain`; decimal and unit
targets instead decide inexactness analytically before their sizing charge.
Exact subnormal results remain admissible. Invalid operations and division by
zero retain the deterministic IEEE exceptional result and flag defined below
because they do not substitute a rounded finite value.

Numeric equality, IEEE `totalOrder` and bit identity are distinct operations.
When an arithmetic operation must produce a quiet NaN from non-NaN operands,
the profile uses canonical quiet-NaN bits `0x7fc00000` for binary32 and
`0x7ff8000000000000` for binary64. With one or more NaN operands, the result
uses the leftmost NaN operand, preserves its sign and payload, and sets its quiet
bit. Consuming any signaling NaN sets `invalid`; quieting it does not discard
the payload. An invalid operation with no NaN operand returns the positive
canonical quiet NaN. These rules apply in operand order to fused multiply-add.

Each operation returns a fresh closed flag set drawn from `invalid`,
`divide_by_zero`, `overflow`, `underflow` and `inexact`; there is no implicit
process-global sticky state. Overflow also sets `inexact`. Underflow uses
tininess after rounding and is set only when the rounded result is tiny and
inexact. Division of a finite nonzero value by zero sets `divide_by_zero` and
returns the signed infinity. Exact subnormal production sets neither `underflow`
nor `inexact`.

A zero result or zero source keeps its sign per IEEE 754-2019 §6.3 in
arithmetic, fused multiply-add, square root and width conversion. The sign of a
product or quotient is the exclusive or of the operand signs, and the square
root of a zero is that zero. A sum of operands with opposite signs, or a
difference of operands with like signs, whose exact result is zero (zeros or
cancelling nonzero operands) is `+0` under every rounding direction and strict
`exact` except `toward-negative`, where it is `-0`. Under every direction, when
`x` is zero, `x + x` and `x - (-x)` keep the sign of `x`; a sum of zeros with
like signs or a difference of zeros with opposite signs is therefore that sign.
Fused multiply-add `fma(x, y, z)` applies the same sum rule to `x × y`, whose
sign is the exclusive or of the signs of `x` and `y`, and `z`: an exact zero
result from operands of opposite signs is `+0` except under `toward-negative`,
where it is `-0`, and one from operands of like signs keeps that sign. A nonzero
result that rounds to zero keeps the sign of the exact result.

`numericEqual` is false if either operand is NaN and treats positive and
negative zero as equal. `bitIdentical` requires the same width and identical
bits. `totalOrder` requires the same width and orders the unsigned bit pattern
`b` by key `not(b)` when its sign bit is one and by `b xor sign_mask` otherwise;
this fixes the order of signed zeros, infinities and every signaling/quiet NaN
payload. The exact qualified intrinsic identities are
`quire::value::ieee::sqrt`, `quire::value::ieee::fma`,
`quire::value::ieee::numericEqual`, `quire::value::ieee::totalOrder` and
`quire::value::ieee::bitIdentical`.

Every IEEE operation or comparison whose IEEE operands differ in width, or that
mixes an IEEE operand with an exact type, is refused by type checking with
`refused { code: ill_typed }` before any charge, until an explicit conversion
selects a common type. A user declaration bound to a reserved qualified
intrinsic identity refuses package admission as `refused { code:
invalid_package, cause: conflicting-definition }`.

Explicit width conversion of a finite value, zeros included, forms the source's
exact real and rounds it once to the target width with the selected policy and
flags; a zero keeps its sign. Width conversion of an infinity keeps its sign and
sets no flag. Width conversion of a NaN keeps its sign and its integer payload
below the quiet bit, sets the quiet bit and, when the source is signaling, sets
`invalid`, as for every consumed signaling NaN. A payload not smaller than the
target width's quiet bit is refused as `refused { code:
ieee_nan_payload_not_representable }` with no flags, even for a signaling
source, because the refusal precedes consuming the NaN. This is a
language-policy refusal: IEEE convertFormat is total, and this profile refuses
rather than truncating a payload.

Explicit conversion of an IEEE value to an exact number targets a grammar-named
`Rational[lo, hi; dmin, dmax]` type. A NaN or infinity is undefined. A finite
value, either zero included, forms its exact reduced rational, with negative
zero becoming zero. Because the IEEE operand is a runtime bit pattern, this
conversion is exempt from FR-044's static result-bound proof: membership is
decided at evaluation, where the reduced numerator must lie in `[lo, hi]` and
the positive denominator in `[dmin, dmax]`. Membership charges nothing, and a
non-member returns the runtime refusal `refused { code:
ieee_rational_out_of_domain }` after `ieee.exact-intermediate` and before
`ieee.result-retain`. FR-148 defines no conversion from an IEEE value to
`Decimal`, `Integer` or `Int[..]`; a direct IEEE conversion to those types is
refused `ill_typed` with no charge, and an IEEE value reaches exact arithmetic
only as `Rational[..]`. Conversion of a `Rational[..]` quantity into a decimal
or integer target remains governed by FR-142 and is unaffected. Explicit
conversion of any exact integer, rational or decimal value to an IEEE width
rounds its exact value once under the selected policy, with strict `exact`
refusing as for arithmetic; a decimal source is sized by its retained
coefficient and scale, not by its reduced form.

Before decoding retained operands or constructing an exact-real intermediate,
the evaluator charges only the named IEEE points and counters from
`quire.value.accounting/v1`. An unavailable charge returns incomplete without a
partial bit pattern or a host-float retry. I13 negotiation for a backend missing the
selected width, operation, rounding direction, NaN/flag policy or resource proof
returns a per-item `unsupported` or `requires-bound` disposition without
changing package/profile admission and without substituting decimal, rational or
host floating point.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-148-AC-1 | NaN, infinity and signed-zero vectors produce the selected equality and ordering results. | Test (TC-193) |
| FR-148-AC-2 | Changing rounding mode can change the bit-pattern result and therefore changes run provenance. | Test (TC-193) |
| FR-148-AC-3 | I13 negotiation for a backend lacking the selected width, rounding or exceptional-value policy returns `unsupported` or `requires-bound` without changing package admission or substituting decimal/rational semantics. | Test (TC-193) |
| FR-148-AC-4 | A fused multiply-add vector demonstrates its single-rounding result separately from a multiply followed by add. | Test (TC-193) |
| FR-148-AC-5 | The same NaN and signed-zero bit vectors distinguish numeric equality, total ordering and bit identity. | Test (TC-193) |
| FR-148-AC-6 | Signaling/quiet NaN propagation, invalid/divide-by-zero/overflow/underflow/inexact flags and strict `exact` refusal follow the closed profile without global state. | Test (TC-193) |
| FR-148-AC-7 | A concrete binary64 limit tuple succeeds, while denial of the named final IEEE charge returns incomplete without bits or flags. | Test (TC-193) |
| FR-148-AC-8 | Mixed-width or IEEE-with-exact operations and comparisons refuse `ill_typed` before any charge; zero operands take the finite charge path with IEEE signed-zero results; strict `exact` refuses after `ieee.round` with `nearest-even` would-be flags and no bits. | Test (TC-193) |
| FR-148-AC-9 | Width, IEEE-to-exact and exact-to-IEEE conversions charge the IEEE points at their stated sizes and positions; NaN width conversion keeps sign and payload, or refuses an unrepresentable payload after `ieee.operands` and before `ieee.result-retain` with no flags. | Test (TC-193) |
| FR-148-AC-10 | A user declaration bound to a reserved qualified intrinsic identity returns `refused { code: invalid_package, cause: conflicting-definition }` at package admission. | Test (TC-193) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.
