---
id: SR-20111
title: "Failure-domain review of QSL-1105 and QSL-1107"
type: SpecReview
analysis: failure-domain
scope: "FR-401 and FR-485 changes for QSL-1105/QSL-1107"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-485
    type: reviews
---

## Summary

The requirements identify the callable acquisition and declaring package as the nominal-data identity, reject ambiguous ownership, and separate a membership `false`, each QSV failure class, cancellation, and an internal body fault. The cancellation cause is snapshotted against the same handle at conversion time; body-side failures explicitly have no expression location and do not add a meter charge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No additional failure-domain defect found in the changed requirements. | FR-401, FR-485 |
