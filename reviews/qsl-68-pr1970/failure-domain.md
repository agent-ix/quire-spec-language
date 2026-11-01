---
id: SR-2029
title: "Failure-domain review of QSL-68 identity and provenance contracts"
type: SpecReview
analysis: failure-domain
scope: "FR-081, TC-1023, QSpec FR-150"
review_set: all
---

## Summary

The revised contract separates stable structural identity from ancestry and phase-4 provenance. The reviewed graph cases retain direct-parent facts and test view changes; no additional failure-domain gap was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found (placeholder). | - |
## Scope

- **FR-081** (examined; `spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md`): A derived declaration identity is the structural pair of its owner effective type and original declaration key; ancestry remains in derivation facts.
