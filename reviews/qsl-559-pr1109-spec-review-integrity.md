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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | FT-1's fork now writes, and clears, the `bind` locations of every path it reuses. Under the new PS-2 rule, a path can be reused while a live thread outside it can still reach a node that reads its binder. Example: a branch of a later `parallel Q` of the same parent, still running under `outstanding continue`, that reads `a` from the all-branch-joined `P`. PS-5 keeps a binder 'while some rest point of some live thread can still reach a node that reads it', so the clear contradicts PS-5, and the read changes meaning depending on the order. The code does the same (steps.rs:470-476). Fix: also hold the ordinal while a live thread can still reach a reader of a binder under it, or state in PS-5 that a re-fork ends the old binder and say what the reader then sees. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:137 |
| FND-010 | medium | For a `send`, PD-4 says '`FullQueue` when `block` finds the queue at capacity, else `Constraint`', so a full queue gives `FullQueue` even when no payload satisfies the constraint. FR-211 says `FullQueue` 'for a `send` disabled only because `block` finds the queue at capacity', which excludes that case. The code on main follows FR-211 (channel.rs:375-411: `FullQueue` only when some admitted payload exists, else `Constraint`). The full-queue, unsatisfiable-constraint send therefore has two causes. Fix: in PD-4, 'A `send`: `Constraint` when no payload satisfies its constraint, else `FullQueue`', to match FR-211 and the code, and add that case to FR-211-AC-5. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:251 |
| FND-011 | low | FT-1's fork `W` names 'iteration-counter locations', but FT-3's `ProtocolLocation` has no counter location. A counter is keyed by thread path (PS-8 `bounds`) and is covered by that path's `ctl`, which the fork already writes. Fix: drop 'and iteration-counter', or say counters are part of `ctl(t)`. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:294 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | high | PS-5's new resolution rule reads a node's binder from the nearest of the thread and its ancestors that bound it, else from a thread it forked. Under that rule a branch of a later `parallel Q` never sees `a`, which was bound by a branch of the earlier all-joined `P`. Yet S3 admits that read. Its scope rule follows QSpec FR-466 (qsl-semantics/src/check/protocol_slots.rs: a `parallel` joined by `all` adds every branch's binders to `Out`, and a sequence threads `Out(P)` into `Q`'s branches), and FR-218 says 'a branch binder stays readable after an all-branch join'. So a read S3 accepts meets 'its frame holds no value', and the spec gives that read no result. The code turns it into an internal fault: the slot is `None` (protocol.rs:894-910, `Instance::binding` at protocol.rs:313-338), and evaluation then hits `invariant("evaluation-local-value-unbound")` (qsl-eval/src/value/expression/evaluate.rs:1320). 'The frame holds no value for that binder' is therefore not enough, and FR-205-AC-7 asserts the defective case. Fix, option (a): resolve through the ancestor chain, so a thread reads the joined branch binder of the nearest ancestor that forked the binding thread; the re-fork case of FND-009 then needs the ordinal held while a live thread can still read a binder under it. Fix, option (b): narrow FR-218 Scopes and QSpec FR-466 so that a branch binder after an all-join is readable only in the forking thread's own continuation, not in threads it forks later, and refuse the read at S3 with `missing_declaration`/`missing-name`. Either way, rewrite FR-205-AC-7 to match. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:137 |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | high | PS-5's resolution reads 'the joined branch binder of the nearest ancestor (or itself) that forked the binding thread', in the singular. But the new PS-2 hold clause lets two binders of one node coexist under one forking thread. Take FR-205-AC-7 with `u` = `sequence { attempt U1; check Ck { a.ready } }`. While `u` of iteration 1 still rests at `U1`, the second fork of `P` takes ordinal 1 (ordinal 0 is held). It binds `a` under `(0, P, left, 1)`, joins, and forks `Q` again at ordinal 1, because `u`'s `Q` instance is still live. Now `t` forked both `(0, P, left, 0)` and `(0, P, left, 1)`. The rule picks neither for `u` of iteration 1 or `u` of iteration 2, though each should read its own iteration's `a`. The code refuses exactly this case with `fault("a-reader-sees-one-binding-of-a-node")` (protocol.rs:329-334). Fix: make the binder a reader sees a function of state. For example, a fork of `Q` records, for each branch, the ordinal of each all-joined instance whose binders it can read, so the instance that the forking thread most recently joined before that fork, and resolution reads that one. Put the recorded choice in the state key (PS-8), and add the two-iteration case to FR-205-AC-7. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:137 |
| FND-014 | medium | In FR-205-AC-7, `u`'s only control is `check Ck { a.ready }`, and `A1` binds `a` with `ready` true. A `check` adds no step (FR-206-AC-5), so `fork(Q)` settles `u` through the true check to its end, and `u` is `completed` in that same step. So no live thread can reach a reader of `a` at the second fork of `P`, and 'takes ordinal 1, not 0, and the key still holds `a`' can never happen: that half of the AC is vacuous, and TC-650 step 7 cannot observe it. Fix: give `u` a step before the reader, for example `sequence { attempt U1; check Ck { a.ready } }`, and state the state in which `U1` has not yet been taken. | spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md:116 |

## Dispositions

Round 1, head ccae02a7d. The checks were re-run against the code on main. PS-2's new holding rule (instance live, or any live thread's path extends a branch path) is the code's test (steps.rs:445-458), and FT-1's fork reads still cover it. A lower ordinal is freed only by a step that writes the `inst(i_o)` or a descendant `ctl` the fork reads. The taken ordinal is filled only by a fork of the same thread. The compensation analogue now reads `ctl` up to the taken ordinal, and `cend` writes `reg(c, r)`. PD-4's order matches the code: Gate first (steps.rs:951-969); attempt causes (attempt_cause); and receive Constraint, then HeadBlocked, then EmptyQueue (channel.rs:352-372). `Displaced` is spec-ahead-of-code, and an unordered channel is never HeadBlocked. The one mismatch is the send order (FND-010). FT-1's branch-end write is now the whole `inst(i)`, which is sound and coarser: sibling branch ends of one instance are dependent. SE-1 (e) and FR-215 list `fence`. QSpec main is still 44afcf4, and no companion quire-specification PR is open.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | deferred | The companion quire-specification PR carries FR-429's cause list and narrowed empty-queue, the FR-426 send record, nested cancellation and the waiting list, and FR-425's ordinal and channel-name rules (PR body). No such PR is open, and QSpec main 44afcf4 does not have it, so QSL stays ahead of QSpec until it lands. |

Round 2, head 771649f1d (one fix commit over ccae02a7d). PD-4's send order (Constraint, then FullQueue) now matches FR-211 and the code. `send_cause` returns `FullQueue` only when some payload the constraint admits exists, and `Constraint` otherwise (channel.rs:375-411), and FR-211-AC-5 has the full-queue, unsatisfiable-constraint case. The fork's write footprint now drops the counter wording and says counters are covered by `ctl`. The new PS-5 makes ADR-027 consistent with itself: a thread outside the forking thread's subtree never holds the binder, so the fork's clearing write (`bind`) is dependent only with the forking thread's own steps, and FT-1 is sound under PS-5 as now written. The rule contradicts S3's binder scope, though (FND-012), so the reader in FND-009's example still exists. QSpec main is still 44afcf4, and no companion PR is open.

| FND | outcome | reason |
| --- | --- | --- |
| FND-009 | still-open | The fix removes the PS-5 contradiction by declaring that a branch of a later `parallel` never holds the earlier branch binder. S3 still admits that read (FND-012), so the read in the example is reachable and now evaluates against an absent value. Once FND-012 is fixed with option (a), the re-fork clearing a binder a live thread reads must be resolved again; option (b) makes this finding moot. |
| FND-010 | fixed | - |
| FND-011 | fixed | - |

Round 3, head d9d55d263 (one fix commit over 771649f1d). Option (a) was taken. PS-5 now resolves a later `parallel`'s branch to the binder of `P`'s branch through the forking parent, which matches S3's scope (protocol_slots.rs: `Out` of an all-joined `parallel` adds every branch's binders, and a sequence threads them into the next control). The PS-2 hold clause is decidable and state-local. It uses the same 'can still reach a node that reads it' relation as PS-5's retention, which the code computes from rest points alone (program.rs:655-700, `reads_from`). That relation already masks the forking thread: a thread resting at or before the binding node reaches no reader of the current binding. So the forking thread at `fork(P)` does not hold its own old ordinal, and ordinals stay bounded by live readers. FT-1 agrees with it. Every live reader of a binder under `t`'s fork is `t` or a thread below it, because a reader outside that subtree can exist only after `t` has ended. The step that moves a reader past its last reader node writes that reader's `ctl`, so reading the `ctl` of every live thread at or below `t` covers each change to the hold. This is conservative and sound. The code has not caught up: `Instance::binding` and `read_by` resolve only through self, ancestors and descendants (protocol.rs:313-365), and fork's free-ordinal test has no binder clause (steps.rs:445-458). That is the coder's work, not a spec defect. QSpec main is still 44afcf4, and no companion PR is open.

| FND | outcome | reason |
| --- | --- | --- |
| FND-009 | fixed | - |
| FND-012 | fixed | - |
