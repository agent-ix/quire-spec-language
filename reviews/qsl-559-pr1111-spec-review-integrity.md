---
id: SR-13590
title: 'Spec review of PR #1111: protocol specs aligned with QSpec PR #302, integrity'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@91a8b0938; git diff origin/main...91a8b0938 (PR #1111, branch qsl-adr027-followup, base main 693c16869): ADR-027 PS-5, PS-7, ST-3, PD-4 and FR-206, FR-211, FR-218, TC-651, TC-656, TC-1056; read against ADR-027 PS-8, PX-1, FT-1, SE-1, SE-5, FR-205, FR-217, TC-650, code qsl-eval/src/simulation/protocol/{steps,channel,identity}.rs and qsl-semantics/src/check/protocol_slots.rs on this head, and QSpec PR #302 head f60002b FR-170, FR-425, FR-426, FR-429, FR-433, FR-434, FR-466; tickets QSL-559, QSL-562'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-027
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-206
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-211
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-218
  type: reviews
---

## Summary

Tickets: QSL-559, QSL-562. PR quire-spec-language#1109's follow-up, quire-spec-language#1111, head 91a8b0938, spec only (7 files, +44/-22). This file checks that the changed rules agree with QSpec PR #302 at its current head f60002b, with the rest of ADR-027 and with the code they cite.

QSpec #302 moved on after the commit this PR was written against (0825a54). Its rounds 1 to 3 (e5df81b, b8e9d97, f60002b) widened `no-role-instance` to send and receive nodes, added a `cattempt` order line, made the send record a `send_record` member of a counterexample `send` step, and reworded FR-429-AC-15. QSL does not have these yet (FND-001, FND-002 here; SR-13591 FND-001; SR-13589 FND-001).

Agreement that holds. ST-3, FR-206, FR-206-AC-4/AC-7, FR-218-AC-11 and TC-651 step 7 bind the send binder to the payload, of the `carries` type, and keep the send record out of the binder and the state key, as QSpec FR-425, FR-426 and FR-170 do. The unlisted edits (FR-218's eligibility row `send` payloads, FR-218-AC-17 'a `send` binder', TC-1056 row 30) match the S3 code, where a `send` binder is `Eligibility::Never` (protocol_slots.rs:481-494). `FinishWaiting` and `CommitClosed` match FR-429's meanings, and so does the `finish` order. `Gate` including a fence matches FR-429's `gate` row, FR-434's gate row and ADR-027 SE-1 (e) and SE-5. PS-7's `<protocol>::<handle>` matches FR-429's qualified channel name. The PS-5 narrowing to the branch's `Out` matches FR-425, except for `await` (FND-003).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| ADR-027 PS-5 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 PS-7 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 ST-3 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 PD-4 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 PS-8, PX-1, PX-2, FT-1, SE-1, SE-5 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | context_only |
| FR-206 send bullet, FR-206-AC-4, FR-206-AC-7 | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md | examined |
| FR-211 Outputs and Behavior (wait causes) | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md | examined |
| FR-218 Channels, eligibility table, FR-218-AC-11, FR-218-AC-17 | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| TC-651 step 7 | spec/test-cases/TC-651-protocol-step-kinds.md | examined |
| TC-1056 row 30 | spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md | examined |
| FR-217 | spec/functional/FR-217-replay-a-protocol-counterexample.md | context_only |
| protocol_slots.rs Out and eligibility | qsl-semantics/src/check/protocol_slots.rs | context_only |
| QSpec FR-170, FR-425, FR-426, FR-429, FR-433, FR-434, FR-466 at f60002b | agent-ix/quire-specification spec/ | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `NoRoleInstance` has a narrower scope here than in QSpec #302. QSpec FR-429 at f60002b: the `no-role-instance` row covers 'a thread at an event, attempt, send, receive or compensation-attempt node owned by replicated role `r`', and the order line is 'an `attempt`, `event`, `send` or `receive` node, or a compensation attempt, owned by a replicated role (`by r`): `no-role-instance` first'. QSL PD-4 and FR-211 say 'an event, attempt or compensation-attempt node' and order only 'An `attempt` or `event` owned by a replicated role: `NoRoleInstance` first'. Take a `receive` owned by a replicated role with an empty population and nothing queued. QSL reports `EmptyQueue`, QSpec `no-role-instance`, so the same deadlock gets two different causes. Copy QSpec's row and order line into PD-4 and FR-211. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:251; spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md:122-127 |
| FND-002 | medium | The send record has no carrier in QSL. ST-3 and FR-206 say only that the step 'records a send record as its result', not in the binder or the key. QSpec #302 at f60002b carries it as the `send_record` member of each `send` step of a counterexample (FR-426, FR-433 wire table). Replay recomputes and compares it: a recorded record that differs refuses `stale_dependency`/`revision-mismatch`, and a missing or extra one is a wire-shape refusal before replay (FR-433-AC-8). PX-1 lists each step as its transition identity and post-state digest only, and FR-217 is unchanged. So a QSL counterexample cannot carry what QSpec replay requires. Add the carrier to PX-1, PX-2 and FR-217, or record it as an explicit follow-up. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:164,278-279; spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md:77-80 |
| FND-003 | medium | PS-5's list of entries an `all` join does not add leaves out `await`. Its list is 'an entry bound below an inner join of another policy, in a `choice` case or in a `repeat` body'. QSpec FR-425 also excludes 'an `await` match outside its `then` path'. FR-466's table gives `Out(await)` as `In(await)`, and the S3 code maps `Await` to its input (protocol_slots.rs:1528-1536). As written, PS-5 adds a binder that a branch bound inside an `await`, though S3 refuses every read of it after the join. Add `await` to the list. FR-466 says that neither the matched event nor a `then` binder escapes, so that is the wording to use. QSpec's own 'outside its `then` path' is narrower than FR-466, which is worth relaying to the QSpec lane. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:137 |
| FND-004 | low | PS-5 still opens with 'Each live thread's `ctl` holds a **binder environment**'. PS-8 keys environments in `observations` by (binder node, thread path), and QSpec FR-425 now says the environment is 'keyed in `observations` ... and not in the thread's `control` entry'. QSpec made this exact fix in its own review (SR-13588 FND-007). Use the same wording here. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:137,140 |

## Verdict

Not mergeable. The PR's three stated changes are right against QSpec #302 at 0825a54, and the two unlisted edits are consistent with the code and with QSpec. But #302 has moved on: its head widens `no-role-instance` to send and receive nodes, which QSL contradicts (FND-001), and gives the send record a carrier that QSL lacks (FND-002). PS-5's narrowing also misses `await` (FND-003).

The code on this head lags the spec. That is expected for a spec PR, but the implementation ticket needs to know. `WaitCause` has no `Displaced`, `FinishWaiting`, `CommitClosed` or `NoRoleInstance` (identity.rs:371-417). `finish` reports `JoinWaiting` (steps.rs:990-1003). The send step binds a `{payload, outcome}` record with outcomes `sent`/`displaced` as the binder's key encoding (channel.rs:62-70, 235-245), not the payload and the `{channel, payload, outcome, displaced}` record of ST-3.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The new wire-shape refusal names no code. FR-217's bullet says the executor 'SHALL refuse a `send` step without a `send_record`, and any other step with one, before replay, as a wire-shape defect'. PX-2 and FR-217-AC-6 say the same. Every other refusal in FR-217 names its code and subcode, so two implementations could refuse these with different codes and both pass AC-6. Merged QSpec FR-433 names no code here ('refuses before replay, like any missing or extra member'), so QSL can choose one without contradicting it. QSL already has a precedent: a hyper counterexample missing a member is refused by the reader 'as `invalid_runtime_input`/`invalid-value` before replay' (FR-183-AC-6). Name `invalid_runtime_input`/`invalid-value` in PX-2, FR-217 and FR-217-AC-6, and the code TC-662 step 5 expects. The current `replay_model_trace` takes a typed envelope and has no counterexample wire decoder (qsl-replay/src/model_check/replay.rs:405-420), so the spec has to name the code. | spec/functional/FR-217-replay-a-protocol-counterexample.md:86-88,122; spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:279; spec/test-cases/TC-662-replay-a-protocol-counterexample.md |

## Dispositions

Round 1, head d4cd7a4f6 (one commit over 91a8b0938; base main 693c16869). Checked against merged QSpec main 4ea88c8, whose spec text is the same as f60002b's. Gates: `make check-spec-validation` exits 0, with 17 warnings, none in a changed file; `tools/check-index-completeness.sh` exits 0. The new refusal for a differing `send_record` is `stale_dependency`/`content-mismatch`, where QSpec FR-433 uses `revision-mismatch`. This follows QSL's own digest refusal in PX-2 and FR-217-AC-2, which already differed from QSpec FR-433-AC-2 before this PR, so it is consistent within QSL and outside this delta. reviews/ already holds the round-0 SR files byte for byte, and must be re-copied for this round's sections.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | PD-4 and FR-211 now give `NoRoleInstance` for event, attempt, send, receive or compensation-attempt nodes owned by a replicated role, first in the order, as merged FR-429 does. |
| FND-002 | fixed | PX-1 now carries `send_record` on `send` steps only, outside the key and the transition identity. PX-2, FR-217 and FR-217-AC-5/AC-6 recompute and compare it, and TC-662 step 5 tests that. The missing refusal code is recorded as FND-005. |
| FND-003 | fixed | PS-5 now excludes entries bound in an `await`, and says that its matched event and `then` binders do not escape. That matches FR-466's Out(await) = In(await) and protocol_slots.rs:1528-1536. |
| FND-004 | fixed | PS-5 now says that the environment is keyed in `observations` by binder node and thread path, and not in the thread's `control` entry, matching PS-8 and merged FR-425. |
