---
id: SR-12133
title: "Code review of quire-spec-language PR #959: TC-573 and scoped-effect tests closing PR #912 mediums"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@b40f7222c261839f9b729dc6375bf9c96f0dc395; qsl-eval/tests/it/dispatch_calls.rs, qsl-package/src/emit/tests/receiver_frames.rs, qsl-semantics/tests/it/model_conformance.rs"
review_set: subset
---
# Code review of quire-spec-language PR #959

## Summary

Ticket: QSL-522. Code review with the rust-review lane, scoped to
`git diff origin/main...HEAD` (three test files, tests only). Static review:
no cargo or make run, per the dispatch brief.

Examined:
- `qsl-eval/tests/it/dispatch_calls.rs`: new
  `checked_dispatch_operation_refuses_a_target_with_only_a_receiver_scoped_effect`
  and its fixture change (a `model.A.x` field member pushed for the
  receiver-scoped case).
- `qsl-package/src/emit/tests/receiver_frames.rs`: the `UnitParts` /
  `unit_of` split out of `emit`, the new TC-573 test
  `the_two_scopes_of_one_field_differ_in_the_node_and_both_at_once_refuse_at_intake`,
  and the FR-154-AC-4 tag removed from the TC-1043 AC-7 test.
- `qsl-semantics/tests/it/model_conformance.rs`: `receiver_scoped_writes`
  helper and the two receiver-scoped rows added to
  `a_wide_domain_and_a_literal_above_i64_max_are_decided_exactly`.
- Context only: `qsl-semantics/src/model/intake.rs` `resolve_frame`
  (conflicting-binding detail), `qsl-semantics/src/check/checked_dispatch.rs:958`,
  `qsl-semantics/src/model/domain_package.rs` `written_fields`/`is_empty`,
  `qsl-package/src/emit/tests.rs` `assemble_with_models`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The conflicting-binding detail string has a run of about 22 spaces inside it ("to every object at                      {every_object_at}"), because a source line break lacks its trailing backslash. This is pre-existing production code outside the diff. The new TC-573 test checks the detail by substring only, so it does not catch it. | qsl-semantics/src/model/intake.rs:4586 |
| FND-002 | low | The TC-573 test re-runs S1, S2 and `admit_unit` by hand instead of reusing `assemble_with_models`, which follows the same path but returns a `String` error. It also has function-local `use` items. This duplicates code but does not weaken the test. | qsl-package/src/emit/tests/receiver_frames.rs:248-300 |

## Verdict

PASS. No major bug. The `UnitParts` split leaves `emit` unchanged: it
destructures the same unit, document, packages and digest it built inline
before, so no existing assertion is weakened. The dispatch fixture change only
adds the `model.A.x` field member that normalization needs before a
receiver-scoped write reaches the query-only check. It applies only to the new
test, and the three sibling not-a-query tests are untouched. The new dispatch
test fails if `is_query` drops any of the three scoped members: the call then
links and `expect_err` panics, or it refuses with a cause other than
`NotAQuery` and the match panics. The `FR-154-AC-4` tag is correctly removed
from the AC-7 test. The two lows are recorded, not blocking.
