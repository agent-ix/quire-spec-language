---
id: FR-008
title: "Evaluate the admitted state reference semantics"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-003"
    type: implements
---
# FR-008: Evaluate the admitted state reference semantics

## Description

When a checked clause and validated context are supplied, the reference evaluator shall execute the selected state semantics.

## Inputs

Reference-evaluable clause, immutable context and versioned expression/graph budgets.

## Outputs

Completed Boolean and actual activation events, or incompleteness.

## Behavior

Evaluation preserves branch order, evaluate-once let bindings, sequence multiplicity, reference identity and exact finite reachability. Resource ceilings never become domain/path bounds. The initial concrete workflow must include healthy, violating and refused/incomplete cases; authored expectations are not execution evidence.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-008-AC-1 | The healthy parent-order case yields true. | Test |
| FR-008-AC-2 | The violating parent-order case yields false. | Test |
| FR-008-AC-3 | A self-loop satisfies one-or-more-edge reachability. | Test |
| FR-008-AC-4 | Zero evaluation budget receives resource_exhausted. | Test |
| FR-008-AC-5 | Cancellation yields no Boolean value. | Test |

## Dependencies

- [US-003](../usecase/US-003-evaluate-bounded-state.md) supplies the user need.
- [Detailed contract or implementation evidence](../../README.md) supplies the scoped context.

## Status

Draft. Specification review and prerequisite acceptance remain distinct from existing code/tests. No acceptance criterion is claimed satisfied solely because this artifact has been authored.
