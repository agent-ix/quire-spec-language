---
id: SR-9085
title: "Gap analysis of quire-spec-language PR #832: QSpec V1-EXPR-017..022 trace tags"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@77b16803f4ba9a2b7c9372fa5a33b42c0529cab9; PR #832: QSpec rows V1-EXPR-017..022 and the QSpec FR-009, FR-012, FR-013 and FR-006 acceptance ids tagged in qsl-eval/tests/it/model_reference_queries.rs, qsl-replay/src/spine/clause/tests/{frame,pre_call_invocation,state_clause_replay}.rs, qsl-semantics/tests/it/state_clauses.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #832

## Summary

Ticket: QSL-716. Plan completion: not assessed. The oracle is the private QSpec inventory and FR rows, cited here by id only. The rows map as 017 to FR-009, 018-020 to FR-012, 021 to FR-006 and 022 to FR-013. That mapping is correct.

Bindings spot-checked against the QSpec AC each one claims:

- QSpec-FR-012-AC-2 and QSpec-FR-013-AC-4 on `an_absent_pre_snapshot_is_incomplete_not_a_violation`: correct. It asserts `unavailable_observation` and `Incomplete`, with no violation.
- QSpec-FR-012-AC-3 on `result_outside_a_postcondition_refuses_wrong_anchor`: correct. It asserts `wrong_snapshot` in invariant and precondition context.
- QSpec-FR-012-AC-4 on `pre_of_a_captured_post_reference_retains_its_post_observation`: correct.
- QSpec-FR-012-AC-5 on `a_precondition_over_an_invocation_reads_only_the_pre_side`: correct in substance. It reads `parent`, not `versionNumber`.
- QSpec-FR-012-AC-8 on the new test and on `pre_refuses_outside_a_postcondition_context`: correct.
- QSpec-FR-013-AC-1 on the `frame.rs` violation tests, `attempt_update_refuses_a_change_outside_modifies` and rows 26 and 27: correct. They assert `frame_violation`/`unauthorized-change`.
- QSpec-FR-013-AC-2 on the declared-delta tests and row 28: correct. They assert `population_delta_mismatch`/`delta-disagreement`.
- QSpec-FR-013-AC-5 on `a_same_named_field_of_an_unrelated_type_is_never_authorized` and `a_write_to_a_field_that_redefines_a_modifies_grant_admits`: correct.
- QSpec-FR-013-AC-6 on `a_creation_of_a_subtype_of_a_creates_grant_admits`: covers one half only (recorded).
- QSpec-FR-012-AC-1 on `an_invocation_naming_the_wrong_observation_roles_refuses_wrong_observation`: wrong (FND-001).

## Verdict

Changes requested. FND-001 and FND-002 block. FND-003 is recorded.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The QSpec-FR-012-AC-1 tag overclaims. The AC concerns different pre/post invocation identities. The tagged test swaps the role references of one invocation, which the PR body itself says leaves the identity case untested. The matrix will still show AC-1 covered. Drop the AC-1 tag and keep the V1-EXPR-020 tag. | qsl-semantics/tests/it/state_clauses.rs:1515 |
| FND-002 | medium | The PR body says rows 019/020 and 022 are covered. QSpec-FR-012-AC-6, QSpec-FR-012-AC-7 and QSpec-FR-013-AC-3 carry no tag anywhere in the repo, and none of them is on the not-covered list. Tag an existing test for each, or add each to the not-covered list. | qsl-semantics/tests/it/state_clauses.rs; qsl-replay/src/spine/clause.rs:83 |
| FND-003 | low | Recorded. QSpec-FR-013-AC-6 is tagged only on the subtype-creation test. The deletion half is asserted by `invocation_admits_a_subtype_created_and_deleted_under_a_supertype_grant`, which is untagged. | qsl-semantics/tests/it/state_clauses.rs:2450; qsl-semantics/tests/it/model_population.rs:2801 |
