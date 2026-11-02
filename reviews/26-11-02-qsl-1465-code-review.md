---
id: SR-202611020541
title: "Code review of QSL-1465 PR #2111: linear complete NodeKey debug output"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@e7562c9c0; PR #2111: qsl-foundation/src/source/node_table.rs, qsl-package/src/emit/tests/admission_corpus.rs, qsl-analyze/tests/it/schedulability_routing.rs"
review_set: subset
---

## Summary

Reviewed PR #2111 against current `main`, including the Rust review lane. The formatter's scoped table emits complete structural identities once and cleans up after formatting; no-std and Kani builds retain complete-key output. The admission corpus accounts for the newly emitted and not-emitted IR forms, and the synthetic schedulability fixture supplies its required identity fields.

Focused evidence on the reviewed head: `cargo check --locked -p qsl-analyze --all-targets`; the three semantics NodeKey-debug tests; both checked-package and indexed-I2 scaling tests; the FR-375-AC-12 internal-list and emitted-family admission tests; `cargo fmt --all -- --check`; `cargo check --locked -p qsl-foundation --no-default-features`; focused Clippy for qsl-foundation, qsl-semantics and qsl-package with `-D warnings`; and `cargo deny check`.

## Verdict

**PASS** — no findings in the changed files. The focused regression suite and review checks passed. `cargo deny` reports existing unapproved git-source warnings; its advisories, bans, licenses and sources checks pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
