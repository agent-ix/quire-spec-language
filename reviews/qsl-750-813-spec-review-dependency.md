---
id: SR-9017
title: "Dependency review — QSL-750 model-profile selection and gating (PR #813)"
type: SpecReview
analysis: dependency
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

**CHANGES REQUESTED.** The code slice depends on unbuilt FR-110 layer selection (QSL-515) with no edge in Linear. All other dependencies (FR-056, FR-102, FR-302, FR-304, FR-308, FR-309, QSpec FR-001, FR-450 and FR-453) are stated and exist.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-361's code slice needs FR-110's layer selection. Items 2 and 4 and AC-8 require E3 to resolve `quire.model.complete/v1` as a layer, compute its `requires` closure and emit the closure's lock rows. FR-110 says "Layer selection (AC-7, AC-8) and AC-1's layer-closure lock rows are not yet implemented". The code agrees: `resolve_profiles` in qsl-semantics/src/check/profile.rs accepts only the `root` row. That work is QSL-515 (HL2, FR-110 AC-1/7/8), which is in Backlog under the Layer 4 milestone. QSL-750 is in the Layer 2 milestone, and Linear has no blocks or blocked-by edge between them. The planner's note that "layer selection for all five is implemented" does not match the spec text or the code. FR-361 should say whether QSL-750 builds the model-layer subset itself or depends on QSL-515, and Linear should carry the matching edge. | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:196-200 |

## Dispositions

Round 1, reviewed at 485c2fad7374250b5ceb6a95cf7a4c8c4031ba9b.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e01e00c9f |
