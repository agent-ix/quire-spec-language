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

## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-015 | high | The forking thread can no longer read the binders of its own all-joined `parallel`. PS-5 resolves first through 'the nearest of itself and its ancestors that bound it', and the forking thread bound nothing. It then resolves through its read map, but that map is recorded 'at its creation' from what its *forking* thread had joined, and it 'does not change'. The root of ADR-027 §7's `Fill` is created with an empty map and never gets an entry for `Both`. So the case FR-205-AC-2 states, 'a node after the join reads `a`' (the root reading `(A, (0, Both, left, 0))`), resolves to no binder, which is the absent-value read of FND-012 again. The previous round's rule ('from a thread it forked') covered this case, and the new text dropped it. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:137 |
| FND-016 | high | 'The ordinal of the instance of that node the forking thread most recently joined' is history, not state. A joined instance is removed (PS-4), and PS-8's `control` entry for the forking thread holds only its rest point, its disposition and its own creation-time read map. In FR-205-AC-7's iteration 2, when `t` forks `Q`, the key holds `a` under both `(0, P, left, 0)` and `(0, P, left, 1)`, and nothing in the key says which one `t` joined last. So two states with equal keys can give different successors, which contradicts PS-8's 'Two states with equal keys are one state'. FT-1's fork footprint then reads a choice that no location holds. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:137 |
| FND-017 | medium | Read-map entries never change and are never pruned, so they keep naming instances whose binders no live thread can read any more. Two states that differ only in such an entry (for example the ordinal of an earlier `P` iteration) have different keys although the protocol can do the same things from both. That breaks PS-8's own claim that the key is 'a function of what the protocol can still do, not of how it got there', and it multiplies states. Fix: keep an entry only while the binder it names can still be read, the same test PS-5 applies to removing binders. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:140 |

## New findings (disposition pass 5)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-018 | high | PS-5 drops an entry 'once no rest point that thread can reach reads the node'. A branch thread's reach ends at its branch's end, so in `branch left sequence { attempt A1; attempt A2 }` the entry for `a` is dropped when `left` comes to rest at `A2`. That is before it completes, so the completed-thread exception never applies, and `join all` then has nothing to add. The joiner cannot read `a`, the FND-015 defect again, for any branch with a step after its binding. The same wording, read literally, also drops the joiner's entry at `join(P)`: the joiner then rests at `fork Q`, and `Ck` is a node of `Q`'s branch, not a rest point of the joiner, so `fork(Q)` has nothing to copy. The code's reach relation (program.rs:655-700) spans the whole tree-order interval of the thread's scope, nested branch bodies included, and masks a rest at or before the binding node, which is the relation the rule needs. Fix: define reach as that interval, and let a branch thread of an `all` join reach on into its joiner's continuation for entries naming a thread at or below the branch (its S3 `Out`). | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:137 |
| FND-019 | high | Addition (2), 'a step that rebinds a binder a live descendant's environment still names leaves that descendant its earlier value', has nowhere to keep that value. An environment entry holds a thread path, not a value. `observations` (PS-8) holds one value per (binder node, thread path), so a rebinding by the same thread of the same node, for example in a `repeat`, overwrites it. The descendant's earlier value is in no member of the key, so two states with equal keys can differ in what a descendant reads, against PS-8. Fix: make environment entries hold the bound value, copied at `fork` and at `join all`. The descendant then keeps its copy by construction, and nothing else is needed for this case. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:137 |
| FND-020 | medium | Addition (1), 'a completed branch thread keeps those entries until its join has read it', keeps binders that no node reads. In `Fill` (no node reads `a` or `b`), `attempt(A)` completes `left`, which then keeps `A`'s entry until `join(Both)`, so `observations` holds `a` in s2 and s4. FR-205-AC-2 says 'Every reachable state of `Fill` has empty `observations`, because no node reads `a` or `b`'. Fix: keep only entries some node after the join reads, which the FND-018 reach rule gives directly. | spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md:111 |

## New findings (disposition pass 6)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-021 | medium | For nested all-joins, the reach extension is not transitive as written. PS-5 says a branch thread of an `all` join 'reaches on into its joiner's continuation'. In `parallel P { branch left sequence { parallel R { branch x attempt X binding b } join all; attempt L2 } } join all; check Cb { b.ready }`, `x`'s joiner is `left`, and `left`'s continuation (`L2`, up to `left`'s end) holds no reader of `b`. So `x` drops `b` before `join(R)`. `left` then never holds it, and `join(P)` gives the root nothing, although S3's `Out(P)` contains `b`. Fix: 'reaches on into everything its joiner reaches', which brings in `left`'s own extension into the root's continuation. Add this nested case to FR-205-AC-2. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:137 |
| FND-022 | low | `join` with policy `all` now copies entries out of each joined branch thread's environment, which is part of that thread's `ctl`. FT-1 gives `join` only `R` `inst(i)` and `ctl(t)`, and FT-2 says a join reads only 'the instance its fork or join names'. The result is still dependent in practice, because a branch's last step writes `inst(i)` and a completed branch takes no further step. Even so, the read is unstated. Fix: add 'and, under `all`, the `ctl` of each joined branch thread' to `join`'s `R`. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:294 |

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

Round 4, head d6c8c9bce (7ee7fa072 carried only SR files; the spec changes are in d6c8c9bce, over d9d55d263; base main still 5474b9125). Gates: make check-spec-validation exits 0 with the same 17 warnings, none in a changed file; tools/check-index-completeness.sh exits 0. FR-205-AC-7 now uses `u` = `sequence { attempt U1; check Ck { a.ready } }`, names the state with `U1` not yet taken, and covers two iterations, so its hold half can now be observed.

Design assessment, asked for by the coordinator. FND-012, FND-013 and FND-015 to FND-017 are four rounds of patches to one missing concept: which binder of a node each thread may read. That choice should be part of the thread's control state, rather than worked out from paths, reachability and history. A sound design is not a large rewrite, but it replaces the PS-5 resolution rule and the PS-2 hold clause instead of adding to them:
- Each thread's `ctl` holds a binder environment: for each binder node whose value it may read, the thread path that bound it.
- An event step adds its own binder to its thread's environment.
- A `join all` step adds every joined branch's binders to the joining thread's environment. It already writes `ctl(t)`.
- A fork copies the parent's environment into each new branch thread. It already writes the new threads' `ctl`.
- Entries are dropped when no rest point of that thread can reach a reader. That is PS-5's existing test, applied per thread.
- A binder is kept while some live thread's environment names it, and an ordinal is held while its instance is live, a live thread's path extends one of its branch paths, or some live thread's environment names a binder under it.

All of that depends on the current state only, it is finite, and it matches S3's In/Out scope (protocol_slots.rs). FT-1 then needs nothing beyond what it already has: `join` writes `ctl(t)`, `fork` writes the new threads' `ctl`, and `fork` reads the `ctl` of live threads at or below `t`. This is a semantic design choice (binder environments as protocol state, which QSpec FR-425 and FR-466 would carry too), so it should be decided once, not patched again. The code has none of it yet (protocol.rs:313-365, steps.rs:445-458).

| FND | outcome | reason |
| --- | --- | --- |
| FND-013 | still-open | The read map makes a descendant's choice unique, but the rule is not well-defined: the forking thread has no entry (FND-015), and 'most recently joined' is not in the key (FND-016). |
| FND-014 | fixed | - |

Round 5, head 943685e5a (one commit over d6c8c9bce; base main still 5474b9125). Gates: make check-spec-validation exits 0 with the same 17 warnings, none in a changed file; tools/check-index-completeness.sh exits 0.

Hand derivations, using the code's reach relation (tree-order interval of the scope, masked at or before the binding node). FR-205-AC-2's variant (a node after `join(Both)` reads `a`): `attempt(A)` adds `A` to `left`'s environment, and `left` completes at once and keeps it. `join(Both)` adds it to the root's environment, which reaches the reader, so the binder lives from `attempt(A)` to the reader's step. The variant holds. The base `Fill` does not hold (FND-020). FR-205-AC-7, iteration 1:
- `join(P)` gives `t` `a` -> `(0, P, left, 0)`, and `fork(Q)` copies it to `u` and `v`.
- `t`, resting again at `fork P`, sits before the binding node and drops its entry.
- With `U1` not yet taken, `u`'s entry holds ordinal 0, so the second `fork(P)` takes 1.

Iteration 2:
- `Q`'s instance 0 is still live while `u` runs, so `Q` forks at 1, and `u2` names `(0, P, left, 1)`.
- `u` evaluates `Ck` true over ordinal 0 and `u2` false over ordinal 1, and no state refuses.
- When `u` completes, ordinal 0 is freed.

The AC holds under that reach relation, but not under PS-5's 'rest point' wording (FND-018).

Nested and compensation cases. An inner all-joined `parallel` inside a branch passes its binders up through two joins, and holds only if the branch keeps them (FND-018). A compensation thread is created by activation, not by a fork. It starts with an empty environment and reads its captures and its own attempt binders, which is consistent with CP-1 to CP-5. S3 In/Out: the copy at fork matches In(branch) ⊇ In(`parallel`), and the join adding branch entries matches Out of an all-joined `parallel`. A `choice`, `repeat` or `await` adds nothing past itself, which pruning gives. FT-1: event steps, `join` and `fork` already write the `ctl` that holds the environment, so the footprints are complete for the choice.

Convergence. One decision remains: **do environment entries hold the bound value (copy at `fork` and `join all`) rather than a thread path?** With values:
- FND-019 is resolved by construction.
- PS-2's binder hold clause, the fork's clearing of reused paths' `bind` locations, and FT-1's `ctl` read of every live thread below `t` for that purpose can all be deleted, because a reused path can no longer take a value someone reads.
- The key merges states that differ only in which path bound an equal value.

With paths, addition (2) needs a second value store in the key. FND-018 and FND-020 are wording fixes to the reach relation under either choice.

| FND | outcome | reason |
| --- | --- | --- |
| FND-013 | fixed | - |
| FND-015 | fixed | - |
| FND-016 | fixed | - |
| FND-017 | fixed | - |

Round 6, head 3a6b41fea (one commit over 943685e5a; base main still 5474b9125). Gates: make check-spec-validation exits 0 with the same 17 warnings, none in a changed file; tools/check-index-completeness.sh exits 0.

Hand derivations under the new PS-5.

FR-205-AC-2:
- Base `Fill`: `left` completes at `attempt(A)` and reaches no reader, so it drops `a` in that step, and `observations` is empty in every state.
- Variant with a reader after the join: `left`'s reach extends into the root's continuation, so it keeps `a` until `join(Both)`. The join adds `a` to the root, which keeps it to the reader's step.
- Multi-step `left` (`A` then `A2`): resting at `A2`, `left` still reaches the reader through the extension, so `a` is held to the join and then by the root. All three hold.

FR-205-AC-7:
- After `join(P)`, `t` reaches `Ck`: it is inside `Q`'s branch, which nested bodies include, and also via the back edge after `fork(Q)`. `fork(Q)` copies `a`(true) to `u` and `v`.
- `join(Q)` settles `t` back to `fork P`, before the binding node, so `t` drops its entry. `P`'s instance 0 is gone and nothing extends its paths, so `P` forks at 0.
- `Q`'s instance 0 is live while `u` runs, so `Q` forks at 1. `u2` gets `a`(false), `u1` keeps `a`(true), and `u1` drops it on completing (`join any` gives no extension). The AC holds.

Repeat: a rest at or before the binding node reaches nothing, so a stale value is dropped at the loop head, and the next iteration's binding replaces it.

Compensation threads start empty and read their captures and their own attempt binders, which is consistent with CP-1 to CP-5.

S3 In/Out: the copy at fork matches the `In` of the branches. The `all` join adds `Out` and the other policies add nothing. `await` (`then` only), `choice` and `repeat` follow from reach.

PS-8 is now a function of state. Values are keyed by the thread that holds them and pruned, and two states that differ only in which thread bound an equal value are one state.

FT-1 without the extra read: a fork's ordinal no longer depends on environments, and every environment change is made by a step of the owning thread (an event, fork, join or prune) that already writes its `ctl`. The one gap is the join's copy, FND-022.

Major findings: none. FND-021 is medium and FND-022 is low. FND-008 (QSpec companion) stays deferred.

| FND | outcome | reason |
| --- | --- | --- |
| FND-018 | fixed | - |
| FND-019 | fixed | - |
| FND-020 | fixed | - |
