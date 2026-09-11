---
id: SR-279
title: "Dependency review of the runtime input schema"
type: SpecReview
analysis: dependency
scope: "Task-036 prerequisites at f4679ef"
review_set: all
---
## Summary

The schema depends on already implemented runtime shapes and readers. Publishing
it does not gate native execution or require work in B/C repositories.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Ordering

| Requirement | Class | Prerequisite for this slice |
| --- | --- | --- |
| NFR-006 | Enablement | Existing construction limits |
| FR-018 | Enablement | Existing flat draft and wire shapes |
| FR-024 | Feature | Existing reader precedes AC-5 companion schema |

The local order is FR-018 with NFR-006 → FR-024 reader → FR-024-AC-5.
There is no cycle. Existing jsonschema 0.17.1 supplies Draft 2020-12 validation
in Rust tests; Cargo.toml/lock are unchanged. Shared catalog adoption (#28),
IR resource classification (#27) and native-profile reconciliation (#30) remain
separate work, with no invented prerequisite for this schema.
