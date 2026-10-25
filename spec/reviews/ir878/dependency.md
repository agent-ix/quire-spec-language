---
id: SR-17023
title: "IR-878 dependency review"
type: SpecReview
analysis: dependency
scope: "spec/functional/FR-485-supplied-admission-and-conversion-owner-allocation.md"
review_set: all
---

## Summary

Checked whether each allocation points to its owning requirement. FR-485 names QSV FR-109 and FR-122, exact FR-372 and FR-380, QSpec FR-323, and IR-878.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No missing implementation-owner dependency found for the allocated behaviors. QSpec FR-323 is referenced in prose because its cross-repository identity is not represented as a local artifact. | FR-485 |
