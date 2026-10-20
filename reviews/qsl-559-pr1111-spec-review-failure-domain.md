---
id: SR-13591
title: 'Spec review of PR #1111: protocol specs aligned with QSpec PR #302, failure domain'
type: SpecReview
analysis: failure-domain
scope: 'agent-ix/quire-spec-language@91a8b0938; git diff origin/main...91a8b0938 (PR #1111, branch qsl-adr027-followup, base main 693c16869): ADR-027 PD-4, PS-5 and FR-211 wait causes; read against ADR-027 ST-11, ST-14, CP-3, CP-4, CP-5, RR-1 to RR-4, FR-205-AC-2, FR-205-AC-7, TC-650 and QSpec PR #302 head f60002b FR-429; tickets QSL-559, QSL-562'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-027
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-211
  type: reviews
---

## Summary

Tickets: QSL-559, QSL-562. PR quire-spec-language#1111, head 91a8b0938. This file looks for waits the new causes leave without a unique cause, and for binder states the PS-5 narrowing changes.

The new causes. `FinishWaiting` (root at `finish` with another `running` thread), `CommitClosed` (CP-4: no further `cattempt`, and `cend` only at the retry point) and `NoRoleInstance` (no live instance of `r` and no enabled `spawn`, ST-14) each describe a reachable wait that no earlier cause covered. A compensation thread at its retry point after commit is not waiting, because `cend` is always enabled (ST-12), so `CommitClosed` is limited to the first attempt as it should be.

PS-5 and the earlier derivations. FR-205-AC-2's nested variant still holds under the narrowing. `b` is bound in branch `x` of the inner all-joined `R`, so it is in `Out(x)` and, recursively, in `Out(left)`, and it stays held by `x`, then `left`, then the root. FR-205-AC-7 also still holds: `a` is bound directly in branch `left` of the all-joined `P`, so `join(P)` adds it and `fork(Q)` copies it. The S3 `Out` computation (protocol_slots.rs:1495-1537) gives the same sets. The wording of the exclusion is the subject of SR-13589 FND-004.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| ADR-027 PD-4 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 PS-5 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| ADR-027 ST-11, ST-12, ST-14, CP-3, CP-4, CP-5, RR-1 to RR-4 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | context_only |
| FR-211 Behavior (wait causes) | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md | examined |
| FR-205-AC-2 | spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md | examined |
| FR-205-AC-7 | spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md | examined |
| TC-650 steps 2 and 7 | spec/test-cases/TC-650-protocol-subject-and-state-key.md | context_only |
| QSpec FR-429 at f60002b | agent-ix/quire-specification spec/functional/protocol/FR-429-report-protocol-deadlocks.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A thread at a compensation attempt has no cause order, so its cause is not unique. PD-4 and FR-211 require 'exactly one cause: ... the first that applies in this order for its rest point', but their order lists cover only `attempt`, `event`, `receive`, `send` and `finish`. A compensation thread at its first attempt after the commit step can be `CommitClosed` and, at the same time, fail ST-1's conditions for the template's operation (`Precondition`, `NoPostState`, `Constraint`, ST-11). If its role is replicated and has no instance, it can also be `NoRoleInstance`. QSpec FR-429 at f60002b adds the line 'a compensation attempt (`cattempt`): `commit-closed`, `precondition`, `no-post-state`, `constraint`' and puts `no-role-instance` first for a role-owned compensation attempt. Add the same line to PD-4 and FR-211. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:251; spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md:113-127 |

## Verdict

Not mergeable on this file's finding alone: one rest point the new causes reach has no defined unique cause (FND-001), and QSpec #302 has already closed that gap. The other new waits are sound, and the PS-5 narrowing keeps FR-205-AC-2 and FR-205-AC-7 as #1109 derived them.
