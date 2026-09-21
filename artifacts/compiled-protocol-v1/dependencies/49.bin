---
id: FR-057
title: "Enforce commit boundaries and authored recovery relations"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-006
    type: implements
  - target: ix://agent-ix/quire-specification/FR-056
    type: depends_on
---
## Description

When evaluating compensation, the conformance evaluator SHALL check authored commit ordering and decide recovery only through the compensation's exact typed recovery relation.

## Inputs

A registered compensation obligation, commit observations, compensation
attempt/effect observations, immutable captures and an authored recovery
predicate evaluated under the selected state/temporal profiles.

## Outputs

A compensation obligation result with separate attempt, effect, commit and
recovery facts, or a typed violation/incomplete outcome.

## Behavior

The evaluator SHALL reject or violate registration after a forbidding commit,
compensation before the paired effect and compensation after a forbidding commit.
The evaluator SHALL NOT infer recovery success from compensation-operation success.
The evaluator SHALL retain the target and achieved states used by the recovery relation.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-057-AC-1 | A full refund effect before commit satisfies a full-restoration relation only when the captured amount and target state match. | Test (TC-057) |
| FR-057-AC-2 | A successful partial refund can satisfy an authored partial-recovery relation while failing full restoration. | Test (TC-057) |
| FR-057-AC-3 | Registration after commit and compensation before the forward effect or after a forbidding commit are distinct ordering violations. | Test (TC-057) |
| FR-057-AC-4 | Missing state, commit or compensation-effect inputs yield incomplete/pending rather than recovery success. | Test (TC-057) |

## Dependencies

- [FR-056](./FR-056-register-compensation.md).
- Agent A's state predicates and E's temporal/closure semantics.
