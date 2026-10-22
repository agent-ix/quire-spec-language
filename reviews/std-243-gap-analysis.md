---
id: SR-13972
title: 'Gap analysis of PRs #1187 and #328: spawn/retire instance member, pop resolution, send/receive owner, predicate reads (STD-243), changed ACs against TC rows'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@949dcb17f; git diff origin/main...949dcb17f (PR #1187, branch std-protocol-243): FR-208-AC-4, FR-208-AC-5, FR-209-AC-4 (edited), FR-216-AC-6, FR-218-AC-28, FR-218-AC-29, TC-653, TC-654, TC-661, TC-663; agent-ix/quire-specification@2ad2f256 (PR #328, branch std-protocol-243-qspec): FR-426-AC-15, FR-428-AC-11, FR-428-AC-12, TC-382 RR-10 to RR-12; ticket STD-243'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-208
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-209
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-216
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-218
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-653
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-661
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-663
  type: reviews
---

## Summary

Ticket: STD-243. PRs: quire-spec-language#1187 (head 949dcb17f) and quire-specification#328 (head 2ad2f256). Method: a planless gap analysis of each new or changed AC against the TC that verifies it, spec-only. This PR adds no code, so no test tags were checked against the new ACs; the code lane adds them. The existing tests tagged FR-209-AC-4 (qsl-analyze/tests/it/protocol_roles.rs:1155, weak_memory.rs:2217) assert the combined-bound cases only, so the edit leaves them correctly bound.

Every changed AC has a TC row with procedure and expected result. FR-208-AC-4 and AC-5 are TC-653 steps 4 and 5, with the scope widened to AC-5. FR-209-AC-4 is TC-654 step 4, with the identity comparison removed. FR-216-AC-6 is TC-661 step 6, with the scope widened to AC-6. FR-218-AC-28 and AC-29 are TC-663 step 8, with the scope naming both. FR-426-AC-15 is TC-382 RR-10, and TC-382 gains a `verifies` edge to FR-426. FR-428-AC-11 is RR-11. FR-428-AC-12 is RR-12. The moved identity statement is QSpec FR-428-AC-3, verified by TC-381.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-208-AC-4 / TC-653 step 4 | spec/functional/FR-208-spawn-and-retire-replicated-role-instances.md:85; spec/test-cases/TC-653-replicated-role-instances.md:24-25,40-41 | examined |
| FR-208-AC-5 / TC-653 step 5 | spec/functional/FR-208-spawn-and-retire-replicated-role-instances.md:86; spec/test-cases/TC-653-replicated-role-instances.md:26-27,42-43 | examined |
| FR-209-AC-4 / TC-654 step 4 | spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:121; spec/test-cases/TC-654-activation-on-each-and-instance-budget.md:25,37-38 | examined |
| FR-209 bound statement | spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:89-91 | examined |
| FR-216-AC-6 / TC-661 step 6 | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md:143; spec/test-cases/TC-661-protocol-step-footprints.md:35-37,54-55 | examined |
| FR-218-AC-28, AC-29 / TC-663 step 8 | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:439-440; spec/test-cases/TC-663-s3-checks-protocol-control-constructs.md:33-34,54-57 | examined |
| QSpec FR-426-AC-15 / TC-382 RR-10 | quire-specification spec/functional/protocol/FR-426-apply-protocol-steps.md:224; spec/test-cases/TC-382-replicated-role-lifetimes.md:44 | examined |
| QSpec FR-428-AC-11 / TC-382 RR-11 | quire-specification spec/functional/protocol/FR-428-check-replicated-roles-and-per-trigger-activation.md:150; spec/test-cases/TC-382-replicated-role-lifetimes.md:45 | examined |
| QSpec FR-428-AC-12 / TC-382 RR-12 | quire-specification spec/functional/protocol/FR-428-check-replicated-roles-and-per-trigger-activation.md:151; spec/test-cases/TC-382-replicated-role-lifetimes.md:46 | examined |
| QSpec FR-428-AC-3 | quire-specification spec/functional/protocol/FR-428-check-replicated-roles-and-per-trigger-activation.md:142 | context_only |
| Existing FR-209-AC-4 tests | qsl-analyze/tests/it/protocol_roles.rs:1155; qsl-analyze/tests/it/weak_memory.rs:2217 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-209's bound statement still requires that `max_live_instances` "SHALL be a method bound with no ceiling, outside the subject and its obligation identity". With the identity sentence removed from FR-209-AC-4, no QSL AC verifies the "obligation identity" half, though the requirement stays normative in QSL. The new Dependencies bullet places that half in QSpec FR-428-AC-3 and the request writer. Phrase the clause as a reference ("outside the subject; its obligation identity excludes it as QSpec FR-428-AC-3 states") so that QSL no longer holds a SHALL that no QSL AC verifies. | spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:89-90 |

## Verdict

Mergeable on coverage grounds. Every new or changed AC in both halves has a TC row that asserts what the AC states. FND-001 is a low traceability point about a normative clause left without a QSL AC by the move. SR-13970 FND-001 (FR-218-AC-29's region clause cannot fail as written) and FND-006 (TC-382 RR-10 leaves `p` unstated) also affect what the TC rows can show.

## Dispositions

Disposition pass 1 at `agent-ix/quire-spec-language@5c5adee7a` (fix commit `5c5adee7a`) and `agent-ix/quire-specification@f9683dd`. The outcome was checked against the spec text at that head, not taken from the commit message. The fix commits keep every changed AC bound to a TC row: FR-208-AC-4's added `thread` clause is in TC-653 step 4, FR-216-AC-6's added `send`/`spawn` dependence is in TC-661 step 6, and FR-218-AC-29's region placement is in TC-663 step 8.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed 5c5adee7a | FR-209 after: "`max_live_instances` SHALL be a method bound with no ceiling, outside the subject; its obligation identity excludes it, as QSpec FR-428-AC-3 states." The identity half is now a reference to the QSpec AC that verifies it (TC-381), so QSL holds no SHALL on it without a QSL AC. ADR-027 AE-4 ("not part of the subject or its obligation identity") agrees. |
