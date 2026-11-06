---
id: SR-20261105101025
title: "Code review with Rust lane for QSL-1743 PR #2415"
type: SpecReview
analysis: code-review
scope: "qsl-package/src/introduced_binders.rs, FR-390-AC-68, TC-1071 row 26"
review_set: subset
---
## Summary

Rust implementation, test helper, and test/AC alignment examined; no defect found. Existing gate evidence in PR body was accepted; no Cargo build was started.

## Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| `qsl-package/src/introduced_binders.rs` | examined | The `sources` test helper sorts then deduplicates parameter names; the added test builds one unread collection binder listed as an application dependency and another nested case, then asserts only `s` is a source. |
| `FR-390-AC-68` | examined | An unread binder is introduced and is no source even when its application lists the node among its dependencies. |
| `TC-1071 row 26` | examined | Includes unread binder and case-arm binder term walks; verifies only target, activated, recovered are sources and extra introduced-binder keys are refused. |
| `tests/it/bare_recovery_binder.rs` | context_only | Existing checked-package integration coverage exercises unread binder source admission and nested query behavior. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
