---
id: SR-13583
title: 'Spec review of PR #1109: protocol wait causes, fences, ordinals and footprints, base'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@389b22c99; git diff origin/main...389b22c99 (PR #1109, branch qsl-559-protocol-spec, base main 5474b9125): spec/decisions/ADR-027-protocol-transition-system-for-parallel.md, spec/functional/FR-205, FR-206, FR-211, FR-216, FR-218, spec/test-cases/TC-650, TC-651, TC-656, TC-661, TC-1056; read against main code qsl-eval/src/simulation/protocol/{steps,channel,identity}.rs, ADR-025, and QSpec main 44afcf4 FR-170, FR-425, FR-426, FR-429, FR-435, FR-436; tickets QSL-559, QSL-562'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-206
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-211
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-216
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-218
  type: reviews
- target: ix://agent-ix/quire-spec-language/ADR-027
  type: reviews
---

## Summary

Tickets: QSL-559, QSL-562. PR: quire-spec-language#1109, head 389b22c99, scoped to `git diff origin/main...HEAD` (11 files, +95/-40), spec only. Methods: spec-review/base (SR-13583, this file: checklist, AC/TC coverage and testability), spec-review/ears (SR-13584), spec-review/integrity (SR-13585). code-review and gap-analysis were not run because no code or test file changed. Gates on the head: `make check-spec-validation` exits 0 with 17 grammar warnings, none in a changed file; `tools/check-index-completeness.sh` exits 0. quoin 0.28.3, quire 0.36.2 (engine 0.50.2). There is no changelog, status or ticket-owner wording in the diff, and no hash, digest or pin.

What was checked. (1) The FT-1 fork footprint was checked against the fork code (steps.rs:445-458). The fork takes the smallest ordinal that no live sibling holds. Only two things can change that choice. One is a lower ordinal becoming free, which happens at the step that ends the last running branch thread or descendant; that step writes the `ctl` and `inst(i_o)` that the fork now reads. The other is the taken ordinal becoming occupied, which only a fork by the same thread can do, and that fork shares `ctl(t)`. So the stated pair is dependent, and its two orders reach different paths (ordinal 0 or 1). FR-216-AC-2 states this correctly. The compensation-thread analogue is not covered (FND-002). (4) FR-206-AC-5's `exhausted` case is now observable and can fail: a mutation that skips `exhausted`, enters it early, or takes `X` a third time fails the AC. Two hedges remain (FND-006). (5) Every new ADR rule has an FR statement, an AC and a TC step, except ST-7's nested cancellation (FND-003) and the compensation-thread ordinal (FND-007).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| ADR-027 PS-2 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 PS-3 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 PS-7 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 ST-3 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 ST-7 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 SE-5 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 PD-4 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 FT-1 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 FT-5 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| FR-205-AC-3 | spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md | examined |
| FR-206-AC-4 | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md | examined |
| FR-206-AC-5 | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md | examined |
| FR-206-AC-7 | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md | examined |
| FR-206-AC-8 | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md | examined |
| FR-211-AC-5 | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md | examined |
| FR-216-AC-2 | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md | examined |
| FR-216-AC-3 | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md | examined |
| FR-216-AC-5 | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md | examined |
| FR-218-AC-27 | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| TC-650, TC-651, TC-656, TC-661, TC-1056 row 51 | spec/test-cases/ | examined |
| QSpec FR-170, FR-426, FR-429 | agent-ix/quire-specification spec/functional/protocol/ | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-211-AC-5's fixtures name no delivery policy. QSpec FR-170 defaults delivery to `unknown`, which admits `lose` and, where capacity allows, `duplicate` (ST-5). So at the claimed last states a `lose` step is still enabled and none of them is terminal. In the FullQueue case, losing `P` lets `send Q` go and the protocol finishes. In the HeadBlocked case, losing `P` exposes `Q`, and losing `Q` instead ends in `Displaced`, not `HeadBlocked`. In the Displaced case the deadlock state has an empty queue, not 'only Q's entry'. As written, the AC cannot pass. Fix: give every channel in the AC and in TC-656 step 5 `delivery exactly-once-premise`, or another policy that admits no loss. | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md:129 |
| FND-002 | high | PS-2 now gives compensation threads the same smallest-free-ordinal rule as forks, but FT-1 makes no matching read. The step that activates a registration (CP-2) reads and writes `reg(c, ·)` and writes the new thread's `ctl`. `cend(c, r)` (ST-12) writes only its own thread's `ctl` plus model locations. Take two registrations of template `c`: the `cend` of thread `(i, c, 0)` and a trigger step that activates the second registration are reported independent. Yet the new thread takes ordinal 1 if the trigger step goes first and ordinal 0 if `cend` goes first, so the two orders reach different states. This is the same non-commuting pair that the fork fix closes. Fix: the activating step reads `ctl` of each compensation thread of `c` in the instance for ordinals 0 up to the one it takes, as the fork does. Add `cend`'s status write, `reg(c, r)`, to its `W`. Add an FR-216 AC for the pair. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:294 |
| FND-003 | medium | ST-7 now cancels every thread at or below a cancelled branch, and every parallel instance those threads forked. No FR-206 SHALL states this, and no AC checks it. Only its footprint is tested (FR-216-AC-3). FR-206-AC-3 has flat branches only, so an implementation that cancels only direct branch threads, leaving inner branches stepping, passes every AC. Fix: add the rule to FR-206's Behavior, and an AC with a branch whose inner `parallel` has forked under `outstanding cancel`: the inner threads are `cancelled` and take no step, and the inner instance is settled and removed. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:168 |
| FND-004 | medium | FR-206-AC-7 uses a capacity-1 channel, where the head entry and the tail entry are the same entry. So 'loss head and loss tail give displaced the head entry and the tail entry respectively' cannot tell the two selectors apart: swapping `Head` and `Tail` passes. Fix: use capacity 2 holding `P1` then `P2`; `loss head` displaces `P1` and leaves `[P2, new]`, and `loss tail` displaces `P2` and leaves `[P1, new]`. | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md:92 |
| FND-005 | medium | The `Displaced` cause is tested only through `overflow loss head`. Nothing checks a `lose` step. Nothing checks `overflow loss new` (outcome `lost`), which PD-4's parenthesis counts but its main clause, 'removal of one from this queue', does not cover. Nothing checks the 'and not a receive' half, where a loss is followed by a receive of a later entry from the same send node and the cause is `EmptyQueue`. Fix: add these three cases to FR-211-AC-5 and TC-656 step 5, after SR-13585 FND-004 settles the wording. | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md:111 |
| FND-006 | low | FR-206-AC-5 keeps two hedges. 'No reachable state within the explored depth' weakens a check that can be exhaustive: an unbounded `repeat` keeps no counter (PS-6), so this protocol has finitely many states. 'Leaves the repeat for its next sibling' names no sibling. Fix: say 'no reachable state', and give the no-`exhausted` variant a following control, for example `attempt W`, at which the thread rests after the second `X`. | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md:90 |
| FND-007 | low | PS-2's sibling and smallest-free-ordinal rule for compensation threads has no AC; FR-205-AC-3 covers only `parallel` instances. Fix: add an FR-205 case in which two registrations of one template activate, the first thread ends, and the next activation reuses ordinal 0. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:134 |
| FND-008 | medium | FR-218-AC-27 is verified by TC-1056, an S3 check test, but a queue key, a `duplicate` identity and a wait cause are built only by the explorer (FR-205, FR-211). TC-1056 row 51 also drops the wait cause that the AC asserts. Fix: keep the S3 half (the checked channel name) in FR-218-AC-27 and TC-1056 row 51. Move the queue-key, `duplicate`-identity and wait-cause halves to TC-650 and TC-656 under FR-205 and FR-211 ACs. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:418 |

## Verdict

Not mergeable. FR-211-AC-5 cannot pass under the default delivery policy (FND-001), and the compensation-thread ordinal leaves a non-commuting pair reported independent, the same kind of defect this PR fixes for forks (FND-002). The fork footprint is sound for the stated pair. The send record, fence and visibility ACs are traced. FR-206-AC-5 is now non-vacuous. The index is complete and validation is clean on every changed file.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | FR-206-AC-9's state ('`b` has forked `Inner` and `y` is `running` and `a` has completed') does not say where `x` and `b` are. If `x` has completed, `Inner`'s `join any` may have settled and `b` may rest at `After`, and `x` may already have left the state. In that state 'the `join(Outer)` step makes `b`, `x` and `y` `cancelled`' cannot hold for `x`. Fix: name the state as `x` and `y` both `running` and `b` resting at `Inner`'s join, or state the outcome for each thread that is still `running`. | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md:101 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | The fix commits added reviews/round0-base.md, reviews/round0-ears.md and reviews/round0-integrity.md. These are the reviewer's scratch snapshots of the round-0 SR files, copied in with the folder. They repeat the ids SR-13583, SR-13584 and SR-13585, so each id now names two documents, which `quire validate` does not catch. Fix: delete the three round0-*.md files, and copy only the qsl-559-pr1109-spec-review-*.md files. | reviews/round0-base.md:2 |

## Dispositions

Round 1, head ccae02a7d: one fix commit over 389b22c99, base main still 5474b9125. The three SR files the commit adds to reviews/ are byte-equal to the reviewer's round-0 files. Gates on the head: make check-spec-validation exits 0 with the same 17 warnings, none in a changed file; tools/check-index-completeness.sh exits 0. FR-211-AC-5's cases were checked one by one under the new PD-4 order. FullQueue: queue [P] at capacity 1. HeadBlocked: queue [P, Q] with head P. Constraint: head P fails the constraint, fifo and unordered. Displaced by loss head: [Q]. Displaced by loss new: Q lost, queue [P]. Displaced by `lose` under at-most-once. EmptyQueue: the second P displaces the first, R1 receives it, and R2 waits after a `receive`. All give the stated causes, and `exactly-once-premise` leaves no `lose` or `duplicate` enabled.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | fixed | - |

Round 2, head 771649f1d (one fix commit over ccae02a7d; base main still 5474b9125). FR-206-AC-9 now fixes the state: `a` completed, `b` resting at `Inner`'s join, `x` and `y` both `running`. In that state `join(Outer)` cancels all three, and `Inner`, now settled with no running branch, is removed (steps.rs:593-615, normalize at steps.rs:823-829). Gates: make check-spec-validation exits 0 with the same 17 warnings, none in a changed file; tools/check-index-completeness.sh exits 0.

| FND | outcome | reason |
| --- | --- | --- |
| FND-009 | fixed | - |
