---
id: SR-13585
title: 'Spec review of PR #1109: protocol wait causes, fences, ordinals and footprints, integrity'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@389b22c99; git diff origin/main...389b22c99 (PR #1109, branch qsl-559-protocol-spec): ADR-027 PS-2, PS-3, PS-7, ST-3, ST-7, SE-5, PD-4, FT-1, FT-5 and FR-205, FR-206, FR-211, FR-216, FR-218; read against ADR-027 PS-4, PS-5, SE-1, FT-3, ADR-025 TSO-6, RA-5, MR-1, main code qsl-eval/src/simulation/protocol/{steps,channel}.rs, and QSpec main 44afcf4 FR-050, FR-170, FR-425, FR-426, FR-429, FR-435, FR-436; tickets QSL-559, QSL-562'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-027
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-205
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-211
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-216
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-218
  type: reviews
---

## Summary

Tickets: QSL-559, QSL-562. PR quire-spec-language#1109, head 389b22c99. This file checks that the changed rules agree with each other, with the rest of ADR-027 and ADR-025, with the code on main, and with QSpec.

(2) Wait causes. The PD-4 receive chain (HeadBlocked, then Displaced, then EmptyQueue) is mutually exclusive as written, and FullQueue applies only to a `send`. The chain clashes with the list-order 'first that applies' precedence in two places: the constraint-failing head and Gate (FND-003). `Displaced` is a deterministic function of the reported prefix, because replay recomputes it from that prefix (PX-2). That is sound, since the blocked list is not in the state key, but the rule does not say that the cause belongs to the prefix rather than the state, and its wording leaves `loss new` and multi-key queues open (FND-004).

(3) Fences. SE-5 matches ADR-025 TSO-6 and QSpec FR-436 (`seq_cst` fence enabled only when the buffer is empty, the other orderings always enabled), RA-5 (`ra` fences never wait), and `sc` (always enabled). It also matches QSpec FR-426's fence row ('its memory model's fence rule'). The defect is that SE-5 calls the fence rule SE-1 (e)'s gate, which does not cover fences (FND-002).

(6) QSpec. QSL contradicts QSpec FR-429 until the companion edit lands (FND-008). It does not contradict FR-426: the send record and ST-7's nested cancellation extend FR-426's send and join rows, which say nothing on those points. The companion QSpec edit must carry the following:
- FR-429 Blocked threads: add `{"type":"head-blocked","channel":"<qualified-name>"}`, `{"type":"full-queue","channel":"<qualified-name>"}` and `{"type":"displaced","channel":"<qualified-name>"}`.
- FR-429: narrow `empty-queue` to 'no entry from the named send node is queued and none was lost'.
- FR-429: state the one-cause precedence and that `displaced` is computed from the counterexample prefix.
- FR-429: add an AC per new cause.
- FR-426 send row and FR-170: the send record `{channel, payload, outcome, displaced}`, with outcome `accepted`, `rejected` or `lost`.
- FR-426 join row: under `outstanding cancel`, every thread at or below a cancelled branch is cancelled and its inner parallel instances are settled.
- FR-426 Waiting: add a `block` send at capacity and a fence whose rule is false.
- FR-425: the descendant clause of the ordinal rule (FND-001) and the qualified channel name (FND-006).

A note on the in-flight code, outside this diff. origin/qsl-559b-impl binds `{payload, outcome}` with outcomes `sent`, `rejected`, `lost` and `displaced`. That is not ST-3's `{channel, payload, outcome, displaced}` with `accepted`, so the coder must follow the spec shape.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| ADR-027 PS-2 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 PS-4 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | context_only |
| ADR-027 PS-7 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 ST-3 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 ST-7 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 SE-1 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | context_only |
| ADR-027 SE-5 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 PD-4 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 FT-1 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 FT-3 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | context_only |
| ADR-027 FT-5 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| FR-205-AC-3 | spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md | examined |
| FR-211 Behavior | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md | examined |
| FR-216 Behavior | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md | examined |
| FR-218 Channels, FR-218-AC-27 | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| ADR-025 TSO-6, RA-5, MR-1 | spec/decisions/ADR-025-weak-memory-models-for-parallel.md | context_only |
| QSpec FR-050, FR-170, FR-425, FR-426, FR-429, FR-436 | agent-ix/quire-specification spec/functional/ | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | PS-2 defines a fork's sibling as a live parallel instance only. But PS-4 removes an instance once its join has settled and its branch threads have left `running`, and a thread one of those branches forked can still be running at that point (an inner `outstanding continue` join). PS-2 then frees the ordinal, and the new fork's branch path becomes a prefix of a still-running thread's path. The code avoids this: it treats an ordinal as held while any thread below its branch paths remains (qsl-eval/src/simulation/protocol/steps.rs:440-458). FT-1's read of 'the `ctl` of every thread at or below each of its branch threads' only makes sense under that rule. So PS-2 contradicts both the code and FT-1, and QSpec FR-425 has the same gap. Fix: in PS-2, an ordinal is held while its parallel instance is live or any live thread's path extends one of its branch paths. Add that case to FR-205-AC-3, and to QSpec FR-425 in the companion edit. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:134 |
| FND-002 | medium | SE-5 calls a fence's enabling condition 'its memory model's fence rule (the gate of SE-1 e)'. SE-1 (e) defines the gate as an extra condition on `fork`, `join`, `send`, `receive` and `attempt` steps, and fences are not in that list. QSpec FR-426 also keeps the 'fence rule' (FR-435) separate from the gate. So the seam has no member that supplies the fence rule. Fix: add `fence` to SE-1 (e)'s list, or make the fence rule a ninth seam member. Name it in FR-215's `MemoryModel` seam. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:230 |
| FND-003 | medium | PD-4 says 'the first that applies', and its list puts `Constraint` before `HeadBlocked` and `Gate` last. Yet HeadBlocked includes a head from the named send node 'whose payload fails `n`'s constraint'. The code on main returns `Constraint` in that case (channel.rs:359-360), and checks `Gate` before every other cause (steps.rs:951-969). Two readings therefore give two different causes. Fix: state the order for each rest-point kind, for example receive: Gate, then HeadBlocked, Displaced, EmptyQueue. Decide whether a constraint-failing head is `HeadBlocked` or `Constraint`, and add an AC for it. For an `unordered` channel, which has no head, say which cause a constraint-failing entry gets. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:251 |
| FND-004 | medium | PD-4's `Displaced` clause says 'the last removal of one from this queue ... was a loss (... the send record's `lost` or `displaced`)'. Under `loss new` the arriving payload is never queued, so nothing is removed and the main clause gives `EmptyQueue`, against the parenthesis. 'This queue' is also unclear when the channel holds one queue per FIFO key. And the cause depends on the prefix, not on the state: two counterexamples ending in one state can carry different causes, which the rule does not say. Fix: 'the last event that removed or dropped an entry from that send node on this channel, on the reported prefix, was a `lose`, a displacement or a `lost` send, not a `receive`'. State that the cause belongs to the reported prefix. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:251 |
| FND-005 | medium | FT-1 says a branch-ending step 'writes only that branch's disposition in `inst(i)`'. But FT-3's `ProtocolLocation` has `inst(i)` only, with no per-branch location, so the narrower write cannot be expressed in a `Footprint`. As typed, two branch ends of one instance remain dependent. Also, when the last running branch ends after the join has settled, that step removes the whole instance (PS-4), which is more than one disposition. Fix: add `disp(i, b)` to `ProtocolLocation`, with the rule that a read or write of `inst(i)` meets every `disp(i, ·)`. State that the step that removes the instance writes `inst(i)`. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:294 |
| FND-006 | medium | PS-7 qualifies a channel by 'the name of the package that declares it'. But channels are declared inside a protocol: TC-1056's `Pipeline` declares `channel Pay`, and FR-218 refuses duplicates per scope. So two protocols in one package that each declare `Pay` get one qualified name. QSpec FR-050 instead keys a static channel by the qualified protocol-definition identity and its channel handle. FR-218-AC-27 also treats `M`, the fixture's model alias, as the checking package. Fix: qualify a channel as package, protocol and channel, to match FR-050, or state that channels are package-scoped. Correct AC-27's package names. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:139 |
| FND-007 | medium | When a fork reuses an ordinal, it clears the binders and counters of the reused paths (steps.rs:470-476). FT-1's `W` for the fork omits those `bind(·, t')` entries. Example: under PS-5, a branch binder stays readable after an all-branch join. A later `outstanding continue` branch of the same parent can still read `a` from the earlier `parallel` when the next iteration re-forks that `parallel` at ordinal 0. The fork and that read step would then be reported independent although they do not commute. Fix: add the reused paths' `bind` and counter locations to the fork's `W`, or extend PS-2 so an ordinal stays held while a live thread can still read a binder under it. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:294 |
| FND-008 | medium | QSpec FR-429 lists its causes as a closed set ('`cause` one of'), without `head-blocked`, `full-queue` or `displaced`, and defines `empty-queue` as 'a receive with no deliverable entry'. QSL now narrows `EmptyQueue` and adds three causes. The PR body says FR-429 is not changed here, so the two specs disagree until the companion edit lands. Fix: land the QSpec edit listed in Summary (6) before, or together with, this PR. | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md:111 |

## Verdict

Not mergeable. PS-2's ordinal rule frees an ordinal that a running descendant still sits under, which the code and FT-1 both avoid (FND-001). The fence rule has no seam member that supplies it (FND-002), and the cause precedence has two readings (FND-003). The send record, fence semantics under `sc`, `tso` and `ra`, FT-5 visibility, and the fork footprint for the stated pair are all consistent.
