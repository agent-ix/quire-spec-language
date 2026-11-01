---
id: SR-1957109
title: "Manual gap analysis of quire-spec-language PR #1978: QSL-1377"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@peter/qsl-1377-selected-exact-identities; PR #1978 diff 8220d6c0a...HEAD; local FR-390-AC-17 and TC-142 trace tags; IR FR-352-AC-9 plus QSL-1377 ticket acceptance for record, inherited-field and reference identity rows"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-390, type: reviews }
---
# Manual gap analysis of quire-spec-language PR #1978 (QSL-1377)

## Summary

The installed Quoin CLI does not expose `quoin gap-analysis` (`command gap-analysis not found`). A scoped `quire matrix --scope . --format json` produced module diagnostics but did not return within the review window and was interrupted (exit 130). Per the reviewer fallback, this is a manual acceptance-to-tests review of the QSL-1377 acceptance and local QSL FR-390-AC-17. It makes no repository-wide coverage claim and skips optional semantic review.

The test for the producer's actual IR join is the object/reference case in `qsl-package/src/emit/tests/owner_model.rs`. The raw intake unit test asserts a record declaration row and local/inherited `MemberId` rows, but does not carry them into an admitted IR package. The new trace annotation also names `FR-352-AC-11`, which is not a criterion in this QSL repository: local QSL FR-352 is capability coverage and has AC-1 through AC-3. TC-142 is the compensation-slot emitted/readback scenario, not the selected identity producer scenario. These trace tags do not bind this test to the identity requirement it exercises.

## Verdict

**FAIL** — the selected-record and inherited-field identity rows lack the required admitted-package test, and the only integration test is tagged to a nonexistent local criterion and an unrelated test case.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The actual integration test for the object/reference producer carries `FR-352-AC-11`, but the local QSL FR-352 has only AC-1 through AC-3; `TC-142` specifies compensation-slot strict-readback, not identity projection. The test is therefore unbound to the local identity-witness obligation and creates an untracked criterion tag. Bind it to the relevant local QSL criterion/test case or add the missing QSL requirement/test-case trace artifact. | qsl-package/src/emit/tests/owner_model.rs:162-165; spec/functional/FR-352-report-capability-coverage.md:Acceptance Criteria; spec/test-cases/TC-142-evaluate-v2-compensation-expressions.md:Description | wrong-requirement |
| FND-002 | medium | The ticket requires rows to match admitted records and reference supplies, including inherited fields. The only IR-admission/projection test uses an object/reference row and asserts that the field-row vector is empty. The record/inherited-field test checks generated values before `SelectedExactIdentities::from_stage_rows` joins them to IR admission, so structural identity and inherited-member compatibility with the admitted record remain untested. Add an end-to-end record-view case that validates the record and inherited field rows and exercises missing-witness refusal. | qsl-semantics/src/model/intake/unit.rs:571-717; qsl-package/src/emit/tests/owner_model.rs:167-206; IR FR-352-AC-9 | correct-requirement-no-evidence |

## Coverage

- Plan completion: not assessed
- Criteria examined: IR FR-352-AC-9 and QSL-1377 acceptance for selected-document/declaration/field rows, admitted record/inherited-field joins, reference supplies, and missing-row refusal; local QSL FR-390-AC-17 checked for related identity-witness refusal behavior.
- Bindings examined: `selected_model_identity_rows_back_reference_projection_and_missing_rows_refuse` → `FR-352-AC-11` (invalid local criterion) and `TC-142` (unrelated compensation-slot test case). `intake_mints_record_field_and_reference_supply_rows_from_the_admitted_model` has no trace annotation and exercises row construction only, not IR join/projection.
- Method: manual focused acceptance-to-test and source-path inspection because Quoin's gap-analysis command is unavailable and the scoped matrix did not finish. No reverse-gap or repository-wide stub claim is made.
- Semantic review: skipped; not requested.
