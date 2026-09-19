---
id: FR-140
title: "Evaluate exact decimal values"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-010
    type: implements
  - target: ix://agent-ix/quire-specification/AD-005
    type: references
---

# FR-140: Evaluate exact decimal values

## Description

When checking a decimal expression, the semantic kernel SHALL represent every
value as an exact signed coefficient and declared nonnegative scale.

## Inputs

Decimal types with coefficient/scale bounds, literals, operands and explicit
conversion requests.

## Outputs

A completed exact/rounded decimal with optional typed loss, or an undefined,
refused or incomplete evaluator outcome. I13 dispositions are not evaluator
outputs.

## Behavior

Literal spelling determines an exact coefficient/scale before normalization.
Equality compares mathematical values; declared representation constraints
remain validation conditions. Arithmetic derives exact intermediate scales and
requires a non-`exact` rounding mode only at a rounding step, when the exact
value is not an integer multiple of `10^-T` for target scale `T`; a value
outside the declared range refuses under every mode. No binary float is used as
an intermediate.

An omitted rounding spelling selects strict `exact`. A successful rounded
operation carries `DecimalLoss { exact_numerator, exact_denominator,
rounded_coefficient, rounded_scale, mode }`; the exact rational difference is
therefore reconstructable and a message string is never the loss authority.
The exact rational is canonical: its denominator is positive, numerator and
denominator have greatest common divisor one, and zero is exactly `0/1`.
`exact_numerator/exact_denominator` is the exact mathematical value of the
operation before rounding, independent of the target scale; it is neither the
discarded difference nor a scaled coefficient-space intermediate.
Strict `exact`, division by zero, domain refusal and resource exhaustion carry
no decimal value and remain distinct outcomes.

## Decimal domain and operations

A decimal representation is (`coefficient: Integer`, `scale: u32`) and denotes
`coefficient × 10^-scale`. Mathematical normalization removes trailing decimal
zeros while `scale > 0`; every zero normalizes to (`0`, `0`). Equality and
ordering use normalized mathematical values. Source and result provenance retain
the pre-normalized coefficient/scale.

In `Decimal[lo, hi; smin, smax; mode]`, `lo` and `hi` are the inclusive
coefficient bounds, `smin` and `smax` are the inclusive scale bounds and `mode`
is the optional rounding spelling. The type is well formed only when
`lo <= hi`, `smin <= smax` and `smax <= u32::MAX`; any other declaration refuses
type checking with `refused { code: ill_typed }`. For a value `v` with
normalized representation (`c`, `s`), the membership scale is
`s* = max(s, smin)` and the membership coefficient is the exact integer
`c* = v × 10^s*`; `v` is a member exactly when `s* <= smax` and
`lo <= c* <= hi`. Membership is therefore a function of the mathematical value
only: equal values have equal membership, and a pre-normalized coefficient or
scale never decides it. This differs from `Rational` bounds, which check the
normalized numerator and denominator, because a decimal scale is
representational: `Decimal[0,10000;2,2]` admits `1` and `1.1` as `1.00` and
`1.10`. Membership is decided without materializing `c*` (for example by
comparing digit counts and exact magnitudes), makes no accounting charge and is
total for every well-formed `smin` and `smax` up to `u32::MAX`. The target
scale of a result is `smax`.

Each operand's retained representation is its literal coefficient/scale as
written or, for a computed result, (`v × 10^T`, `T`) at that result's target
scale `T`; operations and accounting use retained representations, never a
normalized form. Addition/subtraction align to the greater operand scale using exact powers of
ten. Multiplication multiplies coefficients and adds scales. Unary negation
negates the coefficient. Division by a divisor whose normalized coefficient is
zero is undefined. Otherwise division into target scale `T` forms the exact
rational `N/D`, where `N = dividend coefficient × 10^max(0, T + divisor scale -
dividend scale)` and `D = divisor coefficient × 10^max(0, dividend scale -
divisor scale - T)`; `N/D` is the exact quotient in units of `10^-T`.
A rounding step occurs exactly when the exact mathematical result is not an
integer multiple of `10^-T`, that is, when at least one discarded digit is
nonzero; discarded digits that are all zero are not a rounding step, select no
rounding mode and produce no `DecimalLoss`. At a rounding step, `exact`
refuses, while `toward-zero`, `toward-positive`, `toward-negative`,
`nearest-even` and `nearest-away` produce a coefficient at scale `T`. Nearest
modes compare twice the discarded magnitude to one target unit; ties choose an
even coefficient or the greater absolute coefficient respectively. Every result
is kept at its retained representation (`v × 10^T`, `T`) after any rounding
step and then checked for membership by value; a
nonmember refuses with no value and is never re-rounded to a coarser scale,
clamped or widened.

An explicit conversion of a `Rational[n1,n2;d1,d2]` value with `d2 > 1`, reduced
`n/d`, into `Decimal[c1,c2;s1,s2;m]` is exactly the division of `(n, 0)` by
`(d, 0)` into target scale `T = s2` under mode `m`: `N = n × 10^T`, `D = d`, the
same rounding step, `DecimalLoss` (whose exact value is `n/d`) and membership
rule, and the accounting decimal division schedule. A rational source whose
denominator bound is one converts exactly as the integer `n`. The rounding
conversion is lossy, so FR-149 never admits it as an equality conversion. FR-149
classifies it, and every decimal-to-decimal conversion to a smaller maximum
scale, as a scale reduction (`quire.op.numeric.convert_rounding`), the only
conversion that rounds. A conversion that changes only the admitted range, such
as `Int[0,300]` into `Decimal[0,20000;2,2]`, is an FR-149 range narrowing
(`quire.op.numeric.narrow`): its membership is an FR-146 obligation, and it
never rounds or clamps.

Decimal typing derives no default type. The operands of `+`, `-`, `*`, `/` and
unary `-` and the result have one named `Decimal[..]` type, whose `smax` is the
target scale; operands of distinct decimal types are
`refused { code: ill_typed, cause: type-mismatch }` until an explicit
`convert` selects one. A `decimal(c,s)` literal takes exactly its unique
expected type: the other operand's static type when that operand is not a
literal, a declared field, parameter, result or annotated type of its position,
a `convert` target, or the unique expected type of the enclosing decimal
operation, which that operation passes to its operands. A literal without one, such as `decimal(15,1) +
decimal(225,2)` as an unannotated `let` right side or both operands of `=`, is
`refused { code: ill_typed, cause: ambiguous-literal }`; no coefficient range,
scale, rounding mode or target scale is inferred from the literal spellings.

Under `quire.value.accounting/v1`, the evaluator charges the named decimal
points and counters before a power-of-ten expansion, coefficient operation or
result retention. The first unavailable charge returns incomplete accounting
without attempting a smaller or floating approximation. The zero-divisor
undefined check follows `decimal.operands` and precedes
`decimal.scale-expansion`; strict `exact` refusal at a rounding step follows
`decimal.arithmetic` and precedes `decimal.rounding`; membership refusal
follows `decimal.rounding`, or `decimal.arithmetic` when no rounding step
occurs, and precedes `decimal.result-retain`. Undefined and refused outcomes
make no later charge.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-140-AC-1 | `1.0` and `1.00` compare mathematically equal while retaining their declared representation provenance. | Test (TC-185) |
| FR-140-AC-2 | Exact representable arithmetic returns the mathematical result without rounding. | Test (TC-185) |
| FR-140-AC-3 | A result that is not an integer multiple of `10^-T` refuses under `exact`, while a non-`exact` mode returns the rounded value plus a canonical-rational loss record; a result outside the declared range refuses under every mode. | Test (TC-185) |
| FR-140-AC-4 | The six rounding modes produce their declared result on positive and negative half-way values and `exact` refuses every nonzero discarded digit. | Test (TC-185) |
| FR-140-AC-5 | Division by zero is undefined, while a value whose membership coefficient `c*` or membership scale `s*` is outside the target domain is refused; neither produces a value or uses a floating intermediate. A declaration with `lo > hi`, `smin > smax` or `smax > u32::MAX` refuses type checking with `ill_typed`. Membership charges nothing and is total up to scale `u32::MAX`. | Test (TC-185) |
| FR-140-AC-6 | Exact-bound accounting succeeds and denial of a named next decimal charge returns incomplete without a value or an implementation-specific retry; zero-divisor, strict-`exact` and membership outcomes occur at their defined charge positions, and all-zero discarded digits make no `decimal.rounding` charge. | Test (TC-185) |
| FR-140-AC-7 | A `Rational` to `Decimal` conversion follows the decimal division rule and schedule at target scale `smax`, decimal operands and result share one named type, and a `decimal(c,s)` literal without a unique expected type is `ill_typed` with cause `ambiguous-literal`. | Test (TC-185) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.
