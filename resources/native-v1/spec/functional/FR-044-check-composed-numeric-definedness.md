---
id: FR-044
title: "Establish exact numeric definedness in the selected domain"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
---
## Description

When checking an admitted numeric expression, the checker SHALL establish that each potentially evaluated result lies in its selected exact authored domain before permitting evaluation.

## Inputs

Named integer/rational types with explicit bounds and units, expression operations, unique literal contexts, sound guard facts and checking resource controls.

## Outputs

A checked defined expression, a typed type/domain/definedness refusal, or an explicit resource-incomplete checking result.

## Behavior

Ordinary operands/results share their named numeric type and exact unit. The state contract specifies permitted operations, rational normalization-before-bounds and sum's explicit result-domain transfer. No float, saturation, machine-width inference, implicit conversion or silent widening is permitted. Failure to establish totality is reported as unproved definedness, distinct from a proved type/domain error or observed logical violation. Checking may be conservative but cannot certify unproved operations or label its limitation a counterexample.

Rational division additionally requires a proved nonzero divisor, exact quotient
normalization and result bounds in the same named dimensionless rational type.
Integer division/remainder remain refused. The literal constructor does not
implicitly convert integer operands or introduce quotient rounding.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-044-AC-1 | An unguarded possible out-of-range result refuses; a sound guard narrowing every potentially evaluated result within inclusive bounds permits checking. | Test (TC-044) |
| FR-044-AC-2 | Normalized rational operation results are checked after sign/gcd reduction, including raw intermediates above normalized bounds, without floating approximation. | Test (TC-044) |
| FR-044-AC-3 | Wrong named types/units, implicit integer-to-rational conversion and prohibited division/remainder refuse, including in unreachable syntax. | Test (TC-044) |
| FR-044-AC-4 | A minimum-only integer remains unadmitted without inventing a maximum; signed-64 boundary overflow cannot wrap or widen. | Test (TC-044) |
| FR-044-AC-5 | Insufficient proof capability is identified as unproved definedness, and exhausted checking resources as incomplete; neither is a completed Boolean violation. | Test (TC-044) |
| FR-044-AC-6 | For a dimensionless rational type with normalized numerator [-1,1] and denominator [1,2], (1/2) divided by (1/2) admits as 1/1 after reducing raw 2/2, while 1 divided by (1/2) refuses its result bound and division by zero refuses definedness. | Test (TC-044) |
| FR-044-AC-7 | An exact nonzero guard permits an otherwise total rational quotient only where that operand's guard fact holds; an unguarded possible zero divisor or a guard for another value cannot discharge the obligation. | Test (TC-044) |

## Dependencies

- [Detailed draft contract](../../proposals/quire-v1/state-contract.md).
- [Shared grammar](../../proposals/quire-v1/shared-grammar.md).
- [Planned acceptance matrix](../composed-foundation/tests.md).

Proposed composed-v1 obligation. Historical definitions and their acceptance
remain separate; this artifact does not establish implementation coverage.
