---
id: SR-17017
title: "Integrity analysis of QSL-1054 guard provenance"
type: SpecReview
analysis: integrity
scope: "spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md; spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md; spec/test-cases/TC-520-model-behaviours-read-as-temporal-traces.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-361
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-520
    type: reviews
---

## Summary

Integrity review found the provenance rule consistent with FR-125's guard and absent-key behavior, and the FR-361 exception constrained to model-profile selection and body admission. FR-125-AC-17 and FR-361-AC-16 have independent observable outcomes; TC-520's scope and procedure cover both.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved integrity finding remains for the delta. | FR-125-AC-17; FR-361-AC-16; TC-520 |
