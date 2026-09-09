---
id: FR-008
title: "Evaluate the admitted state reference semantics"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-006
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-012
    type: traces_to
---
# FR-008: Evaluate the admitted state reference semantics

## Description

When a validated context is supplied, the reference evaluator shall execute its selected native clause under the admitted state semantics.

## Inputs

A borrowed ValidatedContext, caller-lowered EvaluationLimits and a cancellation
poll. The original native AST and CheckedClause types belong to the retained
CheckedPackage. There is no evaluation entry point for unchecked snapshots.

## Outputs

A source/model/input-bound report with Completed(Boolean), Incomplete(stop) or
Refused(runtime_invariant), actual usage and ordered implication events.
No failed execution produces a logical value. B owns portable result adaptation.

## Behavior

[The evaluation contract](../../docs/native-runtime-evaluation.md) fixes exact
observation capture, immutable values, evaluation order, eligible comparisons,
checked integer arithmetic, sequence quantification, graph expansion and events.
It uses the adopted standard at e897f81. Native execution evaluates the source
AST, independently of the IR proof view or generated backend code.

The selected accounting version is native-ref-cost/1-draft: one step per entered
non-group expression and one per first-expanded object in each reaches call.
Exact independent vectors in the contract are mandatory qualification cases.
[NFR-006](../non-functional/NFR-006-bound-native-runtime.md) separately limits
deep comparisons, text work, events and active depth. Those implementation work
limits do not introduce a domain or path-length bound.

Unsupported profile forms continue to refuse at the existing frontend, including
collect, helpers, recursion and temporal syntax. Unavailable backend probes remain
separate from actual reference events. A completed concrete judgment does not
establish all-input validity or backend cost parity.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-008-AC-1 | The healthy parent-order case yields true. | Test |
| FR-008-AC-2 | The violating parent-order case yields false. | Test |
| FR-008-AC-3 | A self-loop satisfies one-or-more-edge reachability. | Test |
| FR-008-AC-4 | Zero expression budget receives resource_exhausted before entering the clause. | Test |
| FR-008-AC-5 | Cancellation yields no Boolean value. | Test |
| FR-008-AC-6 | The adopted precondition and immutable-parameter examples produce their stated values after actual model linking and runtime validation. | Test |
| FR-008-AC-7 | A false implication antecedent records its entry and value without a consequent-entry event. | Test |
| FR-008-AC-8 | A completed true antecedent followed by consequent evaluation records the corresponding entries in evaluation order. | Test |
| FR-008-AC-9 | Exhaustion before consequent entry preserves completed true-antecedent events and returns no implication Boolean. | Test |
| FR-008-AC-10 | Every emitted implication event identifies its exact authored source binding, implication/operand ExprIds and original operand region, including CRLF and multibyte text. | Test |
| FR-008-AC-11 | Generated finite functional graphs agree with an independent transitive-closure oracle, including isolated nodes, self-loops, cycles, disconnected targets and distinct equal-valued objects. | Test |
| FR-008-AC-12 | Checked integer operations produce exact admitted results with division toward zero and signed remainder; an unexpected execution invariant failure produces no Boolean. | Test |
| FR-008-AC-13 | Forall/exists visit sequence occurrences in order with duplicates, short circuit correctly and return true/false respectively for empty sequences. | Test |
| FR-008-AC-14 | Let evaluates its initializer once; local reads, selected conditional values and pre expressions preserve captured observations without deep-copying input values. | Test |
| FR-008-AC-15 | Text ordering uses Unicode scalar lexicographic order; eligible records compare declared fields and object/reference equality compares exact identity excluding observation. | Test |
| FR-008-AC-16 | Every independent accounting vector completes at its exact required expression/graph budgets and stops incomplete one below a nonzero required budget. | Test |
| FR-008-AC-17 | Nested and repeated implications retain actual event order; event-capacity exhaustion stops with the recorded prefix and no Boolean. | Test |
| FR-008-AC-18 | Auxiliary comparison/text/depth limits stop before excess work while leaving expression/graph accounting distinct. | Test |
| FR-008-AC-19 | Repeated runs have fresh budgets, locals, visited sets and events; success cannot make a subsequent smaller-budget run succeed or change input bytes. | Test |
| FR-008-AC-20 | A collect expression with duplicate outputs is refused by the actual frontend without silently deduplicating or reaching reference evaluation. | Test |

## Dependencies

- [FR-007](FR-007-validate-runtime-inputs.md) establishes complete valid inputs.
- [FR-016](FR-016-check-native-clauses.md) establishes conditional definedness.
- [US-003](../usecase/US-003-evaluate-bounded-state.md) supplies the user need.
- [IT-006](../integration/IT-006-native-reference-workflow.md) qualifies this API milestone.
- [IT-002](../integration/IT-002-native-state-workflow.md) retains the full compiled-model/backend objective.

## Status

Reference execution and accounting are qualified at
48f53aed5990f7daf15ae6871c1c081d8974c64f by
[SR-098](../../reviews/26-09-09-native-reference-evaluation.md), covering all
twenty criteria through 29 public evaluator tests and two private invariant controls.
IT-006 is qualified at 4ac3597 by SR-099. Task-015's qualification/handoff is
complete, with final plan audit in SR-100. Backend and Quire integration remain
required. The historical
standard/profile bytes remain unchanged.
