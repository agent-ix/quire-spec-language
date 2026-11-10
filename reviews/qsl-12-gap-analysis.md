---
id: SR-(commit)
title: "Gap analysis of l5/p3-matrix (Phase 3 PR 1: qsl-conformance lifecycle matrix)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language(commit); diff origin/l5/integration((commit))...HEAD; spec read from origin/main (commit)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-564
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-565
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-566
    type: reviews
---

## Summary

Ticket: QSL-12. Plan completion: not assessed. A planless gap analysis, scoped to the PR diff. The branch does not contain #2587's spec text, so the ACs were read from `origin/main`. Each test↔AC binding was checked by reading the test against its AC. `quoin matrix` was started but did not finish within the review window, so the bindings below are manual.

## Verdict

**PASS WITH FINDINGS.** Every new production path in the diff has an owning requirement: FR-564/565/566 for `qsl-conformance`, FR-281/FR-286 for the `document()` methods and `DeadlineSupply`, FR-286 for `from_stopped`. Every new test carries a `#[trace]` tag. No stubs and no criterion without a test among the cells the PR builds.

The only gaps are partial ACs whose tags read as full coverage, and one AC clause with no test.

## Bindings

- `check_returns_each_applicable_kind` → FR-564-AC-1: correct, complete.
- `the_incomplete_cells_are_one_below_the_counter_their_input_reaches` → FR-564-AC-1, AC-2: correct.
- `package_returns_each_applicable_kind` → FR-564-AC-2: correct, complete.
- `analyze_returns_each_reachable_kind` → FR-564-AC-4: partial (4 of 7).
- `monitor_returns_each_applicable_kind` → FR-564-AC-5: correct, complete.
- `prove_yields_an_item_of_each_reachable_kind` → FR-564-AC-6: partial (4 of 8).
- `cells_are_in_fr564_order_with_not_applicable_at_its_positions` → FR-565-AC-1: partial (33 cells, no Execute verb, 7 of 8 n/a).
- `every_case_gives_its_expected_stable_document` → FR-565-AC-2, FR-564-AC-7: correct, with a weak identity check.
- `every_case_gives_the_same_outcome_in_an_empty_working_directory` → FR-565-AC-3: correct.
- `every_case_is_unchanged_by_the_registered_descriptors_and_the_plugin_provider`, `configuration_3_runs_an_item_on_the_plugin_provider` → FR-566-AC-1: package-bytes clause missing.
- `the_plugin_adds_itself_only_to_the_kind_it_advertises` → FR-566-AC-2: correct.
- `analyze_never_measures_its_deadline_and_sees_it_only_through_the_handle` → FR-281-AC-12: correct. The compile_fail doctest half carries no trace tag, since doctests cannot carry one.
- FR-564-AC-3 (TC-1390): untagged and unbuilt, as stated by the lead.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-565-AC-1, FR-564-AC-4 and FR-564-AC-6 are tagged by tests that cover only part of each AC (33 of 48 cells with no Execute verb, 4 of 7, 4 of 8), so the computed matrix will report the three ACs as covered. Either drop the tags until the AC is complete, or record the partial coverage on the ticket and leave the AC open. | qsl-conformance/tests/it/matrix_shape.rs:199, qsl-conformance/tests/it/analyze_row.rs:10, qsl-conformance/tests/it/prove_row.rs:623 |
| FND-002 | low | FR-566-AC-1's package-bytes clause ("FR-275-AC-1's package bytes are byte-equal in the three") has no test. | qsl-conformance/tests/it/plugin_invariance.rs:476-494 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | After #2588, `analyze_returns_each_reachable_kind` and `prove_yields_an_item_of_each_reachable_kind` cover all of amended FR-564-AC-4 (4 cells) and AC-6 (4 cells), but both are tagged `TC-1391`/`TC-1393` only. FR-564-AC-8 (pending cells carry a reason and no case) is asserted in `cells_are_in_fr564_order...` but untagged. Amended FR-281-AC-3's item clause is asserted only in the untagged TC-1391 test. The computed matrix reports these covered ACs as uncovered. | qsl-conformance/tests/it/analyze_row.rs:12, qsl-conformance/tests/it/prove_row.rs:10, qsl-conformance/tests/it/matrix_shape.rs:24-52, qsl-analyze/tests/it/analyze.rs:114 |
| FND-004 | medium | FR-565-AC-2's amended Err clause (a removed input returns `Err` with `CaseStage::Read`) has no test, and no `Read` stage exists. | qsl-conformance/src/run.rs:78-121 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Partial tags were reduced to the TC id. Restoring them for the now-complete amended ACs is FND-003. |
| FND-002 | accepted-no-change | The package-bytes clause stays uncompared, with the reason in the test doc: package takes neither registry nor engines, so it cannot vary. |
| FND-003 | fixed | Round 2, `(commit)`. The analyze and prove row tests carry `FR-564-AC-4` and `FR-564-AC-6` again. Amended FR-564 widened both ACs, so the tags now overclaim; see FND-005. |
| FND-004 | fixed | Round 2. `CaseStage::Read` exists, and `a_case_with_a_removed_input_fails_at_the_read_stage` tests an emptied unit and an emptied package. |

## New findings (disposition pass 2)

Scope: `git diff origin/l5/p2-render...origin/l5/p3-matrix` at `(commit)`. Bindings were checked by reading each test against its AC.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | `analyze_returns_each_reachable_kind` (FR-564-AC-4) and `prove_yields_an_item_of_each_reachable_kind` (FR-564-AC-6) are tagged as full coverage, and the analyze test's doc says "`refuted` with its counterexample". No `ItemCounterexample` is produced or asserted. That is a declared gap (a stacked branch adds it), but neither the code nor the tags state it, so the computed matrix reports AC-4 and AC-6 as covered. | qsl-conformance/tests/it/analyze_row.rs:18-24, qsl-conformance/tests/it/prove_row.rs:8-10 |
| FND-006 | medium | `a_passing_run_names_the_three_parts_and_qualifies` is tagged FR-567-AC-1 and FR-567-AC-4 but links no backend. AC-1 needs a linked delegating backend. AC-3 (rounding backend gives `parity/jit`) and AC-4's reference-fault half have no test. This is the declared FR-563 gap, but the tags present it as covered. | xtask/tests/qualify_parts.rs:44-60 |
| FND-007 | low | FR-564-AC-7 and TC-1394 step 2 require serializing each document twice and comparing the bytes. `every_case_gives_its_expected_stable_document` compares only the typed values of two runs. | qsl-conformance/tests/it/matrix_shape.rs:61-112 |
| FND-008 | low | `analyze_returns_each_reachable_kind` is also tagged FR-281-AC-3, but it runs one unadapted item through `analyze_items`, not AC-3's three-item request over `Engines::qsl()` with its request-order and proved and refuted members. | qsl-conformance/tests/it/analyze_row.rs:24 |

## New dispositions (round 3)

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | fixed | The FR-564-AC-4 and AC-6 tags and the "with its counterexample" wording are removed until `ItemCounterexample` lands. |
| FND-006 | fixed | The FR-567-AC-1 and AC-4 tags are removed; the test doc and `parity_part` state that the linked-backend, `parity/jit` and `reference-fault` cases wait for FR-563's runner. |
| FND-007 | fixed | `every_case_gives_its_expected_stable_document` serializes each document twice and compares the bytes (FR-564-AC-7). |
| FND-008 | fixed | The FR-281-AC-3 tag is removed from the analyze row test. |

## Reviewer dispositions (round 3)

The reviewer verified these against the fix commit on the branch. These rows hold over the coder's "New dispositions (round 3)" section above.

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | fixed | The analyze and prove row tests carry only `TC-1391`/`TC-1393`. Their docs say FR-564-AC-4/AC-6 stay open until `ItemCounterexample` lands, and docs/outcome-parity.md lists it under "Known gaps". |
| FND-006 | fixed | `a_passing_run_names_the_three_parts_and_qualifies` is tagged `TC-1396` only, and its doc names the FR-563 dependency. FR-567-AC-1, AC-3 and AC-4 stay open. |
| FND-007 | fixed | `every_case_gives_its_expected_stable_document` serializes each outcome and compares the bytes of the two runs. |
| FND-008 | fixed | The FR-281-AC-3 tag is removed. |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | `the_report_names_the_root_package_version` keeps the full FR-567-AC-5 tag, but it can fail only if the report shows xtask's own version or a version the root manifest does not declare. A hard-coded literal equal to the current version would pass all three asserts. That is exactly the pre-fix `const QSL_VERSION: &str = "0.2.0"`. Only the scratch-copy 9.9.9 half, which is not run, catches that regression. Narrow the tag to the first half, or run the scratch-copy build. | xtask/tests/qualify_parts.rs:63-86 |

## Coder dispositions (round 4)

| FND | outcome | reason |
| --- | --- | --- |
| FND-009 | fixed | The `FR-567-AC-5` tag is removed from the test (it keeps `TC-1396`); the scratch-copy 9.9.9 half needs a rebuild because the version is a compile-time constant, and docs/outcome-parity.md lists it under "Known gaps". |
