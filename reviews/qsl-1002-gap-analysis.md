---
id: SR-16624
title: "Gap analysis of QSL-1002 FR-250-AC-3 coverage"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #1428 (branch qsl1002-solver-budget, head 2595f5f8c); FR-250-AC-3, TC-705; qsl-replay/src/hybrid.rs"
review_set: subset
---

## Summary

Ticket: QSL-1002. This was a planless gap analysis. Plan completion: not
assessed.

`quoin matrix --repo .` (quoin 0.28.4) at head 2595f5f8c shows FR-250-AC-3 as
`tagged`, with 5 binders, all of them the new tests in qsl-replay/src/hybrid.rs.
On origin/main FR-250-AC-3 had no binder.

The new tests carry no doc comments, and no other id is cited in them.

FR-250-AC-3 was compared, clause by clause, against the assertions at
2595f5f8c:

| # | AC-3 clause | Asserted? |
| --- | --- | --- |
| 1 | 1.5 s admitted with a 2 s budget | yes (`from_request`) |
| 2 | "the run uses 2" | no: there is no run; only `resolved()` |
| 3 | 2 stays 2, 0.2 becomes 1 | yes |
| 4 | unset uses the fixture provider's published 60 | tautological: the test passes 60 into `resolved` |
| 5 | time exhaustion, no Cancel, gives TimedOut | yes, but it duplicates AC-2's `a_stopped_solver_settles_incomplete_by_the_budget_that_stopped_it` |
| 6 | distinct from `Cancelled{deadline}` raised on a run whose FR-276 deadline fires | no: `Cancelled` is constructed by hand in the test |
| 7 | wire 1.5, zero, unknown member, empty `hybrid`, empty `limits` each refused "by the hybrid provider before consumption" | the refusal values are asserted on a free function, with no provider and no consumption point |
| 8 | no budget omits both members | yes at `SolverBudget`; `RequestItem` has no `hybrid` member |
| 9 | stored record holds no wall-time member | weak: a substring check on `OutcomeItem` JSON |
| 10 | equal canonical key bytes | only for a helper's bytes, not the FR-292 request key |
| 11 | a `TimedOut` of the shorter one stored by neither | no |
| 12 | 1,000 steps gives `LimitReached` | only by a test tagged AC-2 |
| 13 | unset steps uses the published 1,000,000 | tautological, as for clause 4 |

The PR body and QSL-1035 both state that AC-3 is covered only in part.
QSL-1035 lists TimedOut-not-stored, the `hybrid.limits` request-item wiring,
the process-provider path and the CG-side refusals as still open.

## Verdict

**CHANGES REQUESTED.** The tests assert real behaviour of the clauses they
reach: the rounding red checks fail, and the refusals and key bytes are
correct. However, an `FR-250-AC-3` tag turns the computed matrix row to
`tagged` for the whole AC, while 6 of its 13 clauses are unasserted or
asserted only tautologically. The matrix cannot carry "in part", so the tag
overclaims (FND-001).

Narrow the five tags to `#[trace("TC-705")]`, a TC-only form already used in
this repo (for example `#[trace("TC-166")]`), and keep FR-250-AC-3 out of
every doc comment. QSL-1035 should then add the AC-3 tag when the request-item
wiring, provider refusal and store clauses land. Splitting AC-3 through
/specify is the alternative.

FND-002 and FND-003 concern tests that add no AC-3 evidence.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Five tests tagged `#[trace("TC-705", "FR-250-AC-3")]` flip FR-250-AC-3 from untagged to `tagged` in the computed matrix. They do not assert "the run uses 2", `Cancelled{deadline}` from a real FR-276 deadline, refusal by the hybrid provider before consumption, the FR-292 canonical request key, or "a TimedOut result ... stored by neither". The tag claims the whole AC while the PR body and QSL-1035 say it is partly covered. Narrow the tags to `#[trace("TC-705")]` until QSL-1035 lands the remaining clauses, or split AC-3. | qsl-replay/src/hybrid.rs:1019, 1037, 1058, 1097, 1113 |
| FND-002 | medium | `a_time_stop_settles_timed_out_distinct_from_a_deadline_cancellation` adds no AC-3 evidence. Its "distinct from Cancelled" assertion compares the result with a `Cancelled{Deadline}` value the test builds itself, which no code produces, so it cannot fail. Its TimedOut value and its no-`seconds` item check repeat the AC-2 test's exact-JSON assertion at hybrid.rs:863-883. The `as_str` spellings are already asserted in proof_result.rs:1861. Drop the test, or replace it with one that drives an FR-276 deadline through a run. | qsl-replay/src/hybrid.rs:1113-1140 |
| FND-003 | low | The "unset members take the published bounds" assertions pass the literals 60 and 1,000,000 into `resolved()` and then expect those same literals back. That tests only `Option::unwrap_or`, never the TC-705 fixture descriptor's published `bounds` (`SolverBounds{max_solver_steps: Some(1_000_000), max_solver_time: Some(60)}` in qsl-route/tests/it/route_registry.rs). The test name and the AC-3 claim overstate it. | qsl-replay/src/hybrid.rs:1030-1033 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 (bba4e0735): all four remaining tests are tagged `#[trace("TC-705")]` only. `grep FR-250-AC-3 qsl-replay/src/hybrid.rs` finds nothing, so no AC-3 id appears in any tag or doc comment, and FR-250-AC-3 returns to untagged until QSL-1035 lands the remaining clauses. |
| FND-002 | fixed | Round 1 (bba4e0735): the tautological TimedOut-vs-Cancelled test is deleted. Its slot now holds `budget_members_are_positive_counts_up_to_two_to_the_53_minus_one`, which is tagged TC-705 only. |
| FND-003 | fixed | Round 1 (bba4e0735): the "published bounds" claim is dropped. The test is now `..._unset_members_take_the_supplied_bounds`, uses the arbitrary bounds 7 and 9, and claims no AC. |
