---
id: SR-1303
title: "Dependency analysis of hybrid replay requirements"
type: SpecReview
analysis: dependency
scope: "FR-450, FR-290-AC-19, ADR-013-O-16/O-24"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-450
    type: reviews
---

## Summary

FR-290 consumes FR-193's wire trace form and the checked hybrid model from FR-447; it delegates settlement to FR-450. FR-450 uses ADR-013's shared `DisagreementCause` ownership. These dependencies are acyclic and keep parser, replay and terminal ownership at their existing boundaries.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No dependency-cycle or ordering issue found. | FR-290; FR-450; ADR-013 |
