---
id: SR-1753002
title: "gap analysis of PR #1753 (QSL-1005): dense-time routing"
type: SpecReview
analysis: gap-analysis
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1753 at d8404b5a4 against FR-243, FR-430-AC-1, FR-144, FR-159/#1690 reductions and FR-331"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1753002: gap analysis of PR #1753 (QSL-1005)

## Summary

Ticket: QSL-1005. Head: d8404b5a4.

Covered:
- FR-243-AC-1/3: `timed_items_over_a_dense_subject_route_to_en6_alone` covers the infinite-trace form and TT-1..TT-4 to EN-6, with the derived deadlock and time-lock items dense.
- `an_item_over_an_untimed_subject_keeps_its_candidates`.
- FR-144-AC-1 with FR-430: `an_untimed_refinement_record_routes_to_explicit_state_alone`.
- FR-430-AC-1: `the_timed_refinement_record_routes_to_en6_alone`.

The FR-331 manifest is unchanged, and the missing manifest label is filed as STD-493 22392db5.

Gaps:
- **FND-001.** The derived deadlock-freedom and time-lock-freedom items' dense marks have no production consumer. `deadlock_freedom_items` and `time_lock_freedom_items` are called only from tests, so nothing routes a derived item through `restrict_to_dense_time`. `dense_route` composes the derived route by hand (`candidates_for_temporal_dynamics` plus `restrict_to_dense_time`). FR-243's derived-item half is therefore shown at the API level only. This predates the PR: no request writer emits derived items yet. It needs a tracking item, not a fix here.
- **FND-002.** No test holds a dense subject and an untimed subject in one package. By code the marks are per record, each from its own `over` model or concrete model. A two-model package asserting `[en-6]` for the dense clause and `[en-1, en-6]` for the untimed one in the same `items_from_requirements` call would pin the mixed case.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The derived deadlock and time-lock items' dense marks have no production consumer (no non-test caller of `deadlock_freedom_items`/`time_lock_freedom_items`); FR-243's derived-item route is tested by hand composition only | FR-243-AC-1, FR-243-AC-3 |
| FND-002 | low | No test of a mixed package (dense and untimed subjects in one `items_from_requirements`) | FR-243, FR-144 |

## Verdict

Mergeable. Both findings are low.

## Dispositions

Round 1, head 1f9e11e6b (also merged into #1755 at 9516a8585). At 9516a8585, `dense_route` and `timed_refinement` pass 20/20, with `--all-features` and debug info off.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Moved to subticket QSL-1205 ("write requests for derived deadlock-freedom and time-lock-freedom items", Backlog, parent QSL-1005). No request writer emits derived items yet, so a production consumer of their dense marks is that ticket's work, not this PR's. |
| FND-002 | fixed | `dense_route::a_mixed_package_routes_each_record_by_its_own_subject` holds the dense `M` and the untimed `A` in one package and routes everything in one `items_from_requirements` call. The clause over `M` routes to `[en-6]` (dense). The clause over `A` routes to `[en-1, en-6]` (not dense). The timed refinement `Timed` routes to `[en-6]`, and the untimed refinement `Same` (A by A) routes to `[en-1]`. The test passes. |
