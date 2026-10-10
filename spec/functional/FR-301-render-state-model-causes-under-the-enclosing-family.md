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
with `state-model` cause-family attribution, inside the enclosing family's
result. Here the prefix denotes the cause's owning family, not a prefix
added to `CatalogCode.code`: a refusal retains the unchanged
`ModelRefusal::catalog_code()` required by FR-090. For absent refused lookup
that code/cause pair is exactly `invalid_runtime_input` / `absent-key`.

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
enclosing family's result as a `StateModel` cause. Its typed cause-family
attribution SHALL be `StateModel`, never the enclosing family; its catalog
code and cause SHALL retain their owning spellings unchanged, without an
added family prefix or fallback. For undefined results the owning
`UndefinedCoded` reason and payload remain unchanged. Its O-16 category
SHALL be the category of the `FamilyResult` arm that
carries it (`Refused` → refusal, `Undefined` → undefined), as ADR-012 §13.5
Q210-3 maps it. The S6a family kind is unchanged: the cause travels in the
enclosing family's result.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-301-AC-1 | A `Value` function whose body performs `lookup<T>(p, r)` with an `absent refused` key evaluates to a `FamilyResult::Refused` carrying the actual StateModel-owned model-query cause, not a Value-owned substitute. Its unchanged catalog code/cause is exactly `invalid_runtime_input` / `absent-key`, its owning binding/key fields and lookup locus are retained, and its category is refusal. | Test |
| FR-301-AC-2 | A state clause whose body makes a dispatched call whose selected precondition is false evaluates to `FamilyResult::Undefined` with StateModel-owned cause `precondition-false`, not a ProtocolClause-owned substitute. Its owning operation, selected method, receiver and call locus remain unchanged, and its category is undefined. | Test |

Both criteria retain TC-792 and its separate Value lookup and dispatched
clause procedures. Exact catalog spelling and typed cause-family ownership
are independent assertions: neither a generic code nor a fabricated
`state-model` code prefix establishes the latter.

## Dependencies

- **Upstream:** ADR-016 FP-3 and ADR-013 O-16 place the model causes in
  `StateModel` and their evaluation in the enclosing family;
  [FR-090](FR-090-return-a-family-outcome-or-a-typed-family-refusal.md) defines
  `FamilyOutcome`; QSpec FR-151 and FR-153 fix the cause names.
- **Downstream:** replay and run reports read the rendered code.

## References

- ADR-016 §9 G-2c.
- Linear QSL-382 (specification), QSL-68 (implementation).
