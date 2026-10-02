---
id: SR-773
title: "QSL-312 gap analysis of PR 512 (FR-105 AC-2/AC-4 test remainder)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@5247a61078b8ee67283cbaf82af46c561dad71ca; spec/functional/FR-105-emit-state-nodes.md; spec/test-cases/TC-462-s4-emits-state-nodes.md; spec/test-cases/TC-463-s4-state-package-reads-back-and-is-stable.md; spec/tests.md; spec/spec.md (unchanged); qsl-replay/src/spine/clause/tests.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-462
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-463
    type: reviews
---
## Summary

Ticket: QSL-312. PR: quire-spec-language#512 at 5247a610. There is no plan
bundle. The scope is the FR-105-AC-2 and FR-105-AC-4 remainder. The spec edits
are status and traceability rows only, and no requirement statement changed.
`spec-review` sub-analyses therefore do not apply, and this file covers the
trace check.

Coverage below is measured against the code, with my own `make ci` run at
5247a610 (exit 0; 6433 `ok` lines and 0 FAILED; all 11 new tests pass under both the
default and the all-features runs):

| AC part | Test(s) | Status |
| --- | --- | --- |
| AC-2 dependencies | `s4_state_clause_and_anchor_dependencies_beyond_the_frame` | Covered, with exact sets for the clauses and the anchor. The field read's check is a subset check (SR-772 FND-001). |
| AC-2 occurrences | `s4_state_clause_anchor_and_frame_occurrences_match_the_outputs_table` | Covered, filtered by role (SR-772 FND-002). |
| AC-2 state_clause `semantic_type` | `s4_state_clause_semantic_type_is_the_boolean_scalar_type_node` | Covered. |
| AC-2 `pre` term | `s4_version_unchanged_condition_holds_a_pre_application_over_the_versionnumber_field_read` | Covered. |
| AC-2 `reaches_field` term | `s4_no_cycle_condition_holds_a_reaches_field_application_naming_parent`, `s4_parent_order_self_parent_read_shares_no_cycles_member_shape` | Covered. |
| AC-4 rename | `s4_renaming_parent_order_changes_no_node_id` | Covered. |
| AC-4 `<=` | `s4_changing_parent_orders_comparison_changes_its_node_id_and_the_package_id` | Covered. |
| AC-4 ParentOrder2 | `s4_parent_order2_with_parent_orders_body_adds_no_node_and_a_second_claim` | Covered. |
| AC-4 second post | `s4_a_second_post_on_attempt_update_adds_one_clause_node_and_no_anchor_or_frame` | Covered. |
| AC-4 Sub anchor sharing | `s4_pre_clauses_via_config_version_and_sub_share_one_anchor_at_config_version` | Covered. |

The spec rows are correct. FR-105's AC-2 and AC-4 rows and its Status now cite
QSL-279 and QSL-312. TC-462 and TC-463 Status and the `spec/tests.md` rows
agree. No file under `spec/` or `plan/` references canceled QSL-308 any more.
TC-463 correctly stays Partial, because step 1 is still `#[ignore]`d under
QSL-307.

`semantic_type` and occurrences are asserted on the in-process
`CheckedGraph`, not on the decoded wire. That is acceptable, because
`Candidate::wire_node` copies both straight from the graph
(`qsl-package/src/emit.rs:362`, `442-446`). This is not a finding.

Underspecified code: none. The diff adds no production code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-105 row in the spec index still reads "AC-1 to AC-4 pending STD-111 (QSpec wire); not yet implemented -- TC-462, TC-463 planned". It has been stale since QSL-279. With this PR covering AC-2 and AC-4, change it to: implemented under QSL-279/QSL-312/QSL-313; AC-1, AC-2, AC-4, AC-5 and AC-6 covered; AC-3 blocked on QSL-307. | spec/spec.md:531 |

## Verdict

PASS with one low finding. Every AC-2 and AC-4 clause the ticket names has a
traced test that can fail.
