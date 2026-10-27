---
id: SR-60310
title: "Failure-domain review of the QSL-1119 weak-access item clarification"
type: SpecReview
analysis: failure-domain
scope: "FR-220, FR-224, TC-669 (QSL-1119)"
review_set: subset
---

## Summary

The new rule only promotes an access when an enabled non-atomic shared application occurs in a reachable state. It does not treat guard-agnostic potential footprints or initial-state absence as proof of no application.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | low | No additional failure-domain issues found in the scoped change. | - |
