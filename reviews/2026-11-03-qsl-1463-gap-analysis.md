---
id: SR-2239005
title: "Gap analysis of QSL-1463 PR #2257 against FR-251 (AC-37, AC-38, AC-39, AC-42), FR-487-AC-13 and TC-1081"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ede08267050c252deecec764fe7beb3c592b5e3b; PR #2257 (git diff origin/main...HEAD): qsl-package/src/emit/monitor_plan.rs, qsl-package/src/emit.rs, qsl-analyze/tests/it/monitor_root.rs; qsl-replay/src/monitor_plan.rs read for PlanId"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-251
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1081
    type: reviews
---
# SR-2239005: Gap analysis of QSL-1463 PR #2257

## Summary

Ticket: QSL-1463. Plan completion: not assessed. Spec read: QSL FR-251, FR-357, QSpec FR-487 (call closure, AC-13) and FR-322-AC-309 at quire-specification 78dcf2a90.

| AC | Status | Bound by | Honest? |
| --- | --- | --- | --- |
| FR-251-AC-38 | tagged | monitor_root.rs:400 | root-key half only; the PlanId half is not asserted (FND-001) |
| FR-251-AC-42 | tagged | monitor_root.rs:473 | declaration/field rows and ids yes; re-keying and other row kinds no (see SR-2239004 FND-001) |
| FR-251-AC-37 | untagged | - | no test anywhere (FND-001) |
| FR-251-AC-39 | untagged | - | no test anywhere (FND-001) |

FR-357 is the scalar-parity replay requirement; it states nothing about retained emission rows, so it does not own the source-owned row question below.

Source-owned rows: IR's quire-contract-model (locked 64de9ec) has SelectedEnumDeclarationRow and SelectedEnumMemberRow for source-owned enums; QSL builds only the model-owned SelectedModelEnum* rows (QSL-1398, Done, scoped selected enums from the selected DomainPackage). No collection row type exists in that IR version, and union rows are produced through SelectedExactDeclarationRow (retained by this PR). No FR in this repository, and no Linear issue found by ix-board search, owns a QSL producer for source-owned enum rows. The PR does not drop them: the Emission has never had a field for them and the new FR-251 paragraph does not promise them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-251-AC-37, AC-38 (PlanId half) and AC-39 are untested and, by code reading, not met: PlanId::id() holds the subject (clause NodeKey), binding, premises, topology, activation and atoms as (AtomId, slot count) only, so a TimedMonitorPlan built through its public constructor with one atom or guard body swapped has an equal PlanId (AC-39), and a helper body swap changes neither the subject nor any PlanId field (AC-38). The test tagged FR-251-AC-38 asserts only the plan-root key, so the tag over-claims the AC's PlanId half. Not run (read only). Out of this PR's slice but it is the same ticket's FR-251 identity requirement and no ticket tracks it. | qsl-replay/src/monitor_plan.rs:315-450; spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md AC-37..AC-39 |
| FND-002 | medium | No requirement and no ticket owns a QSL producer for source-owned enum declaration/member rows (IR SelectedEnumDeclarationRow/SelectedEnumMemberRow), and the new FR-251 paragraph lists "the enum rows" without saying they are the model-owned ones. Not a defect of the PR's behavior (nothing is silently dropped by it), but the gap is unowned: a consumer that selects a source-declared enum has no row to authenticate. | spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md:119-126 |

## Verdict

Not blocking on its own: FND-001 and FND-002 are out of the PR's slice, but each needs an owning Linear ticket before QSL-1463 is treated as closed for IR-721/IR-722; the code lead should file them (tickets live in Linear, project QSL V1 Architecture Spine, Layer 4 - Semantic facets).

## Dispositions

Round 1, at 81aa7e1c7.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-1630 (Backlog, QSL V1 Architecture Spine) owns FR-251-AC-37, the PlanId half of AC-38 and AC-39; the root-key test is retagged FR-487-AC-13 and its doc says the PlanId half is not asserted, so the over-claim is gone. QSL-1630 shows a project but no milestone or relations: the code lead should add them. |
| FND-002 | deferred | QSL-1631 (Backlog, QSL V1 Architecture Spine) owns the source-owned enum, collection and union row producers; the Emission field doc now states those row kinds have no producer yet and the FR-251 paragraph names the model enum rows. QSL-1631 shows a project but no milestone or relations: the code lead should add them. |
