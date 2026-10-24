---
id: SR-9016
title: "Traceability and gap review — QSL-750 model-profile selection and gating (PR #813)"
type: SpecReview
analysis: gap-analysis
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

**CHANGES REQUESTED.** Items map to ACs, but the TCs under-pin AC-2 and AC-4. Clean bindings: TC-1012 to AC-1, TC-1014 to AC-3, TC-1016 to AC-5, and TC-1017 to AC-7, AC-8, AC-9 and AC-10 (steps 3 to 6). The expected codes and causes match the native-diagnostics catalog (`unsupported_construct` with `declaration-form` or `expression-form`).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-1015 has no step for `Population<T>`, `allInstances` or field navigation, although AC-4 lists all three. Its expected result accepts any other refusal that "names no missing model profile". FR-110's layer rule refuses navigation, `Population<T>`, `reaches` and state-graph clauses under `quire.value.complete/v1` (state core refuses them) and names only the selected layer. So a mislabelled gate can pass this TC vacuously. Pin the expected outcome per step, for example by using a selecting layer that admits the form, so the TC can fail. | spec/test-cases/TC-1015-forms-qspec-states-no-model-gate-stay-ungated.md:19-31 |
| FND-002 | medium | TC-1013 does not specify its unit. `x.op(1)` needs `x` of a model type, and `lookup<T>(p, r)` needs a population binding `p` of a model object type, so both units must hold a `model` declaration. Under the value profile that declaration is refused too (family `model-import`), and AC-6 requires every construct to be reported. TC-1013's expected result 1 and 2, and TC-1014's expected result 1 (TC-811 step 1's relation binds `ConfigVersion` from a model), list only the one family refusal. TC-1012 does include the `model-import` refusal. TC-1014's description also says "TC-811 steps 2 and 3 give the declaration and the malformed forms", while its procedure uses step 1 for the declaration, and TC-811 gives the malformed forms in steps 3 and 4. | spec/test-cases/TC-1013-a-dispatched-call-and-lookup-need-the-model-profile.md:17-25 |
| FND-003 | low | AC-2 says the model-profile units "pass the refusal check", and TC-1013 step 3 expects "Neither unit gives this refusal". No positive outcome is stated (compiles, or the dispatched call resolves). Contrast AC-1 and AC-3, which state compile-and-assert outcomes. | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:184 |
| FND-004 | low | TC-1012 to TC-1016 each trace one AC, but TC-1017 traces five (AC-6 to AC-10) through six steps with no map from step to AC. Item 8, that S2 builds the form whether or not the profile is selected, has no AC, so no test checks that S2 builds a form in a non-selecting unit. Item 14 (fixture migration) also has no AC. | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:142-146 |

## Dispositions

Round 1, reviewed at 485c2fad7374250b5ceb6a95cf7a4c8c4031ba9b.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e01e00c9f |
| FND-002 | fixed | e01e00c9f |
| FND-003 | fixed | e01e00c9f |
| FND-004 | fixed | e01e00c9f |
