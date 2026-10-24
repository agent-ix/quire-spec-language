---
id: SR-12808
title: "Gap analysis of quire-spec-language PR #966: FR-194 acceptance criteria against TC-629 tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3d62446e19b1e53a3a92e9433a452b8dbf076f26; PR #966: FR-194-AC-1 to FR-194-AC-4 against qsl-eval/tests/it/window_aggregates.rs and qsl-eval/src/temporal/window.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #966

## Summary

Ticket: QSL-572. Planless gap analysis of FR-194 against the PR's tests,
from the computed matrix (`quire matrix --scope .`, quire 0.36.2, engine
0.50.2), plus a focused intent check of each tagged test.

The matrix reports all four criteria as `tagged`: AC-1 and AC-2 by two tests
each, AC-3 by three and AC-4 by two, all in
`qsl-eval/tests/it/window_aggregates.rs`. AC-1 and AC-2 are fully exercised
at the evaluation layer, and their oracles would fail on the wrong behaviour.
A floor quantile rank fails the `q = 1/2, n = 3` case, and a window that
reads ahead fails the `visited` assertions. AC-3's `InsufficientData` and
work-count clauses are exercised. AC-3's run-level `Incomplete(LimitReached)`
clause is exercised at atom level only.

Every test was checked for whether it would fail on the old or wrong
behaviour. There was no old behaviour, because the module is new. Against
plausible wrong implementations, all assertions are discriminating except the
limit boundary (code-review SR-12807 FND-003) and the per-position versus
per-value charge (SR-12807 FND-004).

No untested production code: every public item in `window.rs` is reached
by the TC-629 tests. No stubs.

## Verdict

CONDITIONAL. There are no untagged criteria, but AC-4 is tagged on tests that
cover less than half of it, and no follow-up ticket for the S3 half was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Coverage inflation on FR-194-AC-4. Its two tagged tests cover term refusals (`quantile 1`, `min_count 0`) and the Boolean signal read by the temporal evaluator. They do not cover the S3 clauses: refusing an aggregate over a model subject, the S3 refusal of `past[0, 300 s]` under event-position (only a runtime `WindowKindMismatch` is tested), refusing a threshold of another dimension, horizon equality with and without the aggregate atom, and a holding result that is `tested` with evidence `Observed`. The matrix shows AC-4 as covered. A search of Linear (`ix-board search`, "window aggregate" / "FR-194") found no follow-up ticket for the deferred S3 half, so it would close silently. Fix: file the S3 follow-up and link it from QSL-572 before merge, or keep AC-4 open in the tracker. | qsl-eval/tests/it/window_aggregates.rs:359-361; qsl-eval/tests/it/window_aggregates.rs:513-515 |
| FND-002 | medium | FR-194-AC-3's clause "the run stops `Incomplete(LimitReached{…})`" is tested only at atom level (`AggregateStop::LimitReached`). No test drives `temporal::evaluate` to that stop, and the seam cannot carry it (SR-12807 FND-001). | qsl-eval/tests/it/window_aggregates.rs:297-312 |
| FND-003 | low | FR-194-AC-1's "the threshold `0.005 s` gives the same results" is exercised by dividing the values by 1000 beforehand, not by converting the threshold (the FR-142 checker step, which is out of scope). The tag reads as more than the test shows. | qsl-eval/tests/it/window_aggregates.rs:136-151 |

## Coverage

- Criteria examined: FR-194-AC-1, FR-194-AC-2, FR-194-AC-3, FR-194-AC-4 (4 of 4 tagged).
- Fully exercised at evaluation layer: AC-1 (except the threshold conversion), AC-2.
- Partially exercised: AC-3 (run-level stop), AC-4 (S3 half, horizon and evidence).
- Semantic review: focused per-AC intent check done inline. No per-FR fan-out.
- Plan completion: not assessed

## Dispositions

Round 1, reviewed at `e52200e1bd4915a04840647411cd0809683b5736`. No new
findings.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-864 (E17w-b) now tracks the S3 half of FR-194-AC-4: model-subject refusal, window-kind refusals, threshold dimension and conversion, zero horizon, Observed evidence. The AC-4-tagged tests were narrowed to what they show, and the remaining refusals moved to an untagged test. |
| FND-002 | fixed | `a_window_over_the_limit_settles_the_run_with_the_limit_record` (tagged FR-194-AC-3) asserts that the run outcome is `WindowLimitReached{max_window_values, 2, monitor.max_window_values}` with category Incomplete, distinct from a work-budget Incomplete. |
| FND-003 | deferred | QSL-864 tracks threshold unit conversion. The test is renamed `..._in_either_prescaled_unit`, and its comment now says it does not test conversion. |
