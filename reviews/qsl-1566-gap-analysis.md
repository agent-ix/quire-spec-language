---
id: SR-2239002
title: "gap analysis of QSL-1566 FR-267 temporal wire (quire-spec-language PR 2239)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f2de892fc132e076b6f43d8e39209b14a888ae66; spec/functional/FR-267-write-run-results-as-native-run-result-2.md, spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md, qsl-replay/src/spine/native_result/temporal.rs, qsl-replay/src/spine/clause/tests/temporal/wire.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-267
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1160
    type: reviews
---
# SR-2239002: gap analysis of QSL-1566 FR-267 temporal wire (PR 2239)

## Summary

Ticket: QSL-1566. Plan completion: not assessed. Scope: TC-1160's criteria
this PR touches (FR-267-AC-13, AC-14, AC-15, AC-16, AC-18, FR-266-AC-4) and
TC-769 FR-285-AC-7, against the tagged tests in
`qsl-replay/src/spine/clause/tests/temporal/wire.rs` and the writer/reader in
`native_result/temporal.rs`.

Bindings checked: `a_lasso_violation_writes_its_observed_counterexample` to
AC-13 and FR-266-AC-4 (correct; FR-331 replay of the bytes is not exercised,
as the PR says); `each_temporal_report_writes_its_row_and_each_edit_refuses` to
AC-14 (partial, FND-002); `temporal_only_outcomes_refuse_outside_their_rows`
to AC-16 (partial, FND-002); `an_indeterminate_run_keeps_its_label_and_reason_in_machine_output`
to FR-285-AC-7 and AC-18 (correct); `finite_window_rows_round_trip_and_each_edit_refuses`
and `a_progress_contradiction_round_trips_its_typed_dimensions` to AC-18
(correct).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-267-AC-15 is in TC-1160's scope but no test is tagged with it, and the `insufficient-data` row this PR now writes has no write/read test at all (the `min_count 5` `Windowed` run, the `lookup ... absent-key` undefined, and FR-328-AC-1's `violated` with `interval` `null` over a `Finite` prefix are all unexercised; `window-limit-reached` is covered only under AC-18). Add a test tagged `#[trace("TC-1160", "FR-267-AC-15")]`. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:478, qsl-replay/src/spine/clause/tests/temporal/wire.rs:124-303 |
| FND-002 | medium | AC-14 and AC-16 are tagged but only partly asserted. Missing from AC-14: the empty `Finite` report (`variant` `empty-trace`), the edit adding `temporal {"kind": "holds"}` to FR-109-AC-3's stage `admit` document, and `condition-refused` (see FND-003). Missing from AC-16: adding `temporal` `holds` to FR-109-AC-3's exhausted-work (stage `evaluate`) report, and the `insufficient-data` reason on FR-109-AC-7's `precondition-false` report. Add these cases or narrow the tags. | qsl-replay/src/spine/clause/tests/temporal/wire.rs:124-303, spec/functional/FR-267-write-run-results-as-native-run-result-2.md:477-479 |
| FND-003 | medium | The `condition-refused` row is not implemented: a refused `holds` condition of a `Temporal` selection is still written with no `temporal`, though FR-267 lists the row and QSpec FR-352 says `temporal` is present exactly for that case. The PR body states this as remaining work but names no follow-up ticket. File one (or fix it here) and cite it. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:229, qsl-replay/src/spine/native_result.rs:2279-2310 |
| FND-004 | low | The AC-18 test uses a different fixture from the AC text: the AC names an `Indeterminate` at position 2 with `MissingValuation` (position 1, atom `c.value = 2`); the test's run gives atom `holds(c.value > 0)` at the run's own position, and the AC's `PendingWindow` deadline 4 is asserted as deadline 5. Align the AC's example with the fixture, or the fixture with the AC. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:480, qsl-replay/src/spine/clause/tests/temporal/wire.rs:305-360 |

## Verdict

CONDITIONAL. The new rows have strong edit-refusal coverage; the gaps are an
untagged AC (AC-15), partial AC-14/AC-16 coverage and the unticketed
`condition-refused` row.
