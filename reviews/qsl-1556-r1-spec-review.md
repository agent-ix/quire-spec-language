---
id: SR-20261103155255
title: "QSL-1556 r1 spec review of PR #2205 (FR-517 recipe compensation binder types)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR #2205 head vs origin/main; spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md (recipe table row 3, new paragraph); context: QSL FR-381, FR-383-AC-51, FR-520, examples/protocol-handoff/, examples/checked-handoff/; QSpec FR-464, FR-466, FR-427 (by id)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-517
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: references
---

# QSL-1556 r1 spec review of PR #2205

## Summary

Ticket: QSL-1556. The PR changes one FR-517 recipe-table row and adds one
paragraph. It records that the legacy recipe typed the compensations'
`attempts`, `retry` and `recover` binders with the object `Workflow`, and that
**R** uses the record value types `Attempt` and `Recovery` in their place.

Citations checked:

- QSL FR-381 "Activation, attempts and deadline": `T` that is not a record or
  event type refuses `invalid_model_binding`/`malformed-declaration`.
  FR-381-AC-13 names an object type. Accurate.
- QSL FR-381 "Names a stage may read" and FR-381-AC-14: the retry relation and
  recovery predicate bar model state, and a field read of an object-typed
  binder is a model-state read refused `ill_typed`/`operator-ineligible`.
  Accurate.
- QSpec FR-466 reference check 1: `record_type` must name a
  `record_value_type` or `event_type`. QSpec FR-464 "Slots that bar model
  state" lists the retry relation and the recovery predicate. Accurate.
- Legacy `examples/protocol-handoff/workflow.body.native:16-22, 36-42`: all
  three binders are `M::Workflow`. R `examples/checked-handoff/unit-event-position.native:41-47, 61-67`:
  `M::Attempt` and `M::Recovery`, both `record` in `model.semantic-ir.json`.
  Accurate.

No recipe element is deleted from the FR-517 table by this PR.

## Verdict

The decision is correct: the spine is right and the recipe text was wrong.
The edited row's R cell still calls `Workflow` and `Provider` records, though
R declares both as object types (one medium finding). The paragraph's claim
that R keeps every retry relation is false today; that finding is in the
gap-analysis artifact (SR-20261103155258). The adjacent row inconsistency is in
the integrity artifact (SR-20261103155256).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The edited row's "In R" cell says "the same records and bounded sequence" for legacy `Workflow`, `Notice`, `Provider`. R declares `Workflow` and `Provider` as `object_type` and only `Notice` as `record` (examples/checked-handoff/model.semantic-ir.json:537, 762, 396). The PR's own left cell and paragraph call `Workflow` an object type, so the cell contradicts them on exactly the record/object distinction this ticket is about. Say that R keeps `Notice` as a record and `Workflow` and `Provider` as object types. | spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md:136 |
