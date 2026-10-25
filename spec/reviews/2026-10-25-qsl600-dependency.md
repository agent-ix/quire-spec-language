---
id: SR-60003
title: "Dependency review of QSL-600 inspection entry budget"
type: SpecReview
analysis: dependency
scope: "FR-255, FR-297, TC-1161, TC-1168"
review_set: subset
---

## Summary

Reviewed the dependency direction for the new inspect setting. FR-255 owns the published setting and default; FR-297 consumes them for typed-view construction and already depends on the lifecycle limits contract.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No new dependency gap or cycle found. FR-255 depends on the inspect behavior that defines the counter, while FR-297 references FR-255 for its setting name and default. | FR-255, FR-277, FR-297 |
