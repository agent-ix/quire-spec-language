---
id: SR-1596001
title: "code-review and rust-review of PR #1596 (QSL-580b, FR-231 timed subject)"
type: SpecReview
analysis: code-review
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1596 own commit 48f73e4ff..ddeff5dd6: qsl-eval/src/simulation/model/{timed.rs,model.rs}, qsl-semantics/src/check/timed_model.rs (clock_fields), qsl-eval/tests/it/{timed_subject.rs,main.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-231
    type: reviews
---
# SR-1596001: code-review and rust-review of PR #1596 (QSL-580b, FR-231 timed subject)

## Summary

Ticket: QSL-580. The review is scoped to the PR's own commit ddeff5dd6 over 48f73e4ff. #1596's base branch was rewritten after #1596 branched: it was rebased and main was merged into it. So `git diff origin/qsl-580a-timed-s3...origin/qsl-580b-timed-subject` and the GitHub diff now show 580a's 11 files again (FND-003). I traced the delay admissibility (an upper-bound invariant is checked at the delay's end, which is enough because each invariant is downward closed), the step relation (data precondition through FR-120, then a convex part, resets and target invariants), the cross product of the parts of several preconditions, urgency, quiescence, tick stamps and the idle-tail lasso. These match FR-231. Focused tests at 7ac32b8d7 (the stack head) include `timed_subject`, all passing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Clock places are fixed per universe object, and a reset is the only write. An object that a step creates, or recreates after a delete, keeps the stale value its place last held, not 0 or the constant of its write. A predicate about an absent object is read as not applying. QSpec FR-181 "Timed subjects" gives a clock that a step makes present the constant of its write. Traced, not reproduced: the fixtures' universe is static. | qsl-eval/src/simulation/model/timed.rs:193-202,421-432 |
| FND-002 | low | `timed_trace(idle_tail = true)` returns a closed finite trace when the last state is not quiescent. A behaviour that cannot diverge is then read as a bounded execution with no signal. FR-231 admits a behaviour only when its time stamps diverge. Refuse, or flag the trace as not admitted. | qsl-eval/src/simulation/model/timed.rs:885-905 |
| FND-003 | low | Stack hygiene: #1596's base `qsl-580a-timed-s3` was rewritten (rebased commits 8897aa172..919c40063 and a main merge a02c4c9a1), but #1596 still sits on the old commits 05a3c98cb..48f73e4ff. Its GitHub diff therefore re-shows 580a. Restack it onto the current 580a head before merge, or squash-merge order will conflict. | git log origin/qsl-580b-timed-subject; git log origin/qsl-580a-timed-s3 |

## Verdict

**Mergeable after restack (FND-003).** FND-001 is real but has no effect on the static universes the stack's engines use. Fix it, or defer it with a ticket. Correct: exact rational arithmetic throughout, canonical transition order by transition key, `ParameterizedGuard` and `Unbounded` refusals instead of silent misreads, and urgency and quiescence as FR-231 defines them. Rust lane: no panics in production paths except the documented `last()` invariant on a non-empty `positions`. The `expect` there is safe: `positions` is non-empty by construction.

## Coverage

Methods: code-review with the rust-review lane. Gap analysis is SR-1596002.
