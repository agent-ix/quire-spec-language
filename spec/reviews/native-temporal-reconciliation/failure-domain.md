---
id: SR-380
title: "Failure-domain review of reconciled native temporal subsystem"
type: SpecReview
analysis: failure-domain
scope: "FR-043, FR-044, FR-045, NFR-008, TC-122-125, TM-008, docs/native-temporal-evaluation.md"
review_set: all
---
## Summary

The reviewed scope now distinguishes false guards from missing guards,
identical from conflicting redelivery, unavailable from pending truth, and an
affected resource stop from an already-settled sibling. Native evaluator inputs
remain deliberately fail-closed at unauthenticated profile/clock boundaries.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `/1` carries neither a definition digest nor clock parameters, so it cannot authenticate temporal evaluation or correspondence. The limitation is explicit and delegated to A's separately scoped FR-042 `/2` contract; this subsystem must not bridge around it. | FR-043, TM-008, TC-122 |
