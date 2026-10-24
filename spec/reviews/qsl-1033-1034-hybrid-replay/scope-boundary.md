---
id: SR-1306
title: "Scope-boundary analysis of hybrid replay requirements"
type: SpecReview
analysis: scope-boundary
scope: "FR-450, FR-290-AC-19, ADR-013-O-16/O-24"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-450
    type: reviews
---

## Summary

QSL owns process-result decoding, typed hybrid replay and terminal settlement. QSpec FR-193 owns the source wire spelling for provider traces; FR-450 defines QSL's index mapping, exact replay disposition and retained parity payload.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No responsibility allocation gap found. | FR-450; FR-290; ADR-013 |
