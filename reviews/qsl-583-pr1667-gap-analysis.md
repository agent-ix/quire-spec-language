---
id: SR-1667002
title: "gap analysis of PR #1667 (QSL-583e, FR-242 MightyL bounded copies)"
type: SpecReview
analysis: gap-analysis
date: 2026-10-27
scope: "agent-ix/quire-spec-language; PR #1667 git diff origin/qsl-583d-mightyl...7196018e8; FR-242 (Behavior: TT-3/TT-4 by MightyL, agreement with the evaluator, AC-1), FR-234 (punctuality); TC-697"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-242
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-234
    type: reviews
---
# SR-1667002: gap analysis of PR #1667 (QSL-583e, FR-242 MightyL bounded copies)

## Summary

Ticket: QSL-583. Head reviewed: 7196018e8. Method: each FR-242 Behavior line and AC-1 checked against the tests, mutation of the copy bounds, and probes with the evaluator as oracle. Plan completion: not assessed.

| Requirement | PR claim | Measured |
| --- | --- | --- |
| FR-242 Behavior: TT-3/TT-4 by MightyL | general nested non-punctual intervals built | **Met.** Every non-punctual shape translates, and 176 probe forms agree with the evaluator (SR-1667001). |
| FR-242 Behavior: agrees with the evaluator | agreement, forms and on-each tests | **Partial.** The tests agree, but they do not pin the `release` copy bound or the both-open bound: those two mutants survive (FND-001). |
| FR-242-AC-1 | unchanged | Met. `the_mitl_vectors_agree_with_the_evaluator` passes. |
| FR-242 Behavior: BoundedWindow admits punctual | refused when started at many positions | This is a spec gap already filed on STD-493 (086c74fa), and the refusal is mathematically forced (Alur and Dill, 1994). No new finding. |
| FR-234 punctuality | TT-3/TT-4 punctual is disposed before EN-6 | Consistent. |
| `ClockLimit` (> 64 clocks) | refusal added | **Untested** (FND-002). |

The on-each oracle copies changed to `12 / period + 2`. Past operators are still depth 1 with windows of at most 3 ms (12 quarters), so the windows have settled by then, which is sufficient.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The PR's tests do not pin two of the copy bounds that soundness depends on. Two mutants pass all 19 `timed_trace` tests: `release` capacity `m` instead of `m + 1`, and the both-open `spread` computed as `max(1, ceil)` instead of `floor + 1`. Both make the automaton reject some violating words, so EN-6 would wrongly answer Holds. My probe catches each on the first lassos. The `release` case is `eventually (holds(p) and eventually[1 ms, 2 ms] holds(q))`, whose negation forces `always[1,2]` starts at spaced `p` positions; the word is stamps `[0,3,6,7]` quarters, levels `[0,0,1,2]`, loop to 1 after 8. The both-open case is `eventually always(1 ms, 2 ms) holds(q)` on `[0,4,4,6,8,8]`, loop to 2 after 6. The forms test only uses releases started at every position or by choice, and its lasso gaps never exceed `d`, so a second block is never forced. Add `eventually (p and eventually_I q)` and `eventually (p and always_I q)` for `I` in `[1,2]` and `(1,2)` (and `[2,3]`), and the both-open `eventually always(1 ms, 2 ms)`, to `the_repeated_interval_forms_agree_with_the_evaluator`. | qsl-eval/tests/it/timed_trace.rs:1100-1190; qsl-eval/src/automaton/timed_claim.rs:564-581,597-625 |
| FND-002 | low | The `ClockLimit` refusal has no test. A one-line case covers it: for example `eventually always[32 ms, 33 ms] holds(q)` (65 copies), asserting `ClockLimit` from `TimedClaim::new`, and EN-6's and the checker's mapping. | qsl-eval/src/automaton/error.rs:70-80; qsl-eval/src/automaton/timed_claim.rs:898-903 |

## Verdict

Not mergeable yet, for one reason only: FND-001, a small test addition inside this PR. There are no soundness bugs. FND-002 and SR-1667001's low findings can ride along.
