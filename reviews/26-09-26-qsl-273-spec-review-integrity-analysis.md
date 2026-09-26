---
id: SR-662
title: "QSL-273 integrity review of state clauses on the spine"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@d8b74aba7d349ccb3989583cc4e608aad301c38b; spec/decisions/ADR-012-semantic-family-extension-contracts.md (§15); spec/functional/FR-102 to FR-109; spec/test-cases/TC-456 to TC-469; unchanged: spec/decisions/ADR-011, ADR-013 (§2, O-06, O-08 to O-10), ADR-014 (§4), spec/functional/FR-023, FR-026, FR-028, FR-031, FR-032, FR-056, FR-057, FR-062, FR-088, FR-094, FR-100 (on origin/main 5e7a2615); code qsl-semantics/src/check/identity.rs, qsl-semantics/src/check/lowering.rs, src/linking.rs, src/command/extraction.rs, src/package/reading.rs, src/protocol_artifact/mod.rs, src/checking/composed/solver.rs; quire-specification@0d53cf2 proposals/checked-package-v2/schema.json, node-identity-preimage.schema.json, FR-322, FR-340, FR-180, native-diagnostics.md 1-draft.7; quire-verification-contracts@61f4a44 contracts/checked-operation-catalog-v1.json"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: reviews
---

## Summary

Ticket: QSL-273 (PR agent-ix/quire-spec-language#462). This review checks
consistency: ADR-012 §15 against ADR-011, ADR-013, ADR-014 and the merged
requirements, and FR-105 against the QSpec wire and the operation catalog.

Clean:

- ADR-013 §2 (one encoder). FR-106's `sha256-jcs` digest cites FR-056's
  rule, which names `quire-canonical` (FR-056:82-84). The snapshot digest
  therefore uses the one RFC 8785 encoder.
- ADR-011 M-6c ordering. Native `run` is deleted only after FR-108 passes.
- FR-057 and FR-062. `operation-contract` is the state family's kind
  (FR-057:48, 175-176, 194). FR-104 makes one record per clause, keyed by
  occurrence, which matches QSL-266's per-claim keying.
- IR 48ab5dc. It decodes all five `StateForm`s
  (quire-contract-model/src/checked_package/v2/vocabulary.rs:237-246), and
  `lower` returns `false` for every `state` node (lower.rs:615-621), as FR-105
  says.
- FR-106's codes and causes all appear in QSpec native-diagnostics
  `1-draft.7`, except `invalid_source_identity` (FND-007).

Verdict: changes requested (two high).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `CheckedClauseKind` has two wire halves, and FR-105 specifies only one. FR-088-AC-4 and ADR-013 O-10 require both to be total and injective: the clause operation identity (`wire_operation_identity`, qsl-semantics/src/check/identity.rs:131-156) and the (`node_tag`, `semantic_form`) pair (identity.rs:158-195). FR-105 gives only the pair half for `Invariant`, `Precondition` and `Postcondition`. The catalog and v2 `OperationIdentity` hold no state clause operation (`quire.op.state.transition` is the only `state` one). The existing `StateTransition` also maps to (`state`, `frame`) (identity.rs:178, 191), which is the form FR-105 now emits for frames. Failure scenario: `wire_operation_identity` must gain three arms with no identity to return, so the implementer adds a `_` arm or invents a string. `from_node_tag_and_semantic_form("state", Some("frame"))` then classifies every emitted frame node as a transition clause. FR-105-AC-5's "all seven variants" test covers only the pair half, so it passes. Fix: state that state clauses carry no clause operation, and amend FR-088-AC-4 and ADR-013 O-10 to allow that (or add a catalog identity to the QSpec list). Remap `StateTransition` to (`state`, `transition`). | spec/functional/FR-105-emit-state-nodes.md:85-89, 110; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1179-1182 |
| FND-002 | high | FR-105 emits `model`/`field_declaration` and `model`/`operation_declaration` nodes "keyed by FR-094's ModelOwner rule". That rule cannot key them today. FR-094, unchanged, says a field member and an operation member "has no node of its own", and makes them internal-fault arms of the record match (FR-094:103-106, 124-127). QSpec's `ModelDeclarationNode` preimage admits only `object_type`, `systems_interface` and `relationship` (node-identity-preimage.schema.json:64). FR-322's owner recovery computes keys only for object types and relationships, and it requires a model node's `semantic_type` to be its own node id. FR-105 instead gives the value type, or the result type or `Boolean`. Failure scenario: the emitter has no published preimage to key `field_declaration` with, so QSL invents one. QSpec's I04 read then refuses `missing_declaration`/`missing-selection` or `invalid_package`/`stale-node-key`, and an IR or CG reader cannot reproduce the frame's `modifies` keys. Fix: amend FR-094 (add a `FieldMemberRecord` row, and an `OperationMemberRecord` row if kept), and add the preimage and owner-recovery change to the QSpec change list. FR-340 already needs `field_declaration` nodes for any field frame. | spec/functional/FR-105-emit-state-nodes.md:31-33, 63-64; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1183-1185 |
| FND-003 | medium | FR-105 lowers `reaches` with a `member` that "names the edge's `model`/`field_declaration` node". Three sources name a model field member by the declaring object type's node plus the field name: ADR-013 O-06 ("the declaring node is the model node of the receiver's static object type"), FR-094's `field` member rule, and QSpec FR-322's "Model-owned members". QSL's own `deref(r).f` lowering does the same (qsl-semantics/src/check/lowering.rs:3464-3487). Failure scenario: `NoCycle`'s `reaches` member carries `declaration` = the `parent` field node while `self.parent` in `ParentOrder` carries `declaration` = the `ConfigVersion` node. Two conventions for one member kind reach one package, and the FR-322 model-owned resolution fails for the `reaches` one. Fix: `{kind: "field", declaration: <ConfigVersion object_type node>, name: "parent"}`. | spec/functional/FR-105-emit-state-nodes.md:74-76; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1186-1193 |
| FND-004 | medium | The PR amends merged artifacts without editing them. FR-103 "amends FR-056's Complete-V1 paragraph", but FR-056 is unchanged. FR-094's no-node rule (FND-002) is unchanged. FR-088-AC-4 still requires every variant to have "exactly one ... clause-operation-identity spelling" (FR-088:221), which the three new variants have no way to meet (FND-001). ADR-013 O-09 still defines a clause as a `claim`, `temporal` or `protocol` node, and O-10's serialized-authority list has no state clause. FR-090/FR-063's S6a seam list gains a `ProtocolClause` arm (FR-107-AC-6), and FR-090 is not touched. Failure scenario: an implementer following FR-056, FR-088 or FR-094 as written refuses what FR-103 and FR-105 require, and the drift gate has no amended text to point at. Fix: amend each in this PR, or record each as "amended by FR-10x" in the target file. | spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md:30; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1170-1185 |
| FND-005 | medium | FR-109 conflicts with FR-100 (QSL-271). FR-100 merged to main at 5e7a2615, after this branch's base 6938db3d. FR-100's `qsl_replay::spine::run` already runs a named function with name-keyed arguments and one total outcome mapping: a kernel refusal becomes `invalid_runtime_input` with its cause, and `Refusal::CheckedInvariant` becomes `runtime_invariant` with exit 30. FR-109's `Function { name, arguments, snapshot }` takes positional arguments, and its `exit_code()` sends every refusal through `Code::exit_code`, which never yields 30. FR-109 cites QSL-271 but not FR-100. Failure scenario: one function returns exit 30 through `spine::run` and exit 20 through `run_clause`, and the kernel refusal cause is lost in the second. The branch also conflicts with main in spec.md and tests.md. Fix: rebase, add an FR-100 edge, and reuse FR-100's outcome mapping and argument binding for the `Function` arm, adding only object arguments. | spec/functional/FR-109-run-a-state-clause-through-the-spine.md:53-55, 98-108, 130-132 |
| FND-006 | medium | ADR-012 §15.8 has the FR-109 PR delete `native_model`, `model_source` and `mapped`, but they are not reached only by native `run`. Native compile imports them (src/linking.rs:15; src/command/extraction.rs:9-10). So do `NativePackage` (src/package/reading.rs:9), SEAM-2 (src/checking/composed/solver.rs:19) and SEAM-3 (src/protocol_artifact/mod.rs:49). ADR-011 keeps native compile for `0-draft` until QSL-5, and merged FR-100 routes a `0-draft` `run` to native run (FR-026). Failure scenario: the landing PR either does not compile, or removes `0-draft` compile and run with no ruling. Fix: limit §15.8 to the modules only native `run` and `state` reach, and say what happens to FR-100's `0-draft` route. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1241-1249 |
| FND-007 | low | Code inventory gaps. FR-106 says QSpec native-diagnostics `1-draft.7` holds "every code and cause above", but `invalid_source_identity` is not in it (it is QSL's own code, FR-001). ADR-012 §15.6 says every result maps through its table, but the admission rows leave out `unavailable_observation`, `stage_limit_exceeded`, `unknown_wire` and `invalid_source_identity`. It also leaves out FR-109's stage-`admit` `missing_declaration` and `ill_typed`. Failure scenario: a report category test built from §15.6 misses six codes FR-106 and FR-109 produce. | spec/functional/FR-106-admit-snapshots-and-invocations.md:145, 216-217; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1212-1228 |
| FND-008 | low | FR-108 claims QSL's share of QSpec FR-180-AC-5. FR-180's reference verdict contract has each case pin its v2 package bytes and `package_id`, and admits them "only through I04 `read`". FR-108's expected table pins neither, and the emitted bytes cannot pass I04 `read` until the QSpec changes land (FR-105 Dependencies). Failure scenario: the corpus passes while a later emitter change moves every `package_id` unnoticed. Fix: pin each case's `package_id` in the expected table, and state that the I04 `read` half waits on the QSpec ticket. | spec/functional/FR-108-run-the-configversion-spine-corpus.md:23-27, 123, 133-134 |
| FND-009 | medium | Disposition pass, at c35a6a49. STD-111's text no longer matches the spec it serves. FR-105 now spells a `state_clause` body as an application of `quire.op.state.clause` whose `member` is `{kind: "state_clause", clause: <kind>}` with arguments (parameter aggregate, anchor reference, condition), but STD-111 item 1 still asks for an aggregate of three bindings. STD-111 item 5 says "Keep the existing `StateTransition` -> (`state`, `frame`) decoding", and the fix moves it to (`state`, `transition`). STD-111 also lacks three QSpec changes the new FR-105 needs: an `OperationMember` kind `state_clause` (the v2 union and the catalog's `member_kinds` hold only field, position, element, relationship_end, operation, type_argument and profile_operator); an `application.operator` class for `quire.op.state.clause` with its catalog entry (operands, result, laws); and the rule tying each clause application to its node form. ADR-012 §15.4, FR-088 and FR-105 call (`state`, `transition`) "its QSpec form" without citing STD-111. Failure scenario: QSpec implements STD-111 as written, and the package FR-105 emits fails QSpec's I04 read on the body shape and the unknown member kind. Fix: update STD-111 (see the disposition report) and cite it for the `StateTransition` spelling. | spec/functional/FR-105-emit-state-nodes.md:63-80, 98-110; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1189-1201 |
| FND-010 | low | Disposition pass, at c35a6a49. FR-105 marks AC-1 and AC-2 as "emission pending STD-111", but its Status says AC-4 to AC-6 do not wait, and AC-4 asserts node ids and `package_id` changes of those same emitted `state` nodes, whose preimages include the STD-111 spellings. Failure scenario: AC-4 passes on a pre-STD-111 spelling, and every id it pinned changes when STD-111 lands. Mark AC-4's emitted-id assertions pending too, or state that AC-4 checks only relations between ids (equal or different), which survive a spelling change. | spec/functional/FR-105-emit-state-nodes.md:122-127, 145-149 |
| FND-011 | low | Disposition pass, at c35a6a49. A blank FR-001 label on a snapshot now refuses `invalid_runtime_input`/`invalid-value` (FR-106 check 1.7). FR-001, FR-018-AC-8 (native snapshot construction) and FR-026-AC-6 refuse a blank FR-001 label `invalid_source_identity`. Failure scenario: the same blank `authority` gets two codes depending on the path, and a consumer that keys on the code treats them as different defects. State in FR-106 that the spine uses the catalogued `invalid_runtime_input` on purpose, or keep `invalid_source_identity` and register it in native-diagnostics through STD-111. | spec/functional/FR-106-admit-snapshots-and-invocations.md:174-175 |

## QSpec change list (for the leader's ticket)

Today's schema already admits FR-105's `state_clause` and `operation_anchor`
bodies as a generic `SemanticTerm`, because `BodyBindingRules` special-cases
only `state`/`frame`. No new term kind or operator is needed: `pre`, `deref`,
`reaches`, `record.project`, and the occurrence roles `claim`, `anchor` and
`generated` exist. On this point the author's claim holds. QSpec must still
change the following:

1. A normative body rule for `state`/`state_clause` (an FR beside FR-340, and
   a `BodyBindingRules` branch). It is a closed aggregate of three bindings in
   fixed order:
   - `parameters`: references to `value`/`parameter` nodes. `self` is level
     0, then `result` when present, then the operation's parameters.
   - `anchor`: a `model`/`object_type` node for `invariant`, and a
     `state`/`operation_anchor` node otherwise.
   - one binding named `invariant`, `precondition` or `postcondition`, whose
     value is a Boolean term.

   The rule also fixes `semantic_type` as Boolean, the occurrence role as
   `claim`, and one refusal (with its precedence) per defect.
2. The same for `state`/`operation_anchor`: bindings `context`, `operation`
   and `frame`, and a `semantic_type` rule. The minimal `operation` target is
   a text literal naming the operation. A reference to a
   `model`/`operation_declaration` node needs item 4 for that form.
3. `state`/`frame` node `semantic_type` and occurrence role. FR-340 is silent
   on both, and FR-105 picks the context object type and `generated`.
4. Identity for `model`/`field_declaration` (and `operation_declaration`, if
   kept):
   - extend the `ModelDeclarationNode` preimage's `semantic_form` enum
     (node-identity-preimage.schema.json:64);
   - extend FR-322 "Model-owned members" step 2 owner recovery to field
     records;
   - decide their `semantic_type`. Step 2 requires the node's own id, and
     FR-105 wants the value type.

   FR-340 frames already depend on this.
5. `quire.op.model.reaches` over a field. It needs the `field` member kind,
   whose declaring node is the object type (FR-322 model-owned members, ADR-013
   O-06), not a `field_declaration` node. The catalog `member` is one string
   per entry for all 135 entries, so pick one:
   - (a) make `member` a set: a catalog format change for every reader in
     quire-verification-contracts, QSpec, IR and CG;
   - (b) add `quire.op.model.reaches_field` to the v2 `OperationIdentity` enum
     and the catalog.

   Either way, add a constraint kind that requires the field type to be
   `Ref(T)`, `Option(Ref(T))` or `Seq(Ref(T),N)` for the operand type `T`.
   `reaches` has `constraints: []` today, so without it any field is admitted.
6. A clause operation for state clauses. Either add a catalog identity (for
   example `quire.op.state.clause`), or have QSpec and ADR-013 O-10 state
   that a state clause has none (FND-001).

## Verdict

Changes requested. FND-001 and FND-002 must be fixed before implementation.
Items 1 to 6 above are the QSpec ticket.

## Dispositions

Disposition pass at `agent-ix/quire-spec-language@c35a6a49` (fix commit `c35a6a49`, "QSL-273 spec: fix SR-660 to SR-664 review findings", rebased onto main 5e7a2615). Each outcome was re-checked against the spec and code at that head, not taken from the commit message. `quire validate` over the changed spec files and these reviews exits 0 with no EARS warnings.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed c35a6a49 | The three new kinds spell (`state`, `state_clause`) plus `quire.op.state.clause` and a kind member; FR-088-AC-4 and ADR-013 O-10 are amended; `StateTransition` moves to (`state`, `transition`), and (`state`, `frame`) decodes to no kind. The QSpec side is not in STD-111's text yet: see new FND-009. |
| FND-002 | fixed c35a6a49 | No `field_declaration` or `operation_declaration` node is emitted; members are (object type node, name); FR-094 stays true and needs no amendment. The frame-entry pair needs QSpec FR-340's FrameBody to change, which STD-111 item 3 carries. |
| FND-003 | fixed c35a6a49 | `reaches` lowers to `quire.op.model.reaches_field` with `{kind: field, declaration: <object_type node>, name}`, the same shape as a field read (FR-105-AC-2). |
| FND-004 | fixed c35a6a49 | FR-056, FR-088 and ADR-013 O-08 to O-10 are amended in the PR. FR-094 is unchanged because no member node is emitted, and FR-090 already allows one S6a variant per `ReferenceEvaluation` family (FR-090:45, 143). |
| FND-005 | fixed c35a6a49 | The branch is rebased onto main 5e7a2615. FR-109 reuses FR-100's argument binding, outcome mapping and exit statuses, adds only object arguments and the claim reading of a Boolean, and cites FR-100. |
| FND-006 | fixed c35a6a49 | §15.8 has a three-step order: nothing native is deleted with FR-102 to FR-109; M-6c removes the 0-draft path with QSL-5; `native_model` goes with its last SEAM-2 or SEAM-3 importer. |
| FND-007 | fixed c35a6a49 | §15.6 lists every admission and selection code. `invalid_source_identity` is replaced by `invalid_runtime_input`/`invalid-value`, which leaves a cross-path code difference: new FND-011. |
| FND-008 | fixed c35a6a49 | FR-108-AC-6 pins the `package_id` and the I04 `read`, pending STD-111. |

### Round 2

Disposition pass at `agent-ix/quire-spec-language@39647e7c` (fix commit `f83ca481`, "QSL-273 spec: fix disposition-pass findings (SR-660, SR-662, SR-664)"). Each outcome was re-checked against the spec and code at that head. `quire validate` over the changed spec files and the reviews exits 0.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-009 | fixed f83ca481 | STD-111 is rewritten: item 1 is the `quire.op.state.clause` application, and item 5 adds the operator class, the `state_clause` member kind and the node-form rule. FR-105, FR-088 and ADR-012 §15.4 cite STD-111 item 5 for (`state`, `transition`) and name the TC-250 table update. |
| FND-010 | fixed f83ca481 | FR-105-AC-4 and TC-463 steps 2 and 3 are marked pending STD-111, with the reason stated in FR-105 Status, tests.md and spec.md. |
| FND-011 | fixed f83ca481 | FR-106 check 1.7 states the divergence from FR-001, FR-018-AC-8 and FR-026-AC-6, and STD-110 records the request for `invalid_source_identity` in catalog 1-draft.8 (verified in Linear). |
