---
id: FR-039
title: "Normalize exact rational literals before domain admission"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
---
## Description

When admitting an exact rational literal, the checker SHALL normalize its signed integer numerator and denominator before checking the selected authored rational domain.

## Inputs

The rational(n,d) literal form with signed integer constants, its unique expected imported rational type, source identity and explicit parsing/arithmetic resource limits.

## Outputs

An exact normalized value in its authored domain, or a typed zero-denominator, domain/type, syntax or resource refusal.

## Behavior

Reject denominator zero before normalization. Move the sign to the numerator, divide both components by their greatest common divisor, and normalize any zero numerator to 0/1. Only normalized components are checked against the authored numerator/denominator bounds. The literal is not a runtime conversion or a user-defined function; no floating point or implicit integer-to-rational conversion occurs. Equivalent values preserve distinct original source spellings/identities. Integer slash/div/rem/mod remain separate recognized but refused forms under the carried-forward arithmetic admission.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-039-AC-1 | rational(2,4) and rational(-2,-4) normalize to 1/2; rational(0,-7) normalizes to 0/1, with each original source retained. | Test (TC-039) |
| FR-039-AC-2 | rational(1,0) and rational(0,0) refuse with zero-denominator cause before any predicate result. | Test (TC-039) |
| FR-039-AC-3 | For an explicitly authored normalized numerator range [-1,1] and denominator range [1,2], rational(2,4) admits, rational(2,1) and rational(1,3) refuse, and all inclusive boundary values admit. | Test (TC-039) |
| FR-039-AC-4 | No expected rational type, ambiguous expected types, a runtime-expression argument or implicit integer conversion refuses rather than choosing a domain. | Test (TC-039) |
| FR-039-AC-5 | Normalization work exhaustion produces resource incompleteness, never a floating approximation, domain widening or Boolean result. | Test (TC-039) |

## Dependencies

- [Proposed shared grammar](../../proposals/quire-v1/shared-grammar.md).
- [Shared drafting foundation](../../proposals/quire-v1/shared-foundation.md).
- [Planned matrix](../composed-foundation/tests.md).

This contribution remains proposed until the integrated baseline review. It does
not change historical definition bytes or claim current compiler support.
