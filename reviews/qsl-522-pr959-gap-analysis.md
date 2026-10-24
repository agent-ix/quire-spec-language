---
id: SR-12134
title: "Gap analysis of quire-spec-language PR #959: closure of the PR #912 recorded mediums"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@b40f7222c261839f9b729dc6375bf9c96f0dc395; qsl-eval/tests/it/dispatch_calls.rs, qsl-package/src/emit/tests/receiver_frames.rs, qsl-semantics/tests/it/model_conformance.rs, spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md, spec/test-cases/TC-573-receiver-scope-in-frame-runs-admission-and-emission.md"
review_set: subset
---
# Gap analysis of quire-spec-language PR #959

## Summary

Ticket: QSL-522. This is a planless gap analysis scoped to the PR diff. It is
a tests-only follow-up to merged PR #912 and checks oracle strength, trace
honesty, and closure of #912's three recorded mediums (SR-11754 FND-001 to
FND-003). Plan completion: not assessed. Static review only: no test was run.

Examined:
- FR-154-AC-4 (TC-573): "The S4 frame node of `increment` under
  `modifies self.value` carries the scope `receiver`, and under
  `modifies value` the scope `every-object`; the two nodes' contents differ.
  A frame whose `modifies_receiver` and `modifies` both name `value` is
  refused at intake with `invalid_model_binding`/`conflicting-binding` naming
  both entries and `increment`, and no frame node is emitted for it."
  The test is `the_two_scopes_of_one_field_differ_in_the_node_and_both_at_once_refuse_at_intake`.
  It asserts scope `receiver` on `own`, `every-object` on `all`, and that the
  bodies differ. It also asserts exactly one intake refusal with
  `invalid_model_binding`, cause `ConflictingBinding` keyed to the operation
  `Counter/own`, and a detail naming `modifies[0]` and `modifies_receiver[0]`.
  "No frame node is emitted" is shown by intake refusing: `admit_unit` is the
  intake both the assembler and `assemble_with_models` call. Binding
  correct. The fixture names the operation `own`, not the AC's `increment`.
- FR-154-AC-7: the AC-4 tag is removed from
  `a_scope_or_a_scoped_delete_changes_the_node_id_and_the_package_id`, so
  the test now tags only what it asserts. Binding correct.
- QSpec-FR-151 (query-only dispatch target) under FR-154 effects:
  `checked_dispatch_operation_refuses_a_target_with_only_a_receiver_scoped_effect`
  covers `receiver_modifies`, `deletes_receiver` and `deletes_arguments`, one
  at a time. Its trace matches its sibling not-a-query tests.
- FR-082-AC-9 (TC-911 step 3): the receiver-scoped rows of
  `a_wide_domain_and_a_literal_above_i64_max_are_decided_exactly` assert
  `writer == B/set` on the 2^63+1 refusal. If `field_refinement` stopped
  counting `receiver_modifies`, that row would come back ok and `expect_err`
  would panic.

Closure of the PR #912 mediums (SR-11754):
- FND-001 (AC-9 same-type case): not touched by this diff. It is already
  closed on `main` by
  `one_type_with_a_receiver_scoped_and_an_every_object_field_adds_them_up`
  (qsl-semantics/tests/it/model_population.rs:4663).
- FND-002 (AC-4 trace inflation, no TC-573 test): closed.
- FND-003 (no test for scoped effects in `is_query`, `field_refinement` and
  `lowering/model.rs`): closed for `is_query` and `field_refinement`. Closed
  only in part for `lowering/model.rs` `parent_projections`; see FND-001
  below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | A static read does not show that any test fails if `parent_projections` (lowering/model.rs:333-338) stopped reading `receiver_modifies`. The only receiver-scoped writer test is the TC-911 row. It would catch this only if typing `self.fn` at `Narrow` instead of the parent's `Wide` turned the 2^63+1 refusal into an admit. That holds if the clause facts carry the declared type's interval, and that could not be confirmed without running the test. Confidence low. A one-line mutation run would settle it. | qsl-semantics/src/check/lowering/model.rs:333-338; qsl-semantics/tests/it/model_conformance.rs:1089-1093 |

## Verdict

PASS. No major bug and no real gap. Every new test fails if the behaviour it
names is wrong, and no `#[trace]` or doc comment claims more than its test
asserts. The `UnitParts` helper and the dispatch fixture change do not weaken
any existing assertion. #912's FND-002 is closed, FND-003 is closed except for
the low-confidence `parent_projections` note above, and FND-001 was already
closed on `main`.
