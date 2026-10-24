---
id: SR-14090
title: "integrity review of STD-240 ra pin set and observation (PR #1195, QSpec PR #334)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@f53be97e54425d2f247a90925a66fb4b5e3bc0a5; spec/decisions/ADR-025-weak-memory-models-for-parallel.md, spec/functional/FR-222-explore-the-release-acquire-memory-model.md, spec/functional/FR-229-carry-the-memory-component-through-the-protocol-system.md, spec/test-cases/TC-667-release-acquire-memory-model.md, spec/test-cases/TC-674-memory-component-through-the-seam.md; agent-ix/quire-specification@40cc2f8b08f639c790fba9791b155a3810d59b40; spec/functional/memory/FR-435-classify-memory-accesses-and-orderings.md, spec/functional/memory/FR-437-explore-release-acquire-memory.md, spec/test-cases/TC-389-memory-access-classification.md, spec/test-cases/TC-393-memory-budgets-and-verdicts.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-222
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-229
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-435
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-437
    type: reviews
---
# SR-14090: integrity review of STD-240 (PR #1195, QSpec PR #334)

## Summary

Ticket: STD-240. Each diff is against main; neither PR is stacked. The
review covers ADR-025 RA-7 and the section 10 `ra` paragraph, the new FR-222
and FR-229 bullets, FR-222-AC-3 and AC-5, FR-229-AC-6, TC-667 and TC-674, the
QSpec FR-437 garbage-collection paragraph, FR-437-AC-4, the FR-435 `ra`
observation paragraph, FR-435-AC-7, TC-389 AC-12 and TC-393 MB-07.

Code read: `qsl-eval/src/simulation/protocol/ra.rs` (`running`, `kept_from`,
`collect`, `join`, `view_enc`, `probe`, `reads`, `store`, `exceeds`,
`merge`) and `qsl-replay/src/model_check/replay.rs`.

Pin-rule soundness, traced by hand. A load or RMW reads at or after its own
thread's `cur`, and a store inserts after it. The earliest pinned message is
the minimum over running threads' `cur`, so no running thread can be offered
a message that collection removed. A fork copies a running parent's views. A
`join` or a `receive` only moves a view forward, and a pointer below every
running `cur` joins as nothing (`join` skips an id that is no longer live).
A thread waiting at a `join` gets its branches' joined views, which sit at
or above the branches' pinned positions. The rule cannot lose a message a
running thread may still read. The engine pins in the same way (`running`
excludes only threads that are a prefix of another live thread).

The ADR-025 section 10 sentence holds: `right`'s `cur` points at `x0` until
`right` reads `x`, and `left`'s at `y0`. The memory component is still a
function of the two local states, so 14/4 is unchanged. FR-222-AC-5 matches
the engine: `store` collects before `exceeds`, so a single looping writer
keeps one message and `max_messages` 1 is not reached (MB-2, "after garbage
collection").

Gates: `make check-spec-validation` exit 0 with no diagnostic on a touched
file, and `tools/check-index-completeness.sh` passes. No `docs/` file and no
test-parsed vector table is touched. QSpec `quire validate --scope .
spec/**/*.md` reports 134 failing documents, the same as main, none of them
a touched file. `cargo test --test checked_package_v2 --test
backend_provider_v1`: 14 and 117 passed. Both branches merge cleanly onto
current main.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | QSpec FR-435-AC-7 and TC-389 AC-12 are new ids that open QSpec PR #329 (STD-233) also adds with different content; `git merge-tree` conflicts on both files. | spec/functional/memory/FR-435-classify-memory-accesses-and-orderings.md:138, spec/test-cases/TC-389-memory-access-classification.md:41 |
| FND-002 | medium | QSpec FR-437 says a pointer at a removed message "is encoded as pointing at the earliest live message", but `view_enc` drops such an entry (`filter_map` on `position`). Equal futures can therefore get different keys, and the PR body's "None for the engine rules" leaves the code lane no task. QSL RA-7 says only "reads as", a semantic rule, so the two halves state different things. | spec/functional/memory/FR-437-explore-release-acquire-memory.md:94, qsl-eval/src/simulation/protocol/ra.rs:862 |
| FND-003 | medium | The negative clause of TC-389 AC-12 cannot be reached. The second branch's single attempt stores before it loads, so its view never points at the value-1 message before its store, and "not enabled once its view points at the message with value 1" cannot be exercised. QSL FR-229-AC-6 makes the branch repeat, so the two halves disagree on the scenario. | spec/test-cases/TC-389-memory-access-classification.md:41 |
| FND-004 | low | FR-229-AC-6 leaves the load's precondition ("the load's precondition") and the value the guarded store writes unstated, while QSpec AC-12 fixes the load at `x.v = 1`. | spec/functional/FR-229-carry-the-memory-component-through-the-protocol-system.md:128 |
| FND-005 | low | FR-222-AC-5 and TC-393 MB-07 do not state the value the looping store writes. A counter store (`x := x.v + 1`) makes the state space unbounded, and `proved` is then unreachable. | spec/functional/FR-222-explore-the-release-acquire-memory-model.md:77 |
| FND-006 | low | "Running thread" is not defined. The engine pins the `cur` of every thread that is not waiting at a `join`, a branch that finished but has not been joined included. The spec leaves that branch open. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:220 |
| FND-007 | low | RA-7's "with no `seq_cst` fence, a location's initial message is removed once…" is left over from the old rule. Under the new rule `S` pins nothing, so the qualifier wrongly implies that a fence keeps the message. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:220 |
| FND-008 | low | TC-393 MB-07 now lists three cases, and its expected result's "in the first variant" could mean `SB` itself rather than the both-threads-load-`x` variant. | spec/test-cases/TC-393-memory-budgets-and-verdicts.md:38 |

## Verdict

Not mergeable until FND-001 is resolved. Renumber this PR's FR-435-AC-7 and
TC-389 AC-12, or #329's, before either merges. FND-002 and FND-003 should be
fixed in this PR. The pin rule itself is sound and agrees with the engine
(see Summary). The observation rule lives in FR-435, not FR-437, and the
QSL and QSpec observation statements agree. The PR bodies contain no
review-finding paraphrase. QSL #1195's RA-7 row sits next to the RA-8 row
that open #1192, #1193 and #1194 edit, so `git merge-tree` reports a textual
conflict: the second to merge needs a rebase.

## Dispositions

Round 1. Against QSL #1195 head 0971a22 (fix commit 9dab122, then a merge of
origin/main) and QSpec #334 head 52251d6 (fix commit d8c8c83, then a merge of
origin/main). Both heads contain origin/main. QSL gates: `make
check-spec-validation` exit 0 with no diagnostic on a touched file;
`tools/check-index-completeness.sh` passes; no `docs/` file or test-parsed
vector table is touched. QSpec gates: `quire validate --scope . spec/**/*.md`
reports 134 failing documents, none of them a touched file; `cargo test --test
checked_package_v2 --test backend_provider_v1`: 14 and 117 passed. The new
QSpec ids FR-435-AC-8 and TC-389 AC-13 occur in no other open QSpec PR: #329
(head f655246) adds FR-435-AC-7 and TC-389 AC-12, and #328 (head 412a71e)
touches neither file. `git merge-tree` of #334 with #329 still conflicts on
FR-435 and TC-389, but each hunk is only the two PRs' new rows at the same
place, so the second to merge keeps both rows. #1195 still conflicts
textually with #1192, #1193 and #1194 at the RA-7/RA-8 rows, as before. No
other spec text outside reviews/ states the old pin set. The TC-389 AC-13 and
FR-229-AC-6 scenario was re-traced: the repeating branch stores `x := 0` at
its current view, then its load with `x.v = 1` reads the value-1 message after
its view, after which its next store is not enabled, so the negative clause
is reachable.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | QSpec FR-435: "\| FR-435-AC-8 \| Under `ra`, a store whose precondition requires `x.v = 0` is enabled only while its thread's current view points at a message of `x` with value 0, …"; TC-389: "\| AC-13 \| under `ra`, a branch storing `x := 1` once and a second branch repeating an attempt …(FR-435-AC-8) \|". No id overlap with #329 or #328. |
| FND-002 | fixed | QSpec FR-437 and QSL RA-7 now state the same rule: "a view that points at a removed message reads as, and is encoded in the state key as, pointing at the earliest live message of its location"; both PR bodies' Remaining work list the `view_enc` change for the code lane. |
| FND-003 | fixed | TC-389: "\| AC-13 \| under `ra`, a branch storing `x := 1` once and a second branch repeating an attempt that stores `x := 0` with precondition `x.v = 0` and then loads `x` with precondition `x.v = 1` \| after the first branch's store, the second branch's store is enabled while its current view points at the message with value 0 and is not enabled once its view points at the message with value 1; …" |
| FND-004 | fixed | FR-229-AC-6: "one branch repeating a `relaxed` store of the constant `x := 0` with the precondition `x.v = 0` and a `relaxed` load of `x` with the precondition `x.v = 1`, and a second branch storing `x := 1` once: … its load is enabled for each message at or after that view whose value is 1." |
| FND-005 | fixed | FR-222-AC-5: "a `relaxed` store of the constant `x := 1` and then a `relaxed` load of `x`"; TC-393 MB-07 (c): "a `relaxed` store of the constant `x := 1` and then a `relaxed` load of `x`, with `max_messages` 1". |
| FND-006 | fixed | RA-7 and FR-437: "A thread is **running** unless it is waiting at a `join`; a branch that has finished and is not yet joined is running." This matches the engine's `running`. |
| FND-007 | fixed | RA-7: "Removal is part of the canonical state and makes loops finite: a location's initial message is removed once every running thread's current view has moved past it." |
| FND-008 | fixed | MB-07: "(a) TC-390's `SB` under `ra`; (b) `SB` in which both threads load `x` after `Sx`; (c) … \| in (b), `x0` is removed in the step after which no running thread's current view points at it, …" |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | Both PR bodies' Remaining work say "`view_enc` … drops the entry whose message was removed; it encodes it as the depth of the location's earliest live message". The second clause reads as a statement of what the code does now and contradicts the first; it should say the code must encode it that way. | QSL #1195 body, QSpec #334 body (Remaining work) |

## Dispositions (pass 2)

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-009 | fixed | Both PR bodies' Remaining work now say the code must encode a pointer at a removed message as the depth of its location's earliest live message, and that the view encoding drops the entry today. The specs carry no such statement. Branches also merge the STD-239, STD-246 and STD-247 branches; the RA-7 and RA-8 rows keep both sides (RA-7 from STD-240, RA-8 naming PSC-7's omitted clauses). |
