---
id: SR-2027
title: "Code and Rust review of QSL-68 identity evidence changes"
type: SpecReview
analysis: code-review
scope: "QSL PR #1970: qsl-semantics/tests/it/main.rs, model_identity_contracts.rs, model_identity_links.rs, model_normalization.rs, tc235_business_meanings.rs; FR-081 and TC-1023 spec edits"
review_set: subset
---

## Summary

The changed integration tests exercise real QSL model intake and normalization, and the focused identity suites and Clippy pass. One newly added test helper passes vacuously during ordinary test runs and has no trace tag.

## Verdict

**CONDITIONAL** — resolve the helper test's no-op/untraced behavior before merging.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `tc217_child_reports_normalization_output` is registered as a normal Rust test but does nothing unless `QSL_TC217_CHILD` is set, so the ordinary suite counts a successful test without an assertion; it also lacks a local trace tag. Keep it out of the ordinary suite or fold the child mode into the tagged TC-217 test so every executed test has meaningful coverage. | qsl-semantics/tests/it/model_identity_contracts.rs:103-108 |

## Rust Review

No additional Rust correctness, safety, API, or style finding was identified in the changed code. The PR changes test code only and does not change a CI workflow or production implementation. `cargo fmt --all -- --check`, `cargo clippy -p qsl-semantics --all-targets --all-features -- -D warnings`, the three identity test groups, and the affected TC-235 helper module passed. A broad filter also selected the unchanged full-size test; its child failed because the sandbox denied `sysctl kern.clockrate`, while the child test body itself reported success. The aggregate `make ci` gate was left to the code lead's final merge checkpoint.

## Gap Analysis

Not applicable: no production implementation changed. The FR-081 test-matrix trace omission is recorded in the separate base SpecReview.
## Scope

- **model_identity_contracts::tc217_child_reports_normalization_output** (examined; `qsl-semantics/tests/it/model_identity_contracts.rs`): The helper reports child-process normalization output when QSL_TC217_CHILD is set.
