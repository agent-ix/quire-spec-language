---
id: SR-20261105101026
title: "Acceptance-criteria gap analysis for QSL-1743 PR #2415"
type: SpecReview
analysis: gap-analysis
scope: "FR-390-AC-68, TC-1071 row 26, qsl-package/src/introduced_binders.rs, tests/it/bare_recovery_binder.rs"
review_set: subset
---
## Summary

Manual trace of the changed FR-390-AC-68 behavior to tests found relevant focused unit and integration coverage. Repository-wide `quoin matrix` timed out after 30 seconds without output, so matrix-wide coverage remains unverified.

## Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| `FR-390-AC-68` | examined | An unread binder remains introduced and is not a source even when its node appears in application dependencies. |
| `TC-1071 row 26` | examined | The row includes an unread binder and case-arm walk, expects only the three outer sources, and refuses extra introduced-binder keys. |
| `introduced_binders::tests::an_unread_binder_listed_as_a_dependency_is_not_a_source` | examined | Direct walker coverage asserts the unread dependency binder is excluded both alone and nested. |
| `bare_recovery_binder::a_binder_the_body_never_reads_is_no_source` | examined | Checked-package integration test covers unread binder supply/source behavior. |
| `bare_recovery_binder::nested_query_binders_are_no_sources_and_the_predicate_evaluates` | context_only | Existing nested-query behavior contextualizes AC-68 coverage. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
