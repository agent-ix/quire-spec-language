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
requires an explicit rounding rule only when the result type cannot represent
the exact value. No binary float is used as an intermediate.

An omitted rounding spelling selects strict `exact`. A successful rounded
operation carries `DecimalLoss { exact_numerator, exact_denominator,
rounded_coefficient, rounded_scale, mode }`; the exact rational difference is
therefore reconstructable and a message string is never the loss authority.
The exact rational is canonical: its denominator is positive, numerator and
denominator have greatest common divisor one, and zero is exactly `0/1`.
Strict `exact`, division by zero, domain refusal and resource exhaustion carry
no decimal value and remain distinct outcomes.

## Decimal domain and operations

A decimal representation is (`coefficient: Integer`, `scale: u32`) and denotes
`coefficient × 10^-scale`. Mathematical normalization removes trailing decimal
zeros while `scale > 0`; every zero normalizes to (`0`, `0`). Equality and
ordering use normalized mathematical values. Source and result provenance retain
the pre-normalized coefficient/scale.

Addition/subtraction align to the greater operand scale using exact powers of
ten. Multiplication multiplies coefficients and adds scales. Unary negation
negates the coefficient. Division first forms the exact rational quotient and
is defined only when the divisor is nonzero and the result is finite-decimal in
the target scale, or when the target selects one of `toward-zero`,
`toward-positive`, `toward-negative`, `nearest-even` or `nearest-away`.
`exact` refuses any discarded nonzero digit. Nearest modes compare twice the
discarded magnitude to one target unit; ties choose an even coefficient or the
greater absolute coefficient respectively. Every result is checked against the
declared coefficient and scale bounds after rounding and normalization.

Under `quire.value.accounting/v1`, the evaluator charges the named decimal
points and counters before a power-of-ten expansion, coefficient operation or
result retention. The first unavailable charge returns incomplete accounting
without attempting a smaller or floating approximation.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-140-AC-1 | `1.0` and `1.00` compare mathematically equal while retaining their declared representation provenance. | Test (TC-185) |
| FR-140-AC-2 | Exact representable arithmetic returns the mathematical result without rounding. | Test (TC-185) |
| FR-140-AC-3 | An unrepresentable result without a selected rounding rule refuses; an explicit rule returns the rounded value plus a canonical-rational loss record. | Test (TC-185) |
| FR-140-AC-4 | The six rounding modes produce their declared result on positive and negative half-way values and `exact` refuses every nonzero discarded digit. | Test (TC-185) |
| FR-140-AC-5 | Division by zero is undefined, while a normalized coefficient/scale outside the target domain is refused; neither produces a value or uses a floating intermediate. | Test (TC-185) |
| FR-140-AC-6 | Exact-bound accounting succeeds and denial of a named next decimal charge returns incomplete without a value or an implementation-specific retry. | Test (TC-185) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.
