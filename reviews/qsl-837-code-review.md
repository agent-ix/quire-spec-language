---
id: SR-968
title: "code-review of QSL-837 parse CLI retarget (quire-spec-language PR 2137)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; src/main.rs; src/cli.rs; tests/it/cli.rs; tests/it/deep_sources.rs; Makefile"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-002
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-010
    type: references
---

# Code review of QSL-837 parse CLI retarget (quire-spec-language PR 2137)

## Summary

Ticket: QSL-837. `git diff origin/main...HEAD` at dd64c7e54, read-only, no cargo run. The retarget itself is sound: S1 then S2 under `qsl_cst::Limits`, `OutcomeDocument::from_parse`, FR-285 `Category::exit_code` (success to stdout, others to stderr), argument cap raised 13 to 15 correctly (command + 5 + 4x2 options = 14, +1). Nothing is deleted: `quire_spec_language::parse` and its 0-draft tests stay. Defects: the Makefile fixture change breaks three xtask tests that pin the recipe, and the test changes leave the new `--work-units` wiring and the CLI label forwarding unproven.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The Makefile ci-clean-build recipe now ends in spine-compile.native, but three xtask tests assert the old line verbatim and are not updated, so they fail. | xtask/src/ci_clean_build_tests.rs:122,206; xtask/src/ci_feature_lane_tests.rs:607 | missing-requirement |
| FND-002 | medium | No test shows --work-units reaches the parser: the only value passed is 256, the default, and FR-256-AC-5 requires raising s1.work_units to fit. Ignoring the option value would pass. | tests/it/deep_sources.rs:250-258; src/cli.rs:172 | missing-requirement |
| FND-003 | medium | parse_and_format_take_the_four_source_labels and cli_preserves_non_utf8_os_paths (TC-015/TC-017) now assert the labels only on an in-process qsl_replay::spine::parse call; the CLI is asserted for exit 0 only, so a CLI that swapped or dropped labels passes. | tests/it/cli.rs:105-130,344-365 | missing-requirement |
| FND-004 | medium | parse still emits the legacy {"status":"refused","code":...} object for a blank source label (and plain text for read errors); only parse outcomes are quire-outcome/1 documents, so one command has two stderr shapes, and the test pins the legacy one. | src/main.rs:140-150; tests/it/cli.rs:196-207 | missing-requirement |
| FND-005 | low | Assertions removed with no successor: the region-less S0 refusal span byte 0, the authority/identity/revision members on a refusal, and the display path in output. | tests/it/cli.rs:51-57 | missing-requirement |

## Verdict

Not mergeable until FND-001 is fixed. FND-002 to FND-004 should be fixed in the same PR. The node limit was raised to 10,000,000 in the last commit; cli_parse_takes_caller_limits is the only test that depends on it and was not shown re-run, so the full cli and deep_sources filters plus the xtask ci_clean_build and ci_feature_lane filters must be shown green on the final head.
