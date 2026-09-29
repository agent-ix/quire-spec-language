---
id: SR-785
title: "QSL-305/QSL-307 gap analysis of PR 522 (quire-contract-ir pin bump blast radius)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0e93ed8dbb225266de2b55e363075a09f755d154; Cargo.toml; qsl-package/Cargo.toml; Cargo.lock; qsl-package/src/emit.rs; qsl-package/src/checked_v2/tests.rs; qsl-replay/src/spine/clause/tests.rs; tests/it/config_version_spine.rs (unchanged); tests/it/text_enum_identity.rs (unchanged); qsl-semantics/src/value/semantic_node.rs (unchanged); spec/functional/FR-105-emit-state-nodes.md (unchanged); spec/functional/FR-108-run-the-configversion-spine-corpus.md (unchanged); spec/test-cases/TC-463-s4-state-package-reads-back-and-is-stable.md (unchanged); spec/test-cases/TC-469-configversion-spine-corpus-matches-native.md (unchanged); spec/tests.md (unchanged); spec/spec.md (unchanged); quire-specification@e56756f proposals/checked-package-v2 (reference, read only)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-108
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: reviews
---
## Summary

Tickets: QSL-305 (frame_mutations shape) and QSL-307 (TC-463 un-ignore). PR:
quire-spec-language#522 at 0e93ed8d.

What was checked:

- QSL-305 acceptance. My own fresh quire-specification clone (e56756f)
  holds 30 `frame_mutations` vectors. 18 `modifies` entries: 8
  `{declaration, kind}` and 10 `{declaration, kind, name}`. The conformance
  test reads `expected_code`, `expected_cause` and `expected_locus_digest`
  from each vector. None of them are hand-written, and none are copied from
  code output. `make conformance` reports "30 frame-body mutation vectors
  matched".
- Blast radius. No `field_declaration`/`operation_declaration`/
  `clause_member_declaration` `ModelForm` reference is left in QSL code. The
  remaining hits are QSL's own `RecordFieldDeclaration`/`FieldDeclaration`
  types, which are unrelated, and the stale TC-469 doc prose, see FND-001.
  QSL's lowering already records frames as `OccurrenceRole::Generated`
  (qsl-semantics/src/check/lowering.rs:1327). Only test fixtures hard-coded
  `declaration`. The remaining `"role": "declaration"` literals in
  `checked_v2/tests.rs` are on non-frame nodes.
- QSL-307. TC-463 run with `--ignored` at head refuses `IllTyped`/
  `OperatorIneligible` at `/semantic_graph/nodes/25/body/arguments/0`. That
  matches the new reason, and `operations.rs:1090-1092` at 2a286437 confirms
  it. IR-370 exists in Linear, and its description matches the code.
- The other pinned-reader-blocked test. TC-469 step 6
  (`tc_469_step_6_the_emitted_package_admits_via_i04`) is ignored on
  QSL-315's `frame_eligibility` refusal. QSL-315 is Done, and IR-89 (this
  bump) is that fix. Run with `--ignored` at head, it now gets past the frame
  step. It refuses `IllTyped`/`OperatorIneligible` at node 8, and that node's
  body is a `quire.op.model.reaches_field` application, which I checked by
  dumping the emitted bytes in a scratch probe. So it is the same IR-370
  gap, but its ignore reason and three spec status notes still name
  QSL-315. See FND-001.

## Verdict

QSL-305's code change is correct and truly QSpec-derived. QSL-307's new
blocker is real and filed. FND-001 and FND-003 are stale blocked-by notes left
by the bump: TC-469's reason, plus FR-105/FR-108/TC-463/TC-469/tests.md/
spec.md status text. These are cheap spec and comment edits to make in this
PR. FND-002 is pre-existing drift the bump exposed, and should get its own
ticket rather than block this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-469 step 6 is still `#[ignore]`d on "QSL-315: FrameField's modifies wire shape is rejected by the pinned quire-contract-model I04 reader". Its doc cites 48ab5dc's `frame_eligibility`, which admits only `Relationship`/`FieldDeclaration`. The bump to 2a286437 removes that blocker. Measured at head with `--ignored`: the read now refuses `IllTyped`/`OperatorIneligible` at a `reaches_field` node (node 8), which is IR-370, the same gap as TC-463. QSL-315 is Done. So the ignore reason names a closed ticket and a blocker that no longer exists. Fix: re-reason TC-469's `#[ignore]` and doc to IR-370, as was done for TC-463. Update FR-108-AC-6's verification cell, the FR-108 dependency/status notes, and TC-469's status line from QSL-315 to IR-370. | tests/it/config_version_spine.rs:894-916; spec/functional/FR-108-run-the-configversion-spine-corpus.md:131; spec/functional/FR-108-run-the-configversion-spine-corpus.md:141-149; spec/test-cases/TC-469-configversion-spine-corpus-matches-native.md:77 |
| FND-002 | medium | Pre-existing, but surfaced by this bump. QSpec's `ModelOwner` (node-identity-preimage.schema.json at e56756f) requires `kind`, `identity`, `version` and `node`, all `Nonempty`, and `EnumDeclaration`/`Dimension`/`Unit` owners use it. IR 2a286437 now mirrors that with `NominalOwner::Model.version`. QSL's nominal `ModelSubject` has no `version`. `EnumDeclarationPreimage::from_json` admits a versionless model owner, and `text_enum_identity.rs:552-553` asserts that it does. So QSL's model-owned nominal preimage, and the node id hashed from it, diverge from QSpec's. The emit placeholder `version: ""` (SR-784 FND-001) is the visible edge of this. Fix: out of scope for a pin bump. File a QSL ticket to add `version` to `ModelSubject`/`CanonicalOwner`, update the text_enum_identity fixture, and plumb the real version through `nominal_owner`. | qsl-semantics/src/value/semantic_node.rs:50-58; tests/it/text_enum_identity.rs:552-553; qsl-package/src/emit.rs:498-507 |
| FND-003 | medium | QSL's spec still records TC-463/FR-105-AC-3 as blocked on the `state_clause` vocabulary, tracked by QSL-307. This PR resolves that blocker and QSL-307 will close with it, but the real blocker is now IR-370. Stale spots: the FR-105-AC-3 verification cell, the FR-105 Status paragraph, the TC-463 status, the spec/tests.md TC-463 row and the spec/spec.md FR-105 row. Fix: change each to "blocked on IR-370 (reaches_field reference_edge check)". This is a spec-only edit. | spec/functional/FR-105-emit-state-nodes.md:132; spec/functional/FR-105-emit-state-nodes.md:159-161; spec/test-cases/TC-463-s4-state-package-reads-back-and-is-stable.md:57; spec/tests.md:245; spec/spec.md:531 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open | 926873eb re-reasons only the `#[ignore]` string and doc on `tc_469_step_6_the_emitted_package_admits_via_i04`. The finding also required the FR-108-AC-6 verification cell (FR-108:131), the FR-108 Dependencies and Status notes (FR-108:143, :149) and the TC-469 Status line (TC-469:77) to move from QSL-315 to IR-370. All four still say "pending QSL-315", and so does the sibling `tc_469_step_6_package_id_is_pinned_across_every_case` doc (tests/it/config_version_spine.rs:848). They now contradict the test's own ignore reason, which says QSL-315's gap is fixed. |
| FND-002 | deferred | Filed as QSL-325 (Backlog): QSL's ModelSubject has no version field; QSpec's model owner requires a non-empty one. Out of scope for a pin bump; the emit placeholder cannot reach the wire today (see SR-784 FND-001). |
| FND-003 | fixed | 926873eb: FR-105-AC-3 cell, FR-105 Status, TC-463 Status, spec/tests.md TC-463 row and spec/spec.md FR-105 row all cite IR-370; no QSL-307 reference remains in spec/. |
