---
id: SR-733
title: "QSL-245 gap analysis of PR 489 (blank-label and empty-path causes)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-001-AC-6, FR-001-AC-11, FR-001-AC-12, FR-010-AC-11, FR-018-AC-8, FR-026-AC-6; TC-424 steps 2, 6, 7; TC-425 step 3; TC-430 step 2; TC-431 step 2; TC-444 step 5"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: reviews
---
## Summary

Ticket: QSL-245. PR: quire-spec-language#489. Each changed AC is
mapped to the test that backs it. The oracle was checked by mutation in a
detached scratch worktree with a private `CARGO_TARGET_DIR`.
Every mutation below was killed: the named test failed.

| AC / TC step | Test | Mutation | Result |
| --- | --- | --- | --- |
| FR-001-AC-6 / TC-424 s2 (U+3000 blank) | `a_blank_label_refuses_naming_it_and_admits_nothing` (qsl-foundation/src/source.rs) | `trim()` to `trim_ascii()` in `first_blank_label` | killed |
| FR-001-AC-11 / TC-424 s6 | `label_order_precedes_the_path`; `the_reader_reports_blank_label_before_empty_path` (qsl-cst) | path check moved before the label check | both killed |
| FR-001-AC-12 ForeignNode / TC-424 s7 | `rendering_a_foreign_cst_node_is_a_typed_refusal` | `error_without_region` to `error(.., 0, 0)` | killed |
| FR-001-AC-12 EditPredecessor | `a_stale_edit_predecessor_refuses_as_a_source_map_with_no_region` | region at byte 0 restored | killed |
| FR-001-AC-12 RequestRevision | `stale_profile_and_cancelled_editor_requests_are_typed` | region at byte 0 restored | killed |
| FR-010-AC-11 / TC-425 s3 | `cli::parse_and_format_take_the_four_source_labels` | `label` key dropped in `Refusal::render` | killed |
| FR-026-AC-6 / TC-430 s2 | `standalone::run_requests_and_results_carry_the_four_source_labels` | `label` forced to `None` in output; separately, `source_refusal` `identity_cause: None` | both killed |
| FR-018-AC-8 / TC-431 s2 | `runtime_inputs::snapshots_carry_the_four_labels` | `Failure.identity_cause` not set | killed |
| TC-444 s5 | `tc_444_a_blank_label_refuses_at_the_source_stage` | not mutated. It asserts `BlankLabel { Authority }` and `region: None` through `qsl_cst::diagnostic::read_source`, the same mapping the qsl-cst test kills | passes |

No test still asserts `invalid_source_identity` for `EditPredecessor`,
`ForeignNode` or `RequestRevision`. Searching all `*.rs` files for
`InvalidSourceIdentity` finds only blank-label, empty-path and
native-input sites. `an_edit_that_changes_a_source_label_refuses` asserts
only the cause, but it shares the one return site that the stale-edit test
pins.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-001 Status says `blank-label` or `empty-path` reaches the CLI line, the run output, the native-state-input error and the replay refusal. Every surface test covers only `blank-label`. `empty-path` is tested only at the reader level, in the foundation and qsl-cst unit tests. The surface mapping is generic, so the risk is low. But the Status line claims more than the tests show, and it should say that `empty-path` is backed at the reader. | spec/functional/FR-001-read-exact-source.md:228-238; tests/it/standalone.rs:746-748; tests/it/cli.rs:385-387 |

## Verdict

Approve. Every changed AC has a test, and each of these tests fails when its
behaviour regresses. One low-severity overclaim in the Status text.

## Dispositions

Checked against the fix diff on 2026-09-26.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | FR-001 Status (lines 236-243) now limits `empty-path` backing to the reader and to the native `Diagnostic`. The new test `the_native_diagnostic_carries_the_identity_cause` (qsl-foundation/src/diagnostic.rs) fails when `EmptyPath` is dropped from `source_refusal`. The new CLI assertion in tests/it/cli.rs:390-398 shows that an empty operand gives a file error. I checked the unreachability claims against the code. The run path reads `directory.join(file)` through `read_file` before `Source::read_verified` (src/command.rs:432-442), and an empty `file` resolves to the directory, so `read_to_end` fails with `RunCause::Io`. Snapshot and Invocation constructors take no path. Replay passes the reference identity as the path, and that identity is non-empty once the labels are non-blank. |
