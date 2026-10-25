---
id: SR-17019
title: "EARS analysis of QSL-1054 guard provenance"
type: SpecReview
analysis: ears-conformance
scope: "spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md (guard-provenance behavior); spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md (derived helper body family)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-361
    type: reviews
---

## Summary

The changed normative statements name QSL and state concrete behavior for provenance forwarding, lookup exemption, absent-key failure, model-profile-gated helper body family derivation, and retained assembly refusals. Targeted validation found no EARS grammar warnings in the changed requirement and test-case artifacts.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved EARS-conformance finding remains for the delta. | FR-125; FR-361 |
