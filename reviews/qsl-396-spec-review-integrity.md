---
id: SR-1011
title: "QSL-396 integrity review of ADR-027 and its amendments to ADR-011 and ADR-018"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@4873939885cbe47d6da1bf6d47074d695dd0586f; spec/decisions/ADR-027-protocol-transition-system-for-parallel.md; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md; spec/decisions/ADR-016-state-model-finite-execution-mapping.md; spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md; spec/functional/FR-205..FR-218"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---

## Summary

Ticket: QSL-396. This review checks cross-artifact consistency:
- ADR-027's local ids;
- FR-205 to FR-218 against the ADR;
- the in-place amendments to ADR-011 and ADR-018;
- the records ADR-027 changes but does not amend.

**Consistent:**
- Every local id cited (PS, FO, ST, AT, CI, SE, PB, PD, PA, TS, PX, FT, PR, QS, DS, RU, AE, RR, CP) resolves to a defined row.
- RU-1 to RU-5 match the owner's answers recorded on QSL-396.
- The ADR-011 E10 and module-table amendments match TS-1 and TS-6.
- The ADR-018 SM-2 to SM-4, FA-1, FA-2, CX-2, CX-3, DL-1, DL-2 and IV-2 amendments match PS-1, PB-1 to PB-4, PA-1 to PA-3, PX-1, PX-2, PD-1 to PD-3 and PB-3.
- PD-5 and FR-211 settle the PD-2 activation-only deadlock against the instance-limited boundary state consistently, and so does QSpec FR-429-AC-4.
- spec.md and tests.md carry a row for every new artifact.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AE-3 and FR-209's Removal bullet remove an instance in the step that finishes it, for every activation kind, and its registrations and role instances go with it. Several texts keep a finished `on origin` instance in the state: PS-8 and FR-205's `control` key give each live instance a finished flag; §7's s6 is "end, finished"; §7.1's t5 is "finished, closed"; FR-207-AC-1 has the registration "`closed` in the finished state t5"; FR-208-AC-3 has role instances "`retired` in the state after `finish`"; and PX-2 checks "the protocol is not finished". Either scope removal to `on each`, or restate the `on origin` texts. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:204,359,453; spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:83-85; spec/functional/FR-207-run-compensation-templates-as-protocol-threads.md:107; spec/functional/FR-208-spawn-and-retire-replicated-role-instances.md:87 |
| FND-002 | medium | ADR-016 FP-4 states that `ModelSystem` is simulation's only production `TransitionSystem`. TS-1 and FR-215 add `ProtocolSystem` beside it, but ADR-027's "Amendments made with this record" does not amend FP-4, so the two records contradict each other. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:276,565-582; spec/decisions/ADR-016-state-model-finite-execution-mapping.md:364 |
| FND-003 | low | ADR-027 lists the ADR-017 PF-7 amendment under "Amendments to make on acceptance", which it introduces as "drafts on other branches". ADR-017 is in this tree, and PF-7 says "No redesign is authorized". FR-218's Description cites "ADR-017 PF-7, as amended by ADR-027", an amendment this PR does not make. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:587,638-640; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:41-43 |
| FND-004 | low | ST-13's enabled column and FR-209's Activation bullet make `activate` enabled only while "the caller's instance budget admits one more live instance". AE-4 and FR-209's budget bullets say that at the budget the step is enabled, not expanded, and read as enabled by FA-2. One step cannot be both disabled and enabled. The first reading also puts a run parameter into the subject's successor relation, which AE-4 says it is not part of. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:184,205; spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:75-79,94-98 |
| FND-005 | low | ADR-018 FA-5 fixes the constraint type as `FairnessConstraint{kind, operation, granularity}`. PA-1 adds attempt-node, branch, root and template targets, and PA-3 adds a `scheduler` origin, but FA-5 is not amended for either. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:266; spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:565-579 |
| FND-006 | low | The ADR-018 V-6 amendment is appended with no sentence break: "as ADR-013 C-09 Amended by ADR-027 AE-4: …". | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:152 |

## Verdict

Cross-references resolve and the ADR-011/ADR-018 amendments match the rules
they cite. FND-001 is the substantive inconsistency: the record does not say
whether a finished `on origin` instance stays in the state. FND-002 leaves
ADR-016 contradicting this record. The rest are small alignment fixes.
Separately from these findings, the base branch was rebased, and this PR now
conflicts with spec/366-temporal-properties in ADR-018 V-6, CX-2 and CX-3
(GitHub reports CONFLICTING). It has to be rebased before it can merge.
