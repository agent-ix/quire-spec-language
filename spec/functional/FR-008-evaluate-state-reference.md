---
id: FR-008
title: "Evaluate the admitted state reference semantics"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-003"
    type: traces_to
  - target: "ix://agent-ix/quire-specification/FR-006"
    type: traces_to
  - target: "ix://agent-ix/quire-specification/FR-012"
    type: traces_to
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

Preconditions read the selected pre observation; parameters retain their
invocation-supplied values across observation selection. Implication entry/value
events retain exact authored correspondence, including partial observations from
an incomplete evaluation. The referenced standard requirements remain drafts
until the shared review and adoption gates are satisfied.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-008-AC-1 | The healthy parent-order case yields true. | Test |
| FR-008-AC-2 | The violating parent-order case yields false. | Test |
| FR-008-AC-3 | A self-loop satisfies one-or-more-edge reachability. | Test |
| FR-008-AC-4 | Zero evaluation budget receives resource_exhausted. | Test |
| FR-008-AC-5 | Cancellation yields no Boolean value. | Test |
| FR-008-AC-6 | The precondition and immutable-parameter examples in ix://agent-ix/quire-specification/FR-012 produce their stated values only after model linking and runtime validation succeed. | Test |
| FR-008-AC-7 | A false implication antecedent records its entry and value without a consequent-entry event. | Test |
| FR-008-AC-8 | A completed true antecedent followed by consequent evaluation records the corresponding entries in evaluation order. | Test |
| FR-008-AC-9 | Exhaustion before consequent entry preserves the completed true-antecedent events and returns no implication Boolean. | Test |
| FR-008-AC-10 | Every emitted implication event identifies the exact authored source binding and operand region. | Test |

## Dependencies

- [US-003](../usecase/US-003-evaluate-bounded-state.md) supplies the user need.
- [Detailed contract or implementation evidence](../../README.md) supplies the scoped context.

## Status

Draft. Specification review and prerequisite acceptance remain distinct from existing code/tests. No acceptance criterion is claimed satisfied solely because this artifact has been authored.
