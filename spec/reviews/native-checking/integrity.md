---
id: SR-068
title: "Native model and checker integrity review"
type: SpecReview
analysis: integrity
scope: "FR-015/016, FR-006 judgments, docs/native-model-checking.md, IT-005, TC-025–029/040–053 and TM-003"
review_set: all
evaluated_revision: "ceccabb564b61742597ad356eb1019ab0c8d1544"
review_date: "2026-09-08"
---

## Summary

Source roles, exact type constraints, proof discharge and runtime assumptions have consistent boundaries. The inherited FR-006 judgments retain their reference and operation meaning.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Resolved in e1ae696: the legacy profile is named native-formal-environment/1 exactly, preserving the existing import domain rather than inventing a second spelling. | FR-015; FR-013; TC-042; TC-044 |
| FND-002 | low | No remaining contradiction between backward contextual inference and forward evaluation order: type constraints can flow to an initializer, but later guard facts cannot justify its execution. | FR-016; TC-048; TC-050 |

## Traceability

| User value | Requirement | Stakeholder | Verification |
| --- | --- | --- | --- |
| US-002 | FR-015 | StR-001 | TC-040–045; IT-005 |
| US-002 | FR-016, retaining FR-006 | StR-001 | TC-025–029; TC-046–053; IT-005 |

NFR-005 governs every new executable path, including the fixture producer and independent oracle. FR-015 has one atomic admission result with six independently observable criteria; FR-016 has one checked-package result with nine separately testable obligations. Every shall statement names its subject and observable response.

Multi-source ambiguity has an explicit refusal policy. No pagination, authenticated service, executable scaffolder or external CLI is part of the production API. The exact IR dependency already supplies validated declarations and arithmetic checking; there is no unimplemented service hidden behind a fallback.

Scalar roles distinguish nominal types even with identical representations. Text maxima and integer policies are explicit. Records, identity-bearing objects and references remain distinct. Collection order/duplicates are an explicit native-profile interpretation. Artifact byte identity includes provenance and roles and is not IR semantic identity. Conditional observation identity and captured aliases prevent pre from retagging post values. See failure-domain.md for proof/purity hazards and their resolved controls.

## Verdict and provenance

PASS for implementation of this specified scope. Agent A applied the actual
QUOIN base and all seven analysis skills serially, following the owner's
selected all review set. No subagents or builds were started for this review.
No applicable required AssuranceProfile was found. Catalog schema and existing
public interfaces were inspected. Implementation/test completion is not claimed.
The owner-declined optional gap-analysis semantic comparison remains excluded.

