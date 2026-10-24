---
id: SR-16703
title: "Failure-domain analysis of the QSL ruling batch"
type: SpecReview
analysis: failure-domain
scope: "FR-125, FR-169, FR-183, FR-289, FR-290, ADR-023"
review_set: subset
---

## Summary

Checked trust boundaries, identity behavior, evaluation purity, and graph edge cases in the edited clauses. Absent-key now applies only to an expression that reads the bound parameter; overlapping populations retain the existing key identity and conflict checks. No additional failure-domain rule was missing in this scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found. | FR-125, FR-169 |
