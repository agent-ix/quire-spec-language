---
id: SR-20261104130237
title: "Scope-boundary review — QSL-1642 timed population scope"
type: SpecReview
scope: "FR-230 timed-model predicate checker boundary; FR-391 population positions; FR-480-AC-17"
analysis: scope-boundary
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: references
---

## Summary

The amendment assigns the new behavior to QSL's timed-model predicate checker and clearly excludes unit-level clauses and holds from the refusal. No external dependency or component responsibility is introduced.

## Verdict

**PASS** — the changed responsibility remains within the existing QSL S3 checker boundary.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No scope-boundary gap found in the scoped change (placeholder). | - |

## Review Notes

FR-480-AC-17 now explicitly separates model-qualified type resolution from named-population scope. The criterion-id collision is recorded in the base review and does not change which component owns this check.
