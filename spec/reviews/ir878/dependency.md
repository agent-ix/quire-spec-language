---
id: SR-17043
title: "IR-878 dependency review"
type: SpecReview
analysis: dependency
scope: "spec/functional/FR-486-supplied-admission-and-conversion-owner-allocation.md"
review_set: all
---

## Summary

Checked whether each allocation points to its owning requirement. FR-486 names QSV FR-109 and FR-122, exact FR-372 and FR-380, QSpec FR-323, and IR-878.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No missing implementation-owner dependency found for the allocated behaviors. QSpec FR-323 is represented by a `depends_on` edge to ix://agent-ix/quire-specification/FR-323, which resolves, and by a linked Dependencies bullet. | FR-486 |
