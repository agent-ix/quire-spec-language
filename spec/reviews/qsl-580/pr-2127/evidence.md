---
id: SR-2026110215398
title: "QSL-580 PR 2127 evidence review"
type: SpecReview
analysis: evidence
scope: "FR-230-AC-56 and FR-393-AC-9 with TC-685 and TC-1078"
review_set: subset
---

## Summary

Both new criteria use `Test` and point to concrete procedures that inspect parsed typed forms, emitted binders/bounds, empty-collection behavior, and adverse refusals. The deterministic `quoin advise --repo . --json` and `quire matrix --scope . --format tsv` computations did not return within bounded 45-second and 35-second runs, so no catalog recommendation or matrix result is claimed; the new criteria do link to detailed TC-685/TC-1078 procedures and manual evidence alignment found no method mismatch. Source-level test bindings remain to be added with implementation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
