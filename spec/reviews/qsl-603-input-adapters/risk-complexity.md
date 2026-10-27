---
id: SR-60306
title: "Risk-complexity review of QSL conformance input adapters"
type: SpecReview
analysis: risk-complexity
scope: "FR-350-AC-8 through FR-350-AC-10"
review_set: all
---

## Summary

The main risk is translating untrusted corpus JSON into typed runtime and replay requests while preserving existing validation. The contract keeps that conversion at a narrow harness boundary and requires owner admission after conversion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The adapter has several identity-sensitive conversions, especially PopulationId name resolution and ReplayRequestWire byte provision. Mitigate with tests for every tag, unknown names, path confinement, duplicate/missing paths and digest mismatch before broad corpus execution. | FR-350-AC-8, FR-350-AC-9, FR-350-AC-10 |
