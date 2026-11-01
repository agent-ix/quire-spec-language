---
id: SR-1949004
title: QSL-853 spec-review/integrity of PR1949
type: SpecReview
analysis: integrity
scope: agent-ix/quire-spec-language; QSL-853; qsl-package/src/checked_v2/tests.rs,
  qsl-package/src/checked_v2/tests/protocol_corpus.rs, qsl-package/src/emit/tests/protocol_compensation.rs,
  qsl-package/src/emit/tests/protocol_controls.rs, xtask/src/conformance.rs, xtask/tests/fixtures/checks.rs,
  spec/test-cases/TC-1063-protocol-emission-refusals-and-read-back.md; FR-383-AC-19/20
  contribution only
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/TC-1063
  type: reviews
---

## Summary

Ticket: QSL-853. The two alternatives are individually observable, unambiguous and consistent with QSpec FR-464: a set-order error reaches clause validation, while a repeated declaration is refused at the node by the earlier membership stage. Existing TC-1063 verifies FR-383; FR-383-AC-20 names this local mutation family and QSpec refusal contracts. This change does not create or duplicate a requirement or alter stakeholder/US allocation. Completeness is evaluated only for the edited oracle; full corpus replay remains deferred. Entity identity and topology probes were covered by the separate failure-domain lens.

## Verdict

PASS for the TC-1063 row 8a erratum only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Review selection: base plus integrity and failure-domain, scoped to the sole TC row 8a erratum and the changed test/runner paths. No active AssuranceProfile document exists in the local spec tree. EARS was inapplicable because no normative requirement statement changed; dependency/object/scope-boundary/risk-complexity lenses were inapplicable because no edges, domain entities, allocation, or operating risk changed. A separate evidence-method advisor was inapplicable because the Test method did not change. Criterion-strength/Jev and expensive optional gap semantic audit were not selected. Whole-repository matrix was intentionally not launched because of the reported hang; manual explicit reconciliation of actual scoped criteria and actual #[trace] tags is recorded below. This is a scoped review, not a repository-wide gap-analysis PASS. No plan supplied. Plan completion: not assessed.

| TC-1063 row | Test and oracle examined |
| --- | --- |
| 6 | protocol_nodes_and_occurrences_survive_the_whole_i2_read: Verified; each emitted protocol node retains node_id, tag, form, type, declaration, body, dependencies, occurrences; exact source_map equality. |
| 8a | a_misordered_or_repeated_role_is_refused_by_i2_at_its_normative_locus: two edits, InvalidSemanticGraph/None, clause node for descending order; role member node for duplicate. |
| 8b | a_missing_or_second_queue_is_refused_by_i2: two edits, InvalidSemanticGraph/None, exact affected channel node; source map and dependencies repaired so they do not mask the queue check. |
| 8c | a_control_dropped_from_its_holders_children_is_refused_at_the_control: InvalidSemanticGraph/None at orphan control. |
| 8d | a_receive_sequenced_before_its_send_is_a_definition_cycle: InvalidPackage/DefinitionCycle at Got, exact retained Got/Placed control ids in CausalCycle detail. |
| 8e | a_missing_or_swapped_control_binding_is_malformed_wire: missing attempt contracts and swapped send bindings, MalformedWire/None at each control body. |
| 8f | a_record_type_as_a_role_model_is_refused_at_the_target: InvalidModelBinding/MalformedDeclaration at role body argument 0 value target. |
| 8g | a_check_condition_that_reads_model_state_is_operator_ineligible: IllTyped/OperatorIneligible at input binder read. |
| 8h | descending_choice_labels_are_refused_by_i2: InvalidSemanticGraph/None at choice control. |
| 8i | a_compensation_without_a_selected_temporal_profile_is_refused_by_i2: MissingDeclaration/MissingSelection at Full compensation, after explicit admitted positive control without a temporal clause. |


The authorized delivery is local TC-1063 typed mutations 8a-i plus in-place positive conformance checks. No full FR-383-AC-20 discharge is claimed. The QSpec file holds two positives and 406 prose cases, not executable mutations; replay remains QSL-1366, blocked by STD-609. The corpus test preserves the admission oracle and attempts both positives, but P1 refuses MalformedWire/None at /semantic_graph/nodes/2/node_id and P2 at /semantic_graph/nodes/1/node_id. These occur before protocol semantic admission. No corpus positive admission or negative protocol-semantic proof follows from those results. The existing main C14 control log /tmp/codex1-qsl-1359-main-control.log reproduces P1; no reader code, dependency lock or input fixture is changed by this PR. QSL-1337 / PR1930 own that existing data/reader issue per dispatch. The lead retains aggregate-gate responsibility and the authorized baseline exclusion; this reviewer ran no aggregate gate. Full AC-19 reconstruction against the checked clause is beyond this test's node/occurrence read-back contribution. FR-383's pre-QSpec549 cyclic-component prose was left untouched, as required. None of these pre-existing/deferred obligations is classified as a newly introduced production defect.

## Validation

Independent checks at frozen HEAD 11347e06778588569a65f2cf9feafe7c74447d4a: cargo fmt --all -- --check exits 0; cargo test --offline --locked -p qsl-package --lib emit::tests::protocol_ -- --nocapture exits 0 (85 tests); cargo test --offline --locked -p xtask conformance::tests:: -- --nocapture exits 0 (7 parent tests; their synthetic child evidence includes intentional skip/failure cases); cargo clippy --offline --locked -p qsl-package -p xtask --tests -- -D warnings exits 0. Cargo checks used CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=3. The explicit corpus binary selection failed (0 passed, 1 failed) with both P1/P2 results above. TC-1063 explicit Quire validation exits 0. quire 0.37.1 (engine 0.50.3), /Users/peter/.cargo/bin/quire, machine Draco. Its stderr additionally reports semantic.inline-data-schema, DuplicateArchetype for ADR/Plan/Review/SpecReview/Standard from duplicate spec-artifacts-process contributions, and DuplicateInverseEdge part_of; these are tooling diagnostics, not suppressed or represented as document errors. Logs: /tmp/codex1-qsl-853-review-{native,xtask,clippy,corpus,tc-validation}.log. Full workspace clippy/tests and cargo-deny were not rerun by this reviewer; there are no dependency or production-reader changes.

## Examined scope

```yaml
scope:
- id: FR-383-AC-20
  path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
  role: context_only
  excerpt: Over **W**'s package, each mutation of the table in TC-1063 (a set out
    of ascending node-id order, a repeated set entry, a channel with no queue, a second
    queue for one channel, a control held by nothing, a `receive` sequenced before
    its `send`, an argument missing, a layout out of order, a model of the wrong kind)
    is refused by QSL's I2 reader with the code, cause and locus QSpec FR-464, FR-465
    or FR-466 gives it, and QSL's conformance run admits each positive and refuses
    each mutation of the QSpec protocol vectors file `protocol-node-vectors.json`.
- id: TC-1063-row-8a
  path: spec/test-cases/TC-1063-protocol-emission-refusals-and-read-back.md
  role: examined
  excerpt: '| 8a | `roles` out of ascending node-id order; a repeated `roles` entry
    | `invalid_semantic_graph` at the clause for the out-of-order set; `invalid_semantic_graph`
    at the repeated role member for the repeated entry |'
- id: QSpec-FR-464-Single-listing
  path: quire-specification/spec/objects/interfaces/FR-464-checked-package-protocol-node-bodies.md
  role: context_only
  excerpt: A role, channel, queue, compensation or obligation node named by two arguments
    of one clause, or by two clauses, refuses `invalid_semantic_graph` at the node.
- id: QSpec-FR-464-reader-order
  path: quire-specification/spec/objects/interfaces/FR-464-checked-package-protocol-node-bodies.md
  role: context_only
  excerpt: '5. **Membership**: containment, listing and unattached nodes.

    6. **Clause nodes**, each in ascending node-id order. For one clause:'
bindings: []
```
