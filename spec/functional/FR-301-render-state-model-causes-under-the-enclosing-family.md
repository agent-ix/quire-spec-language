---
id: FR-301
title: "Render StateModel causes raised under an enclosing family's evaluation"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-151
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
---
# FR-301: Render StateModel causes raised under an enclosing family's evaluation

## Description

When a model form evaluates inside a `Value` function body or a
`ProtocolClause` body at S6a and raises a model refusal or a model undefined
cause, the evaluator SHALL report that cause as a `StateModel`-owned cause,
rendered with the `state-model` catalog prefix, inside the enclosing family's
result.

## Inputs

- A model form (`deref` (a field read), `allInstances`, `lookup`, a
  dispatched call) evaluated through `value::model_query` from the `evaluate` hook of the
  enclosing `Value` or `ProtocolClause` family (ADR-016 FP-3).
- The admitted population binding and object environment
  ([FR-084](FR-084-admit-closed-populations-and-resolve-lookup.md)).

## Outputs

The enclosing family's `FamilyOutcome`, whose `FamilyEvaluated` result
carries the `StateModel` cause unchanged.

## Behavior

The evaluator SHALL carry each model cause ADR-013 O-16 assigns to
`StateModel` (for example `PreconditionFalse`, the `absent refused` `lookup`
refusal, and the `ancestor-steps` resource refusal of ADR-016 SC-6) in the
enclosing family's result as a `StateModel` cause. Its catalog code SHALL
render with the `state-model` prefix and never with the enclosing family's
prefix. Its O-16 category SHALL be the category of the `FamilyResult` arm that
carries it (`Refused` → refusal, `Undefined` → undefined), as ADR-012 §13.5
Q210-3 maps it. The S6a family kind is unchanged: the cause travels in the
enclosing family's result.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-301-AC-1 | A `Value` function whose body performs `lookup<T>(p, r)` with an `absent refused` key evaluates to a `FamilyResult::Refused` whose cause renders with the `state-model` prefix, not the `value` prefix, and whose category is refusal. | Test (TC-792) |
| FR-301-AC-2 | A state clause whose body makes a dispatched call whose selected precondition is false evaluates to `FamilyResult::Undefined` with cause `precondition-false` rendered with the `state-model` prefix, not the `protocol-clause` prefix, and category undefined. | Test (TC-792) |

## Dependencies

- **Upstream:** ADR-016 FP-3 and ADR-013 O-16 place the model causes in
  `StateModel` and their evaluation in the enclosing family;
  [FR-090](FR-090-return-a-family-outcome-or-a-typed-family-refusal.md) defines
  `FamilyOutcome`; QSpec FR-151 and FR-153 fix the cause names.
- **Downstream:** replay and run reports read the rendered code.

## References

- ADR-016 §9 G-2c.
- Linear QSL-382 (specification), QSL-68 (implementation).
