---
id: SR-9014
title: "Integrity and cross-reference review — QSL-750 model-profile selection and gating (PR #813)"
type: SpecReview
analysis: integrity
date: 2026-10-13
scope: "agent-ix/quire-spec-language@aefcc4103c331ab54907b1dd60c97f8fb5d96b64; spec/functional/FR-361, FR-102, FR-110, FR-302, FR-304, FR-308, FR-309; spec/test-cases/TC-1012..TC-1017; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-361
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-308
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-309
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1012
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1014
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1015
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1016
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1017
    type: reviews
---

## Summary

Ticket: QSL-750. PR #813 (base impl/business-model-meanings) adds FR-361 and TC-1012 to TC-1017, and edits FR-102, FR-110, FR-302, FR-304, FR-308 and FR-309. The review is against QSpec main (shared grammar, model-complete, state-core, complete-value and complete-model locks, native-diagnostics, ADR-006, FR-450, FR-453) and the planner rulings (a) to (f) recorded on QSL-750.

## Verdict

**CHANGES REQUESTED.** Two medium findings: a double refusal against FR-110's layer rule, and the `root` role collision and incomplete lock rows when the complete-model lock is merged. IDs FR-361 and TC-1012 to TC-1017 are unique across every origin branch and every open PR head. `make check-index-completeness` passes, and the spec.md and tests.md rows are present. `make check-spec-validation` exits 0 (quire 0.36.2, engine 0.50.2) with no warning on any changed file.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Item 7 keeps FR-110's per-alias layer rule beside the new unit-level test. QSpec's value-complete and state-core admitted forms do not include `operation-body`, the dispatched call, `lookup` or `abstraction-decl`. So once FR-110-AC-7 and AC-8 are built, a `body ... using V` with `V` resolved to `quire.value.complete/v1` gets two `unsupported_construct` refusals at one span. One comes from the layer rule and names the selected layer; the other comes from FR-361 and names the required layer. AC-6 requires one refusal per construct, and AC-1 expects one. FR-361 gives no precedence or deduplication rule. Say which rule reports, or that FR-361's refusal replaces the layer rule's for the item-5 families. | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:136-138,188 |
| FND-002 | medium | Item 3 puts the complete-model lock's rows into `DefinitionLock::pinned()` beside the complete-value lock's. The complete-model lock's `qualification_catalog` has its own `root` row (`quire.model.complete/v1`) and a `state_graph` row. The complete-value lock already has `root` = `quire.value.complete/v1`. FR-110's `ProfileRefusal` retains "the required role (`root`)" and "the `root` row it was compared with", and today's `resolve_profiles` reads `lock.entry(Root)`. FR-361 does not say how the two `root` roles coexist, whether they are namespaced per lock or which one a refusal names. Separately, AC-8 lists the complete-model lock's `package_selection.always` rows but not the complete-value lock's `package_selection` rows. FR-110 "Lock rows" selects those whenever the union holds `quire.value.complete/v1`, which the model closure does. So the AC under-states the expected lock. | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:78-83,190 |
| FND-003 | low | Item 5 says "QSpec shared grammar names the first, the third, the fifth and the sixth" rows, but the table has five rows. QSpec's shared grammar names rows 1, 4 and 5 (`operation-body`, `dispatch-call`, `lookup`). Row 3 (`abstraction`) comes from QSpec FR-450-AC-3. The ordinals are wrong. | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:91-93 |
| FND-004 | low | Cross-reference sweep. Item 6a writes "(ADR-006 makes it the one protocol admission profile)" with no prefix, but QSL has no ADR-006. The decision is QSpec ADR-006, as the Dependencies section says. Item 13 cites FR-103 but the frontmatter has no relationship to FR-103. QSpec ADR-006 is also absent from the relationships, while QSpec FR-271 and FR-272 are listed and never cited in the body. | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:116-120,171-173 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | FR-361 item 12 and AC-12 now say that in a non-selecting unit, FR-110's per-alias layer rule does not report an item-5 construct. FR-110's layer rule (138-143) still says S3 SHALL refuse every form outside the layer's set, naming the selected layer, and it has no exception or reference to FR-361. Read alone, the two SHALLs contradict each other. Whoever builds the layer rule (QSL-515) will emit the refusal that FR-361 suppresses. FR-110 needs one sentence that defers to FR-361 item 12 for the item-5 families in a unit that does not select the model profile. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:138-143 |
| FND-006 | low | FR-308's bullet still cites "FR-361 item 6a". The fix round renumbered FR-361, and the protocol, memory and temporal item is now item 8. There is no item 6a, so the cross-reference is dangling. | spec/functional/FR-308-build-the-complete-protocol-forms-at-s2.md:82-85 |

## Dispositions

Round 1, reviewed at 485c2fad7374250b5ceb6a95cf7a4c8c4031ba9b.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e01e00c9f |
| FND-002 | fixed | e01e00c9f |
| FND-003 | still-open | The table was reordered to four rows (operation-body, abstraction, dispatch-call, lookup). Item 5 now says "QSpec's shared grammar names the first, fourth and fifth rows, and QSpec FR-450-AC-3 names the second". There is no fifth row, and the shared grammar's forms are rows 1, 3 and 4. It should say "first, third and fourth". |
| FND-004 | fixed | e01e00c9f |

Round 2, reviewed at f7b6bb3aaad45cdcddf2f91e081a1eb1fe438ac2.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-003 | fixed | f7b6bb3aa |
| FND-005 | fixed | f7b6bb3aa |
| FND-006 | fixed | f7b6bb3aa |
