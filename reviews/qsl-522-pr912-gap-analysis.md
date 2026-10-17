---
id: SR-11754
title: "Gap analysis of quire-spec-language PR #912: TC-1043 emission, read and decision rows to tests"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@294414ccb60ac665ab0945bc459a8eef3266f609; qsl-package/src/emit/tests/receiver_frames.rs, qsl-replay/src/spine/clause/tests/receiver_frames.rs, qsl-semantics/tests/it/model_population.rs, qsl-semantics/tests/it/model_receiver_frames.rs, qsl-semantics/src/check/checked_dispatch.rs, qsl-semantics/src/check/field_refinement.rs, qsl-semantics/src/check/lowering/model.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #912

## Summary

Ticket: QSL-522. Planless gap analysis, scoped to the PR diff. Each TC-1043
row in scope (W1 to W6, X1 to X9, F1 to F3, D1/D3/D4, I23) and FR-105-AC-7,
FR-154-AC-4/6/7/9/10 and FR-115-AC-11 is mapped to the test that asserts it,
and each new trace tag is checked against the AC it names. Plan completion:
not assessed. Read-only, with no test run. The three conformance tests that
read QSpec fixtures are excluded per the dispatch brief.

Mapping (examined):
- W1, W2, W3 and FR-154-AC-6: `the_frame_bodies_of_scoped_entries_are_the_wire_vectors`
- W4, W5, W6, B1/B2 and FR-154-AC-7: `a_scope_or_a_scoped_delete_changes_the_node_id_and_the_package_id`
- Step 2b and FR-105-AC-7: `equal_frames_share_one_node_with_an_occurrence_per_operation`
- X1 to X3: `a_missing_or_unknown_scope_and_a_missing_deletes_scoped_refuse_as_schema_failures`
- X4 to X6: `a_misordered_deletes_scoped_and_an_unknown_parameter_refuse` (X6 locus asserted at the re-keyed frame node)
- X7 to X9 and FR-154-AC-10: `both_scopes_of_one_field_refuse_conflicting_binding_in_either_order`
- F1 to F3 and FR-115-AC-11: the two `qsl-replay` receiver_frames tests
- D1 to D4 and FR-154-AC-9: `a_frame_holding_both_scopes_adds_them_up`
- I23 and FR-154-AC-8: `a_record_value_type_operation_refuses_receiver_scoped_members_at_the_member`
- FR-154-AC-4: tag on the AC-7 test (see FND-002)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-154-AC-9 test does not run AC-9's scenario. AC-9 and TC-1043 step 5 put both scopes on one type: `Counter`, `c1..c3`, `receiver_modifies` `value` with `modifies` `step`, so `c2.step` is granted while `c2.value` is refused. The test uses `four_link_chain_bundle` with `model.A.x` receiver-scoped and `model.Plain.x` every-object, which are fields of two unrelated types. So the same-type case of one object passing on one field and failing on the other is never exercised. It is the only test tagged FR-154-AC-9. | qsl-semantics/tests/it/model_population.rs:4570-4661 |
| FND-002 | medium | `#[trace("TC-1043", "FR-154-AC-4", ...)]` tags FR-154-AC-4, whose verification is TC-573. Its second half (`increment` with both scopes of `value` refused at intake with `conflicting-binding` naming both entries and `increment`, and no frame node emitted) is not asserted by this test, and no test anywhere carries TC-573. The tag inflates AC-4 coverage. | qsl-package/src/emit/tests/receiver_frames.rs:184 |
| FND-003 | medium | Three production behaviour changes have no test. `checked_dispatch` `is_query` now treats an operation with only receiver-scoped writes or scoped deletes as a non-query. `field_refinement` and `lowering/model.rs` now count receiver-scoped writers through `written_fields`. Reverting any of the three would leave every test green. | qsl-semantics/src/check/checked_dispatch.rs:958; qsl-semantics/src/check/field_refinement.rs:92-96; qsl-semantics/src/check/lowering/model.rs:307-316 |

## Verdict

PASS (no major bug). The W, X, F rows, step 2b and I23 are each asserted by a
test that checks the stated vector or refusal code, cause and, for X6, the
locus. The three mediums are recorded per the major-bugs-only rule. They are
not blocking.

Recorded, not blocking:
- low: the step 2b test checks three anchors but not TC-1043's "B3 holds two
  clause nodes" (`DoneAll` and `DoneSame` reference different anchors).
- low: the D4 case deletes `r1`, `p1` and `o1` together, where TC-1043 D4
  deletes only `c3`. The violation it names is the same.
