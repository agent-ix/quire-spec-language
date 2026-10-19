---
id: SR-12144
title: "Gap analysis of quire-spec-language PR #973: E5 check_model phase 1"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@fcb94559221fd86a5610e71f091a5396eaa5b302; PR #973 against FR-124-AC-2/3, FR-125 (subject and letters seam), FR-126 (TP-1 and deadlock: AC-2, AC-3, AC-5, AC-8, AC-9), FR-127 (AC-2, AC-5, AC-10), FR-128 (AC-2, AC-4, AC-5); computed matrix from quire matrix --scope . at this sha"
review_set: subset
---
# Gap analysis of quire-spec-language PR #973

## Summary

Ticket: QSL-526. Plan completion: not assessed.

The computed matrix (`quire matrix --scope .`) was compared with the PR's stated coverage and with what each bound test asserts. The tests have strong oracles where they exist. Expected paths come from an independent breadth-first enumeration (`pairs()`) or from Counter arithmetic. Every reported post-state is re-executed. Counterexample kind, `over_binding`, `where` position, locus, cause and state-key digest are asserted exactly. `Refuted` is asserted only through a real `replay_model_trace`. Parity, refusal and fault each settle non-refuted. No assertion is a bare no-panic check.

The gaps are in what the matrix claims. Several criteria are bound by tests that cover one half of the criterion. Two criteria the PR claims are untagged, and one of them cannot pass yet. The optional semantic review (intent↔test↔code) was not run as a separate pass. Its oracle-strength part was done inline, as the brief asked.

The PR's later-phase list (FR-126-AC-1/4/5/6, the TP-2/TP-4/lasso/interval halves, FR-127-AC-1/3/4/6-9, FR-128-AC-1/3/6) is recorded as deliberately deferred, not as gaps.

## Verdict

**CONDITIONAL**: two medium and three low findings, none blocking. The replay identity gap that blocks the merge is SR-12143 FND-001. Its untested FR-128-AC-4 clause is FND-002 below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Coverage inflation: the computed matrix shows these criteria `tagged`, but each binding test covers only part of the criterion. **FR-126-AC-2**: the TP-1 half only, over Counter rather than the example subject; no TP-2 rows. **FR-126-AC-3**: the deadlock half only; the terminal-stutter lasso is untested. **FR-126-AC-9**: the TP-1 and deadlock halves only; the TP-4 claim, the `kind: Formula` conjunction and the TP-2 `Holds` are untested. **FR-128-AC-2**: the `When`-unit envelope settling `ReplayParity` is untested, and `model_settle.rs`' header admits a terminal-declared unit cannot be recompiled. **FR-128-AC-3**: only the prefix refusals are tested; lasso, fairness-set, interval, stutter and `observed` refusals are not, yet the PR lists AC-3 as not covered. **FR-127-AC-5**: the obligation-identity clause is unchecked, and `ModelCheckRecord` has no obligation identity to check. Once merged, later phases will read these as done. Retag them to the TC only, or record the uncovered halves on the ticket. | qsl-analyze/tests/it/model_check.rs:360-366; qsl-analyze/tests/it/model_check.rs:194-200; qsl-analyze/tests/it/model_check.rs:410-451; qsl-analyze/tests/it/model_settle.rs:8-15; qsl-analyze/tests/it/model_settle.rs:236-242; qsl-analyze/tests/it/model_settle.rs:281-291; qsl-analyze/tests/it/model_settle.rs:535-541 |
| FND-002 | medium | The PR body claims FR-128 covered apart from AC-1/3/6, but the matrix shows FR-128-AC-4 and FR-128-AC-5 untagged. FR-128-AC-5 is the TP-1 clause's undefined-evaluation replay, and it cannot pass at this sha because every authored-clause payload refuses `UnknownTemporalClause` (FR-337). Its deadlock-item analogue (`an_undefined_guard_counterexample_reproduces_only_where_it_is_undefined`) is tagged TC-539 only. FR-128-AC-4's "`clause_node` naming another clause refuses `content-mismatch`" is unimplemented for the deadlock item (SR-12143 FND-001) and untested. Correct the PR body's coverage list. | qsl-replay/src/model_check/replay.rs:201-207; qsl-analyze/tests/it/model_settle.rs:353-360 |
| FND-003 | low | No engine test reaches `Undecided(UndecidedSuccessor)`: neither a refused contract conjunction (`contracts`) nor a refused `terminal when` (`TerminalClass::Undecided`). Only the settlement row is tested, with a hand-built outcome. The engine branches that must never settle a verdict when the successor relation is unknown have no test. FR-126-AC-6 holds the vector and is deferred, so this is low. | qsl-analyze/src/model_check/check.rs:571-573; qsl-analyze/src/model_check/check.rs:613-615; qsl-analyze/tests/it/model_settle.rs:461-475 |
| FND-004 | low | FR-124-AC-3's clause "Adding `fair weak inc` to a request's items changes none of these classifications" is not exercised. The test argues it from the classifier's signature, which takes no fairness input, rather than running the deadlock item with fairness present. | qsl-analyze/tests/it/model_check.rs:161-192 |
| FND-005 | low | FR-125-AC-4's `NoInitialState` half is tested (`an_empty_or_unbounded_subject_and_an_undecided_form_explore_nothing`) but not tagged. FR-125-AC-1/2 are untagged although the PR claims the FR-125 subject and letters seam, so the matrix under-reports what this PR does cover. | qsl-analyze/tests/it/model_check.rs:632-647 |

## Coverage

- Plan completion: not assessed
- Semantic review: not run as a separate pass. Test-oracle strength and soundness were checked inline (see Summary and SR-12143).
- In-scope criteria tagged by this PR: FR-124-AC-2, FR-124-AC-3, FR-126-AC-2, FR-126-AC-3, FR-126-AC-8, FR-126-AC-9, FR-127-AC-5, FR-127-AC-10, FR-128-AC-2, FR-128-AC-3. FR-127-AC-2 was already tagged.
- In-scope criteria claimed but untagged: FR-128-AC-4, FR-128-AC-5; FR-125-AC-1, FR-125-AC-2, FR-125-AC-4.
- Deliberately deferred (PR body): FR-126-AC-1/4/5/6/7, FR-127-AC-1/3/4/6-9, FR-128-AC-1/3/6, and the TP-2/TP-4/lasso/interval halves.
- Reverse gap: none. Every new public item traces to FR-124 to FR-128 or to ADR-018.
- Stubs: none. No `todo!`, `unimplemented!` or placeholder returns in the diff.

## Dispositions

Round 1, reviewed at 72346678bf8568550ea3362a10a53f53960c293a. `quire matrix --scope .` was re-run at this sha.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 72346678b. The doc comments of the partly covering tests now start with "Verifies the ... half of FR-x-AC-y, not ..." and no longer lead with the criterion id. The recomputed matrix shows FR-126-AC-2, FR-126-AC-3, FR-126-AC-8, FR-126-AC-9, FR-127-AC-5, FR-128-AC-2 and FR-128-AC-3 as `untagged`. The tests remain bound to their TCs. |
| FND-002 | fixed | 72346678b. The PR body now separates "Covered in full" (FR-124-AC-2, FR-124-AC-3, FR-127-AC-10) from "Covered in part", with the uncovered halves named. The FR-128-AC-4 `clause_node` clause is implemented and tested for the derived item (SR-12143 FND-001). The guard test's doc says it is the deadlock item's analogue of FR-128-AC-5, which waits on FR-337, and not that criterion. |
| FND-003 | deferred | Low. The engine paths to `Undecided(UndecidedSuccessor)` remain untested here. The vector is FR-126-AC-6, which is deferred to a later phase. |
| FND-004 | deferred | Low. FR-124-AC-3's fairness clause is still argued from the classifier's signature. Recorded under the review policy (only major bugs block). |
| FND-005 | deferred | Low. FR-125-AC-1/2/4 remain untagged. The PR body lists FR-125 as covered in part ("subject and letters seam"), so nothing over-claims. Recorded under the review policy. |
