---
id: SR-2026111116452400000003
title: "failure-domain review of PR 2679 unordered head/tail loss ordering"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language; spec/functional/FR-057-admit-shared-capability-kinds.md, spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md, spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md, spec/test-cases/TC-155-keep-admission-backend-independent.md, spec/test-cases/TC-651-protocol-step-kinds.md, spec/test-cases/TC-1314-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md, spec/decisions/ADR-027-protocol-transition-system-for-parallel.md"
review_set: subset
---

## Summary

Ticket: QSL-1877 (QSL-1868 G7d). Failure modes of the PS-7 amendment and the ST-3 head/tail ordering on `unordered` channels, checked against QSpec FR-170 "Queue transition system" (an ordered sequence of occurrences, `head` the oldest and `tail` the newest) and FR-465 "Queue". The capacity, reject/new outcomes and delivery-count interval paths are unchanged and stay consistent: occurrences are appended one after another, so each takes its own send position.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Equal entries at different positions are not handled. ST-4 says "One step per distinct deliverable entry", and ST-5 `lose(h, e)` removes "entry e". In a bag, equal entries (same send node, thread path and payload) are one element. In the new send-ordered sequence of a `loss head`/`tail` unordered channel they are not. For example, receiving either `A` from [A, B, A] gives [B, A] or [A, B], two different states with different later `head`/`tail` displacements. One implementer will collapse equal entries and lose a state, and another will step per position. Fix: state in PS-7 or ST-4/ST-5 that on a sequence queue, entries are distinguished by position: one receive or lose step per position whose entry is deliverable, with steps that reach the same post-state folded together. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:139; spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:165; spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:166 |
| FND-002 | low | ST-3's new rule "A copy that `duplicate` adds (ST-5) counts as sent at that step" fixes a choice QSpec FR-170 leaves open (where a duplicate sits in the order). No AC or TC step covers it: TC-651 step 12 uses `exactly-once-premise`, so `duplicate` is never enabled. Fix: add an `at-least-once` case to FR-206-AC-12 and step 12 in which a duplicate of P1 is then displaced by `loss tail`, not `loss head`. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:164; spec/test-cases/TC-651-protocol-step-kinds.md:44 |

## Verdict

PASS WITH FINDINGS. FND-001 should be fixed with the PR, because it decides the size of the state space. FND-002 is a coverage nit.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified in 626c658b9. |
| FND-002 | fixed | Verified in 626c658b9. |
