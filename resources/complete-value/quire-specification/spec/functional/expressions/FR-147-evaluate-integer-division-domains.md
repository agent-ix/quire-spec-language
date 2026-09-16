---
id: FR-147
title: "Evaluate mathematical and bounded integer division"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-010
    type: implements
  - target: ix://agent-ix/quire-specification/FR-007
    type: references
  - target: ix://agent-ix/quire-specification/FR-044
    type: references
  - target: ix://agent-ix/quire-specification/AD-005
    type: references
---

# FR-147: Evaluate mathematical and bounded integer division

## Description

When an integer division or remainder operator is selected, the evaluator SHALL
apply its exact profile-declared quotient/remainder law and result domain.

## Inputs

Integer type, operands, selected Euclidean, floor or truncating division profile and
consumer-domain bounds.

## Outputs

A completed exact quotient/remainder pair, or an undefined, refused or
incomplete evaluator outcome. I13 `requires-bound` remains a separate provider
negotiation disposition.

## Behavior

Mathematical integers are unbounded language values. `div`/`rem` use the
selected law and satisfy its quotient identity; operator spellings cannot mix
laws. Bounded consumers prove operand/intermediate/result membership. Division
by zero is undefined and no backend may saturate or substitute float division.

## Division profiles

For dividend `a`, nonzero divisor `b`, quotient `q` and remainder `r`, every
profile satisfies `a = b*q + r` and the following additional law:

| Profile | Quotient | Remainder constraint |
| --- | --- | --- |
| truncating | `q` is the exact rational quotient rounded toward zero | `abs(r) < abs(b)` and a nonzero `r` has the sign of `a` |
| floor | `q` is the exact rational quotient rounded toward negative infinity | `abs(r) < abs(b)` and a nonzero `r` has the sign of `b` |
| Euclidean | `q` is the unique integer satisfying the remainder constraint | `0 <= r < abs(b)` |

`div` and `rem` are a paired operation under the selected profile. `mod` names
only the Euclidean remainder and cannot inherit a truncating or floor profile.
Mathematical-integer evaluation has no overflow. A bounded consumer checks the
membership of both results, including the signed minimum divided by negative
one case, before it exposes either value.

The checked package retains exactly one of
`quire.value.integer-division.truncating/v1`,
`quire.value.integer-division.floor/v1` or
`quire.value.integer-division.euclidean/v1` for `div`/`rem`; absence or multiple
selections refuses semantic admission. `mod` always selects Euclidean remainder.
The paired quotient/remainder is admitted atomically. Before evaluation, a
finite I13 consumer without complete operand, intermediate and result bounds
returns the per-item `requires-bound` negotiation disposition without narrowing
mathematical integers. That provider disposition is not an evaluator outcome.
Evaluation uses the named integer-division charges and counters from
`quire.value.accounting/v1`; it never publishes only one member of the pair.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-147-AC-1 | Positive and negative operand vectors satisfy the selected quotient/remainder identity exactly. | Test (TC-192) |
| FR-147-AC-2 | Division by zero is undefined and produces no numeric value. | Test (TC-192) |
| FR-147-AC-3 | I13 negotiation for a finite backend lacking a complete derived domain returns `requires-bound` instead of invoking evaluation or narrowing mathematical integers. | Test (TC-192) |
| FR-147-AC-4 | Positive and negative dividend/divisor vectors distinguish truncating, floor and Euclidean results while preserving `a = b*q + r`. | Test (TC-192) |
| FR-147-AC-5 | `mod` returns the Euclidean remainder independently of the selected `div`/`rem` law; an attempted explicit non-Euclidean `mod` selection is refused, while every zero divisor is undefined, without emitting a quotient or remainder. | Test (TC-192) |
| FR-147-AC-6 | Exact-bound accounting succeeds and denial of a named next integer-division charge returns incomplete without either quotient or remainder. | Test (TC-192) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.
