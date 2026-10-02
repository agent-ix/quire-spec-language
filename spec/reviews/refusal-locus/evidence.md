---
id: SR-406
title: "Evidence review of protocol-role refusal loci"
type: SpecReview
analysis: evidence
scope: "FR-042-AC-8; TC-121; TM-003; issue #68"
review_set: subset
---

## Summary

The authored Test method is appropriate by explicit reviewer judgment: the exact public report,
typed cause, source index, and source slice are observable in one deterministic Rust integration
test. `quoin advise --repo . --json` was run but could not detect the installed Quire 0.31.0 CLI, so
no catalog recommendation is represented as an advisor verdict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Automatic method advice is unavailable because Quoin cannot detect the installed Quire 0.31.0 CLI; Test/Integration is retained as labeled reviewer judgment. | FR-042-AC-8; TC-121 |

The regression symbol `multi_unit_invalid_role_type_reports_the_role_in_its_own_source` carries
`TC-121` and `FR-042-AC-8`. It exercises actual native admission, requires `Invalid::Type`, resolves
the returned source index into the supplied inventory, and compares the selected source slice with
the exact role declaration. This evidence would fail for the observed stale-source defect and does
not approximate correctness from the error kind alone.
