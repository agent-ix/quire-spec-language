---
id: SR-20261105101079
title: "Gap analysis of QSL-595 LC7 QSL builds no binary"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-595 LC7; PR #2465 head 939ef1538; FR-287-AC-1..4, TC-771, TC-1262; deleted tests/it/cli.rs, tests/it/compile_command.rs, tests/it/spine_run.rs, 11 tests of tests/it/config_version_spine.rs, deep_sources::cli_parse_takes_caller_limits; src/command.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: reviews
---

## Summary

Ticket: QSL-595. Plan completion: not assessed. This is a scoped gap analysis of the PR #2465 diff against FR-287 (AC-1 to AC-4, TC-771) and the ticket. The PR deletes 66 tests, all of which ran the `quire-spec` binary. The behaviour they covered is still in the library. That covers `src/command.rs`: `compile`, `run`, `run_with_limits` and `read_clause_run_request`. It also covers `qsl_replay::spine::parse` with `OutcomeDocument::from_parse`. None of the deleted tests was ported to an in-process call. Many of the affected criteria say "Through `command` in process", so they should have been ported.

## Verdict

**NOT MERGEABLE.** Deleting the binary is correct for FR-287-AC-1. Deleting its tests without porting them loses coverage of behaviour that still exists. After this PR, 30 acceptance criteria that the deleted tests tagged have no tagged test anywhere in the repository (repo-wide `#[trace]` grep). Nine test cases have no test at all: TC-015, TC-017, TC-103, TC-104, TC-105, TC-109, TC-425, TC-750 and TC-785.

FR-287-AC-2, the second half of TC-771 and part of the FR this PR implements, has no test. The new AC-1 test checks only the bin-target clause.

FR-287-AC-3 and FR-287-AC-4 are driver criteria (TC-1262). QSL is right not to test the `quire` frontend, so two deleted tests are correctly dropped as CLI-only: `compile_cli_uses_fixed_byte_ceilings_and_rejects_limit_options` and `lower_is_an_unknown_command`. The library limit behaviour those tests also covered is still lost (FR-026-AC-4, FR-027-AC-13 and FR-312-AC-3).

The ticket's fixture-audit and `JSON_DEPTH` items are moot because `tools/fixture-audit` no longer exists on main.

## Coverage

Computed matrix: `quoin matrix --json` ran with a 600-second bound and timed out with no output (exit 124), so no computed-matrix result is claimed. Coverage was mapped by hand. Every `FR-*-AC-*` and `TC-*` id tagged by the deleted tests was searched across all remaining `*.rs` files at the PR head.

Deleted test to remaining coverage:

- tests/it/cli.rs (10 tests). All lost except those noted below.
  - cli_usage_io_and_resource_outcomes_are_distinct (FR-010-AC-4, AC-7): lost. FR-010-AC-3 is no longer in the spec.
  - cli_preserves_os_paths_and_refuses_non_utf8_labels_before_io and cli_preserves_non_utf8_os_paths (FR-010-AC-6, AC-7): lost. The non-UTF-8 label refusal is now driver criterion FR-287-AC-6.
  - cli_parses_and_formats_without_claiming_execution (FR-002-AC-1; FR-010-AC-1, AC-2; FR-026-AC-5): lost. Parse success at S2 is still exercised, untagged, by qsl-replay/tests/it/parse_outcome_facade.rs.
  - parse_output_failure_exits_30 (FR-010-AC-10, no longer in the spec): dropped correctly. It is now driver criterion FR-287-AC-7.
  - parse_of_a_recognized_but_unsupported_construct_exits_21 (FR-010-AC-9): lost.
  - parse_of_genuinely_malformed_syntax_exits_20_not_21 (FR-010-AC-2, AC-9): lost. Part of it is exercised untagged by parse_outcome_facade refusals.
  - malformed_operands_report_the_selected_command_before_io (FR-026-AC-5 tag; operand arity): dropped correctly. It is now driver criterion FR-287-AC-3.
  - parse_and_format_take_the_two_source_labels (FR-010-AC-11): lost.
  - work_units_option_lowers_and_raises_the_s1_work_ceiling (FR-256-AC-5): lost.
- tests/it/compile_command.rs (25 tests, all through the binary). The behaviour is in `command::compile`.
  - FR-027-AC-2, AC-3, AC-7, AC-10, AC-11, AC-12, AC-13 and AC-14; FR-031-AC-6; FR-493-AC-2; FR-026-AC-4: lost, with zero remaining tags.
  - FR-027-AC-5, AC-8 and AC-9; FR-056-AC-9; FR-049-AC-11; FR-147-AC-10; FR-003-AC-9: still tagged in qsl-package, qsl-replay and qsl-semantics. Those tests cover the spine stages, not request intake.
  - compile_cli_uses_fixed_byte_ceilings_and_rejects_limit_options (FR-287-AC-4, AC-5) and lower_is_an_unknown_command: dropped correctly as driver CLI tests.
- tests/it/spine_run.rs (19 tests, all through the binary). The behaviour is in `command::run`.
  - FR-100-AC-1 and AC-2; FR-267-AC-3; FR-289-AC-1: lost, with zero remaining tags.
  - FR-100-AC-3 to AC-6: still tagged in qsl-replay/src/spine/call/tests.rs at the call level. The request-decoding refusals (native-only members, malformed `work_units` or argument values) are no longer tested.
  - FR-100-AC-11 and AC-13: still tagged in src/command.rs and src/command/output.rs.
  - FR-267-AC-10: still tagged in qsl-replay clause witness tests.
- tests/it/config_version_spine.rs (11 deleted tests; each used the binary, some also the library).
  - FR-312-AC-1, AC-2, AC-3, AC-4 and AC-6: lost, with zero remaining tags.
  - FR-312-AC-5: still covered by the remaining in-process `read_clause_run_request` tests.
  - FR-108-AC-1 (walkthrough_requests_reproduce_the_documented_exit_codes): still tagged by other TC-469 tests. The documented exit-code walkthrough in examples/config-version/README.md is no longer checked.
  - FR-267-AC-9: still tagged elsewhere.
  - clause_run_reader_distinguishes_pre_and_invocation_roles and clause_run_accounting_object_sets_all_counters_and_rejects_legacy_member: both made in-process `read_clause_run_request` calls and were deleted whole, not trimmed.
- tests/it/deep_sources.rs cli_parse_takes_caller_limits (FR-256-AC-5): lost. The remaining tests in that file are tagged FR-256-AC-4 only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-287-AC-2 (TC-771 step 2) has no test. No test calls the library in process to assert compile success exit 0, zero-work-units run incomplete exit 22, and empty stdout/stderr | tests/it/no_binary.rs:1-38 |
| FND-002 | medium | The FR-287-AC-1 test checks only bin targets. It asserts neither the root `default_run` null nor the no-normal/build-dependency-on-developer-tools clause, so adding xtask as a normal dependency of qsl-cst would pass | tests/it/no_binary.rs:19-36 |
| FND-003 | high | FR-027 request-intake criteria (AC-2, 3, 7, 10, 11, 12, 13, 14) lost every test when compile_command.rs was deleted. `command::compile` remains and was not ported in process; TC-105 and TC-785 have no tests | src/command.rs:459 |
| FND-004 | high | FR-026-AC-4 and AC-5 (request and dependent byte limits, cwd-independent paths through `command` in process) have no test; TC-103 and TC-104 have no tests | src/command.rs:407 |
| FND-005 | high | FR-100-AC-1 and AC-2 (seven completes via native-run/1; unknown or 0-draft edition refuses at profile) lost every test with spine_run.rs; `command::run` is untested end to end | src/command.rs:402 |
| FND-006 | high | FR-312-AC-1, 2, 3, 4 and 6 (clause-run request routing, selection refusals, observation byte limit, function/frame reports, accounting) have no test after 11 config_version_spine tests were deleted rather than ported | tests/it/config_version_spine.rs:282 |
| FND-007 | high | FR-010-AC-6, 7, 9 and 11 (parse over a non-UTF-8 path, missing path refuses at intake, unsupported construct exit 21, two source labels) have no test. AC-1, 2 and 4 are exercised only untagged in parse_outcome_facade.rs | qsl-replay/tests/it/parse_outcome_facade.rs:64 |
| FND-008 | high | FR-002-AC-1 (parse over spine-compile.native returns an admissible ParsedSource with its exact bytes) has no test | tests/it/main.rs:45 |
| FND-009 | high | FR-256-AC-5 (parse takes caller limits as given; defaults refuse the 100,000-deep source, raised limits parse it) has no test after cli_parse_takes_caller_limits and work_units_option tests were deleted | tests/it/deep_sources.rs:180 |
| FND-010 | high | FR-267-AC-3 (unknown edition run returns a native-run-result/2 command-error envelope) has no test | src/command.rs:402 |
| FND-011 | high | FR-289-AC-1 (empty versus three-manifest registry gives byte-equal package output) has no test; TC-773 remains only on other ACs | tests/it/main.rs:45 |
| FND-012 | high | FR-031-AC-6 (extraction request refusals) has no test; TC-109 has no tests | src/command.rs:462 |
| FND-013 | high | FR-493-AC-2 (compile of 0-draft or unknown edition refuses unknown_edition at profile with no package bytes) has no test | src/command.rs:459 |
