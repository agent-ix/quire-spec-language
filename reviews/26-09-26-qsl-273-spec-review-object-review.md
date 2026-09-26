---
id: SR-663
title: "QSL-273 object review of state clause, anchor, frame and observation objects"
type: SpecReview
scope: "agent-ix/quire-spec-language@d8b74aba7d349ccb3989583cc4e608aad301c38b; spec/functional/FR-102 to FR-109; spec/decisions/ADR-012-semantic-family-extension-contracts.md (§15.2 to §15.5); spec/test-cases/TC-461, TC-463, TC-466; code qsl-cst/src/grammar.rs, qsl-semantics/src/model/domain_package.rs, qsl-semantics/src/model/population.rs, qsl-eval/src/value/expression/mod.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: reviews
---

## Summary

Ticket: QSL-273 (PR agent-ix/quire-spec-language#462). This review audits the
domain objects the PR adds: `StateClauseForm`, the checked state clause, the
`state_clause`/`operation_anchor`/`frame` nodes, `OperationEffect`,
`AdmittedObservations`, `ClauseSelection` and `ClauseRunReport`. It checks
their identities, their cross-references and their entity completeness.

Clean: the family assignment (§15.2) gives every concern one owner, and no
family edge is added. `ClauseRunReport` separates admission work from meter
charges. `AdmittedObservations` keeps each document's identity and digest.
Reference equality ignores the observation, as the QSpec state contract
requires.

Verdict: changes requested (three medium).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Node cardinality contradicts shared identity. FR-105 emits "one `state`/`state_clause` node per clause" with occurrence `claim, 0`. FR-104-AC-6 and TC-461 step 4 make two equal-body clauses share one node id, told apart only by their `claim` occurrence. Node ids are unique within a graph (FR-322), so the second clause must be a second occurrence (`claim`, 1) on the same node, not a second node. Failure scenario: for `ParentOrder` and `ParentOrder2`, one emitter writes two nodes with one `node_id` and I2 refuses the package. Another writes one node with occurrence `claim, 0` only, so `ParentOrder2`'s requirement record (TC-461 step 4, five records) is keyed by an occurrence the source map does not have. Fix: say "one node per distinct clause node id, with one `claim` occurrence per declaration, ordinals in source order", and add the emission half to FR-105-AC-4. | spec/functional/FR-105-emit-state-nodes.md:29-31, 60; spec/functional/FR-104-check-state-clauses.md:131; spec/test-cases/TC-461-s3-records-state-clause-requirements.md:24, 36-37 |
| FND-002 | medium | Operation anchor identity is under-specified for inherited operations. FR-105 emits "one `operation_anchor` ... per operation", but the anchor body binds `context` to the clause's object type. FR-103 makes an operation visible on subtypes, and FR-104 resolves `op` in `M::T`'s effective view. Failure scenario: `pre A using v on M::Base::op` and `pre B using v on M::Sub::op` name one operation from two contexts. One implementation emits one anchor (whose `context`?), another emits two, and their `package_id`s differ. Fix: key the anchor by (context, operation), or by the operation's owner only and drop `context` from the body. Add an AC with a subtype. | spec/functional/FR-105-emit-state-nodes.md:61, 93-95; spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md:72-73 |
| FND-003 | medium | Clause and function names share a selection namespace with no collision rule. FR-104 refuses only a second state clause with the same name. FR-109 resolves the selected name "in the compiled package's own clause and function tables", and FR-107-AC-5 expects a function name to refuse `UnknownClause`. Failure scenario: a unit declares `invariant sameIdentity ...` and `function sameIdentity ...`. Both check, and a `run_clause` selection of `sameIdentity` is resolved by whichever table the implementation searches first. Fix: refuse a clause whose name equals a function's (or any declaration's) name at S3, with an AC. | spec/functional/FR-104-check-state-clauses.md:73-74; spec/functional/FR-109-run-a-state-clause-through-the-spine.md:87-89 |
| FND-004 | low | `reaches`'s `edge` object is narrower than the grammar. S1 parses the edge as a `QualifiedName` (qsl-cst/src/grammar.rs:905-914), but FR-102's `Reaches { edge }` is "the member name and its span", and nothing says what S2 or S3 does with `reaches(a, b, M::T::parent)`. Failure scenario: one builder refuses a multi-segment edge at S2, another takes its last segment. Fix: say which. | spec/functional/FR-102-build-state-clause-forms.md:54-56, 71-74 |
| FND-005 | low | FR-107's Outputs gives `Result<Evaluation, InternalFault>`, but its Behavior gives `evaluate_clause` the result `Result<Evaluation, CallFailure>`, with `InputRefusal::UnknownClause` and `ObservationsMismatch` refusals. Failure scenario: an implementer following Outputs has no channel for FR-107-AC-5's refusals. Make Outputs `CallFailure`. | spec/functional/FR-107-evaluate-state-clauses-at-s6a.md:46-60 |
| FND-006 | low | FR-103 admits frame entries that no object can carry. `modifies` may name a relationship and `creates`/`deletes` a process. But `OperationEffect.modifies` is "field members this operation writes" (qsl-semantics/src/model/domain_package.rs:246-253), the domain package records have no process variant, and FR-106's snapshot has no relationship links. Failure scenario: a `modifies [Rel]` grant is admitted at I1 and never enforced at admission, so a relationship change passes every frame check. Fix: refuse relationship and process entries as `unknown_required_feature` until an observation can carry them, or specify how FR-106 compares links. | spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md:55-57; spec/functional/FR-106-admit-snapshots-and-invocations.md:186-192 |

## Verdict

Changes requested: FND-001 to FND-003 leave object identity open to two
readings.

## Dispositions

Disposition pass at `agent-ix/quire-spec-language@c35a6a49` (fix commit `c35a6a49`, "QSL-273 spec: fix SR-660 to SR-664 review findings", rebased onto main 5e7a2615). Each outcome was re-checked against the spec and code at that head, not taken from the commit message. `quire validate` over the changed spec files and these reviews exits 0 with no EARS warnings.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed c35a6a49 | One node per distinct clause node id, with one `claim` occurrence per declaration in source order; FR-105-AC-4 adds `ParentOrder2` (no new node, a second occurrence at ordinal 1). |
| FND-002 | fixed c35a6a49 | One anchor and one frame per (declaring object type, operation name); FR-105-AC-4 adds the `Sub` case. |
| FND-003 | fixed c35a6a49 | FR-104 refuses a clause named like another clause or a function with `ambiguous_declaration`/`ambiguous-name`, and FR-109 resolves one shared name table (AC-3). |
| FND-004 | fixed c35a6a49 | FR-102 refuses a multi-segment edge with `UnrepresentedConstruct` (AC-3). |
| FND-005 | fixed c35a6a49 | FR-107's Outputs is `Result<Evaluation, CallFailure>`. |
| FND-006 | fixed c35a6a49 | FR-103 refuses relationship `modifies` and process `creates`/`deletes` entries as `unknown_required_feature`/`unsupported-feature` (AC-2). |
