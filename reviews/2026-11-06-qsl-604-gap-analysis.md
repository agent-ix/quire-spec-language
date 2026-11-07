---
id: SR-20261105101075
title: "Scoped gap analysis of QSL-604 slice 1 extension definition validation"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-604 slice 1; PR #2463 head 01ec946f0; FR-354-AC-3, TC-883 steps 1-4, FR-354 Definition validation, FR-111-CON-1, QSpec FR-467-AC-1..6, 8, 10; qsl-semantics/src/library/extension.rs; qsl-semantics/src/library/extension_tests.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-354
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-883
    type: reviews
---

## Summary

Ticket: QSL-604. This is a planless gap analysis scoped to the diff. Plan completion was not assessed. The mapping from acceptance criteria to tests is a manual check of the `#[trace]` tags in `extension_tests.rs` against FR-354, TC-883 and QSpec FR-467 (QSpec read from a temporary worktree at origin/main b09800f7). TC-883 steps 5 and 6 (S1) and FR-354-AC-1, 2, 4, 5 and 6 belong to the later slices QSL-1786, QSL-1787 and QSL-1788, and are out of scope.

## Verdict

**GAPS (non-blocking).** Every TC-883 step 1-4 expectation has a tagged test that drives `hold_extensions` and asserts the full payload: code, cause, definition, and the member, kind, name or path. The tests contain no stubs and no tautologies. Two trace tags claim more than their test covers, and several refusal branches are untested.

## Coverage

- TC-883 step 1 / FR-354-AC-3: `a_missing_member_refuses_naming_the_definition_and_the_member` (TypedNodeSchema, `invalid_package`, `missing-member`).
- TC-883 step 2 / FR-354-AC-3: `a_core_keyword_or_core_node_kind_refuses_naming_the_core_grammar`.
- TC-883 step 3 / FR-354-AC-3: `two_selected_extensions_sharing_a_keyword_or_node_kind_refuse_naming_both`.
- TC-883 step 4 / FR-354-AC-3: `extensions_depending_on_each_other_refuse_naming_the_cycle_in_path_order` ([a, b, a]).
- QSpec FR-467-AC-2, 6, 8, 10: covered by the tests above plus `collisions_within_one_definition_name_both_indices` and `only_the_first_violated_rule_is_reported`.
- FR-354 Definition validation, "held definitions only" and unselected definitions: `only_selected_extensions_are_validated_and_held`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Several refusal branches have no test: `ProductionKeyword`, both `ClauseName` sites, unparseable or non-object bytes, a selection under another authority, and the cross-definition first-rule case (rule 1 in a later definition with rule 2 in an earlier one). The FR-467 rule-1 cause list is only partly exercised. | qsl-semantics/src/library/extension_tests.rs:99-193; qsl-semantics/src/library/extension_tests.rs:370-396 |
| FND-002 | low | The test `a_valid_extension_is_held_with_its_productions_joined_to_node_entries` is tagged `QSpec-FR-467-AC-1` and `TC-883`. FR-467-AC-1 also requires the unit `retention Audit {...}` to parse, which is a later slice, so the tag claims the whole AC while the test covers only the validate half. TC-883's procedure has no valid-hold step. | qsl-semantics/src/library/extension_tests.rs:74 |
| FND-003 | low | The test `only_selected_extensions_are_validated_and_held` is tagged `FR-354-AC-3`. AC-3 says nothing about unselected definitions or held-only reading. That behavior is FR-354 "Definition validation" (held definitions only) and "Independence", so the tag gives AC-3 coverage it does not have and leaves the held-only rule untraced. | qsl-semantics/src/library/extension_tests.rs:398 |

## Evidence and limits

The 8 PR tests passed through `locked-build.sh` with the shared target, jobs 4 and debug 0. `quoin matrix` was not run: the analysis is diff-scoped and the trace check was done by hand. No full gate was run.
