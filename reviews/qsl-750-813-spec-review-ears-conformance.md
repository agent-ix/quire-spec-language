---
id: SR-9015
title: "EARS conformance review — QSL-750 model-profile selection and gating (PR #813)"
type: SpecReview
analysis: ears-conformance
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

**PASS.** FR-361's statement is a pair of event-driven EARS requirements ("When a unit's header selects ..., QSL SHALL admit" and "When ... selects no header profile that resolves ..., QSL SHALL refuse ...") with an explicit subject and response. The FR-304, FR-308 and FR-309 edits keep their EARS form. quire's grammar check raises no warning on any file this PR changes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
