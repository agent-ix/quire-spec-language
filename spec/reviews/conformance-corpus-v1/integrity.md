---
id: SR-820
title: "Integrity review of QSL conformance corpus bindings"
type: SpecReview
analysis: integrity
scope: "FR-350/351/353"
review_set: subset
---
# Integrity review of QSL conformance corpus bindings

## Summary

PASS. Scope selection, vector applicability, expected dispositions and
preflight refusal now reference the same versioned wire fields.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved consistency issue found: a selected capability comes from `coverage.primary`, and an executable vector must name that capability and include its subject consumer type in vector applicability. | FR-350; FR-351; FR-353 |
