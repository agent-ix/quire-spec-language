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

## Dispositions, round 1 (head 856f4f859)


Round 1. Fix 090a00698; head 856f4f859, retargeted to main.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | I re-ran both mutants against `the_repeated_interval_forms_agree_with_the_evaluator`. Both now fail, and the head passes (182 s). |
| FND-002 | fixed | `ClockLimit` is tested in qsl-eval (`[32,33]`: 130 clocks; `[100,101]`: at least 200), and in EN-6 and the checker. |

**FND-001 mutant results.**
- `release` capacity `m`: fails on `eventually (p and eventually[1 ms, 2 ms] q)`, stamps `[0,3,5,7,10]` quarters, loop to 3 after 7.
- Both-open bound as `max(1, ceil)`: fails on `eventually always(1 ms, 2 ms) q`, stamps `[0,2,2]`, loop to 1 after 2.

**Leaving out the ten-clock `always(1,2)` and `always[2,3]` forced starts is acceptable.** The forms that are present already cover each bound:
- `release` `m + 1`: `p and eventually[1,2] q`, plus the both-open `(1,2)` and `[2,3]`;
- `until` `2m + 1`: `eventually always[1,2] q`, and forced by `p and always[1,2] q`;
- both-open `floor + 1`: `eventually always(1,2) q` and `p and eventually(1,2) q`;
- past `m + 1`.

Every capacity-minus-one mutant fails one of them. The two forms left out would only add another `until` instance of bounds that are already pinned.

**Verdict: mergeable.**

Gates at 856f4f859 (locked-build, jobs 4, `--all-features`, QSPEC_DIR at origin/main 60383a35):
- fmt: exit 0.
- qsl-eval `timed_trace`: 19/19.
- qsl-analyze `zone_`: 91/91.
- qsl-replay `certificate`: 27/27.
- clippy on qsl-eval, qsl-analyze and qsl-replay with `-D warnings`: exit 0.

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: '("eventually (holds(c.level = 1) and eventually[1 ms, 2 ms] holds(c.level = 2))", 1_000), ... ("eventually always(1 ms, 2 ms) holds(c.level = 2)", 80),'
- fnd: FND-002
  outcome: fixed
  after_excerpt: 'Some(qsl_eval::automaton::AutomatonError::ClockLimit { clocks, limit: 64 })'
```

+++
