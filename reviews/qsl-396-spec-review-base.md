---
id: SR-1010
title: "QSL-396 spec review of ADR-027 and FR-205 to FR-218"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@4873939885cbe47d6da1bf6d47074d695dd0586f; spec/decisions/ADR-027-protocol-transition-system-for-parallel.md; spec/functional/FR-205..FR-218; spec/test-cases/TC-650..TC-663; spec/usecase/US-024; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-209
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-211
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: reviews
---

## Summary

Ticket: QSL-396. PR agent-ix/quire-spec-language#575, the PR's own commits
74f43335 to 48739398 (stacked on spec/366-temporal-properties at 404a5a88).
QSpec counterpart read on origin/spec/wave-b-q5-memory-protocol: FR-052,
FR-425 to FR-433.

Checked against the owner rulings on QSL-396 and the plan-lead-accepted
calls:
- explicit step rows for every step class: PR-1, FR-214. Met.
- compensation, roles and activation designed here: §2.2a, FR-207 to FR-209. Met.
- deadlock opt-out per protocol: PD-1, PD-3, FR-211. Met.
- scheduler fairness on by default, `scheduling adversarial` opt-out: PA-3, FR-212. Met.
- control steps uncounted for intervals: PB-3, FR-213. Met.
- `max_live_instances` default 3, stated in the result: AE-4, FR-209. Met.
- a `repeat` needs only a progress edge: PS-6, FR-218. Met (see FND-006 on the edge's definition).
- `finish` waits for every thread: ST-9, FR-206. Met in QSL (see FND-007 on QSpec).

Also met: no fixed caps and no depth limit kind; no pins, digests, ledgers or
provenance; no compat paths; ticket ids only in References; undefined
evaluation refutes with `UndefinedEvaluation` (PB-5, FR-217); every AC has a
behaviour TC (TC-650 to TC-663 cover all 57 ACs); `quire validate` on the 34
changed files exits 0 and `tools/check-index-completeness.sh` passes. The
worked examples in §7 and §7.1 were recounted by hand and agree (7/6, 10/9
and 9/8 states/transitions; the interval verdicts).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-205's state key keys `queues` by channel (and FIFO key) and `roles` by role only, with no protocol instance, while AE-1 gives each `on each` instance its own queues, role bindings and replicated role instances. Two states that differ only in which instance holds a queue entry or a role instance get equal keys and are coalesced into one state, which is a wrong result. QSpec FR-425 keys both members by instance ordinal. ADR-027 PS-7 and PS-8 have the same gap. | spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md:125-129; spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:149-150 |
| FND-002 | high | FR-215's (and TS-2's) protocol transition identity gives an `attempt`/`cattempt` the operation and argument vector but no receiver. It also gives `finish` no record, `cattempt`/`cend` no template or registration, and a channel no instance ordinal. QSpec FR-426's identity table has all of these. Without the receiver, two attempts on different receivers with equal arguments share one identity, which contradicts PA-1/FR-212's `each` class per (operation, receiver, arguments). QSL-built counterexamples would also not match QSpec's wire. | spec/functional/FR-215-implement-protocolsystem-as-a-transitionsystem.md:101-111; spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:277 |
| FND-003 | medium | FR-205 to FR-209, FR-211 and FR-213 restate QSpec-owned semantics as normative SHALL rules: the state key contents, step enabledness and effects, compensation, roles, activation, deadlock and counted steps. Their own Dependencies sections say QSpec owns these (QS-1 to QS-13). FND-001, FND-002, FND-004 and FND-008 are drift that already exists between the two copies. The QSL FRs should cite QSpec FR-425 to FR-431 for the meaning, and keep only the compiler and engine obligations and the ACs. | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md:31-38,61-136 |
| FND-004 | medium | FR-211 refuses a second protocol `terminal` member with `conflicting_declaration`/`duplicate-declaration`; QSpec FR-429-AC-6 refuses it with `ambiguous_declaration`/`ambiguous-name`. The QSL engine and the QSpec conformance vector would disagree on the code. | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md:78-80,120 |
| FND-005 | medium | AE-4 and FR-209 make an `InstanceBoundReached` result name the bound and the count of limited states, but not the request member that raises it. The owner rule is that reaching a bound names the limit, its value and how to raise it, and QSpec FR-428 requires the result to name `max_live_instances`. FR-209-AC-1 does not test it. | spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:102-105,117 |
| FND-006 | medium | PS-6 and FR-218 define a `repeat`'s progress edge as "its body holds at least one node that takes a step", and PS-6 concludes "so settling always reaches a rest point". That conclusion does not follow. Take a body `choice` with one case holding an attempt and another holding only a true `check`. It passes the rule, but an iteration through the second case takes no step, and because guards read immutable binders (FO-2) settling loops forever. The rule needs every path through one iteration to take a step, or a stated reading of the empty path. QSpec FR-052 has the same wording. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:148; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:92-95 |
| FND-007 | medium | ST-9 and FR-206 disable `finish` while any other thread of the instance is `running`, which matches the team-leader call. QSpec FR-426's `finish` row has no such condition: its enabled column is only "some record of the finish type satisfies the finish constraint", and FR-429 instead defines "finished" as finish having run plus every thread off `running`. The two repos give different behaviours for the same protocol. The fix is on the QSpec branch. | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md:128-133 |
| FND-008 | low | FR-206 says the protocol system takes "exactly" the kinds `attempt` to `finish` plus memory steps, which leaves out `fence`. QSpec FR-426 lists `fence` as a step kind with its own identity, and FR-213 and PB-3 name fence steps as uncounted. | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md:33-36 |
| FND-009 | low | FR-209-AC-2's second sentence ("With the claim `always holds(k.v = 0)` and `max_live_instances` 1, the run settles V-4 with a counterexample `activate(Tick)`, `attempt(set)`") follows "The same protocol with guard `{false}`", under which no `activate` exists. The AC does not say the guard returns to `{true}`. | spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:118 |

## Verdict

The design follows every owner ruling and both team-leader calls, and the
worked examples check out. Two high findings block merging. The state key
loses the protocol instance for queues and roles under `on each` (FND-001),
and the transition identity loses the receiver (FND-002). Both are drift from
QSpec FR-425 and FR-426, which is FND-003's point: the QSL FRs carry a second
normative copy of the QSpec semantics, and it has already diverged in four
places. FND-007 and part of FND-006 need a QSpec-side fix.
