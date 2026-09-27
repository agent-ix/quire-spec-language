---
id: SR-710
title: "Gap analysis of FR-064 acceptance criteria against the string-edge tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@9686fc5f872dc1e3fc7317f12fc7de44bec0707c; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md; spec/test-cases/TC-162-string-edge-scan-coverage.md; xtask/src/string_edge.rs; Makefile"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-064
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-162
    type: reviews
---
## Summary

Ticket: QSL-145 (cross-reference QSL-268). PR: quire-spec-language#485 at
9686fc5f. This maps each FR-064 acceptance criterion to the tests in
`xtask/src/string_edge.rs` that back it. The reviewer ran
`cargo test -p xtask --lib string_edge`: 17 passed, 0 ignored.

| AC | Tests | State |
| --- | --- | --- |
| AC-1 | `marked_function_is_silent_unmarked_is_reported`, `string_match_is_reported` (tagged) | backed |
| AC-2 | `allow_listed_occurrence_is_silent_removing_the_entry_reports_it_again` (tagged) | backed |
| AC-3 | `cfg_test_module_is_not_scanned`, `file_level_test_modules_are_not_scanned` (tagged) | backed. The second test also asserts behaviour beyond the AC (SR-709 FND-006) |
| AC-4 | `a_non_empty_report_exits_non_zero_a_clean_scan_exits_zero` (tagged) | backed |
| AC-5 | real-site test and combinator fixture (tagged). The two-entry half and the five-site half are `branch_gating_is_distinguished_from_a_non_branching_sink`, `allow_list_entry_at_a_branch_gating_occurrence_is_rejected` and `none_of_the_five_adr010_production_sites_can_be_allow_listed`, all untagged | partly backed (FND-001, FND-002) |
| AC-6 | `the_real_makefile_wires_string_edge_into_ci`, `a_failed_prerequisite_fails_the_aggregate_target` (tagged) | backed. Also measured: `ci:` lists `string-edge` (Makefile:116) |

Mutation evidence, run by the reviewer: turning off the `&&`/`||` widening
fails the real-site test. Scanning with marks honoured fails it too (the
site is marked). So the un-ignored test would catch the detector no longer
seeing a real site.

## Verdict

**Changes requested.** AC-5's real-site half covers 3 of the 4 ADR-010
§4.3 sites still in the tree. The scan also under-implements FR-064's "What
the scan reports", and real post-edge dispatch is invisible to it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-5 says "each of the five ADR-010 §4.3 production dispatch sites". The real-site test covers `Profile::classify`, `valid_digest` and `valid_adapter` only. The `clock:` site still exists (src/temporal.rs:56-57). It was removed from the list because the literal-only scan cannot see `strip_prefix(CLOCK_PREFIX)`. The real-site half is therefore not backed for one in-tree site, and the `strip_prefix` widening has no real-site coverage. | xtask/src/string_edge.rs:1060-1130; src/temporal.rs:50-58 |
| FND-002 | medium | FR-064 Behavior requires reporting a comparison between a `&str`/`String` "and a string literal or another `&str`/`String` value". The scanner is literal-only and does not see named consts. Measured post-edge string dispatch it misses: `identity == NARROW` / `identity != NARROW` (qsl-semantics/src/check/claims.rs:76, 648), a `name == FUNCTION_PARAMETERS` match guard (qsl-semantics/src/check/lowering.rs:240), and `semantic_form() == ENUM_VALUE_FORM` (qsl-package/src/emit.rs:397). The limit predates this PR, but this PR flips FR-064 to "Implemented" and "all six ACs backed". Record the limit as an unbacked part of Behavior with an owner, or widen the scan to const operands. | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:55-58; qsl-semantics/src/check/claims.rs:76; qsl-semantics/src/check/claims.rs:648; qsl-semantics/src/check/lowering.rs:240; qsl-package/src/emit.rs:397 |
| FND-003 | low | The three tests that back AC-5's two-entry half and its five-site fixture half carry no `#[trace("TC-162", "FR-064-AC-5")]`. FR-064's new Status says "with the tracking tags in `xtask/src/string_edge.rs` (all `TC-162`)" and cites these halves as backed. | xtask/src/string_edge.rs:800-801; xtask/src/string_edge.rs:857-858; xtask/src/string_edge.rs:1023-1024 |
