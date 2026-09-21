---
id: FR-058
title: "Preserve retry, duplicate and partial-recovery outcomes"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-006
    type: implements
  - target: ix://agent-ix/quire-specification/FR-057
    type: depends_on
---
## Description

When multiple compensation observations relate to one obligation, the conformance evaluator SHALL preserve each attempt and effect identity and evaluate only the authored retry and recovery relations.

## Inputs

A registered compensation obligation, bounded retry policy, compensation
attempts/effects and any declared finite related-effect population.

## Outputs

Per-attempt and aggregate recovery facts with explicit failed, timed-out,
duplicate, pending and successful outcomes.

## Behavior

The evaluator SHALL distinguish a repeated receipt, retried attempt and duplicate
business effect. It SHALL refuse attempts beyond the authored finite retry bound.
Where recovery aggregates related effects, it SHALL require the exact declared
population scope, membership and completeness authorities before success.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-058-AC-1 | A failed refund attempt followed by one successful retry retains both attempts and one successful refund effect. | Test (TC-058) |
| FR-058-AC-2 | A timed-out attempt, failed attempt and duplicate effect remain three distinct outcomes. | Test (TC-058) |
| FR-058-AC-3 | Two related partial refunds satisfy the captured amount only under a complete declared population with exact arithmetic. | Test (TC-058) |
| FR-058-AC-4 | Unknown or missing population membership and attempts beyond the retry bound cannot produce recovery success. | Test (TC-058) |

## Dependencies

- [FR-057](./FR-057-enforce-commit-recovery.md).
- D-owned relationship/population identity and F-owned completeness.
