---
id: SR-1574001
title: "code-review and rust-review of QSL-1574 enum declaration members and order (quire-spec-language PR 2231)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; Cargo.lock; qsl-foundation/src/diagnostic.rs; tests/it/text_enum_identity.rs; qsl-replay/src/outcome.rs; qsl-replay/src/spine/clause.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: references
---

# Code review of QSL-1574 enum declaration members and order (quire-spec-language PR 2231)

## Summary

Ticket: QSL-1574. Head a46867d0eaf45ac3c36bcbbb0df761486423cd2a, merge base 032fc1a20; trial merge on origin/main a5939662a is clean and leaves Cargo.lock at the same two-line change. Rust lane (rust-review) folded into this file.
Gates (QSPEC_DIR=/Users/peter/dev/quire-specification at cceef5918, via qsl-slot): fmt --check, workspace check --locked --all-targets, clippy -D warnings (default and --no-default-features), arch-lint qualified-core and api-surface --qsl --qsl-only all exit 0. Cargo.lock moves only the quire-semantic-value (ab81a09c) and quire-exact (d8e64115) pair; Cargo.toml is untouched, no rev or pin added. t07b is non-vacuous: with the old lock (QSV d5e28263, exact a3052598) it fails at tests/it/text_enum_identity.rs:516 with left Ok(Completed(true)), right Err(IllTyped DistinctEnumDeclarations); on the new lock it passes. It builds its declarations through EnumDeclarationPreimage and AdmittedEnumDeclaration, so it runs the QSL admission path. The qsl-foundation restatements match FR-285's ten-row table and ADR-013 O-16 and weaken nothing. Root `it` suite: the same nine tests fail on the PR and on clean origin/main a5939662a (869 passed on main, 870 on the PR). qsl-foundation, qsl-semantics and qsl-replay unit and integration suites fail the same tests on main and on the PR except one new failure (FND-001).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The PR turns qsl-replay's `outcome::tests::the_document_category_is_the_most_severe_items` red: it asserts `monitored.category().trace_exit_code() == 0` for an Inconclusive item, and the bumped quire-semantic-value defines `trace_exit_code` as `exit_code`, so it is now 22 (left 22, right 0). The test passes on main. The PR restated the qsl-foundation twin of this assertion but not this one. The old special case existed because a pending trace clause was folded into Inconclusive; QSL production still folds it there (FND-002), so a monitor document whose only item is a pending clause now exits 22 where FR-285-AC-9 requires 0. | qsl-replay/src/outcome.rs:2742; qsl-replay/src/outcome.rs:1215 | implementation-bug-despite-evidence |
| FND-002 | medium | QSL production keeps the interim mapping that FR-285 retires once the catalog holds the categories: `TemporalCategory`, `shared_category` (Pending to Inconclusive, Indeterminate to Undefined, so exit 10 for an indeterminate clause where FR-285-AC-7 requires 11) and `label_and_exit` with local "pending"/0 and "indeterminate"/11 literals, plus `ClauseDisposition::category`, which feeds `Operation::Monitor`'s `trace_exit_code`. Category::Pending and Category::Indeterminate now exist, so FR-285-AC-11 says the catalog's label and exit are read and QSL holds no local mapping. A missed consumer of the new variants; a pending or indeterminate clause reaches the monitor exit through the wrong category. | qsl-replay/src/spine/clause.rs:339-359; qsl-replay/src/spine/clause.rs:517-577; qsl-replay/src/outcome.rs:1209-1219 | missing-requirement |
| FND-003 | low | Stale comments after the restatement: `tc_769_exit_function_maps_every_table_row` has a doc comment reading "each of the nine table rows" over a ten-row assertion, and `OutcomeDocument::settled` documents the order "30, 20, 21, 22, 10, 0" while FR-285 states 30, 20, 21, 22, 10, 11, 0. | qsl-foundation/src/diagnostic.rs:960-963; qsl-replay/src/outcome.rs:1209-1212 | implementation-bug-despite-evidence |

## Verdict

Not mergeable as is: FND-001 is a regression the PR introduces (a previously green qsl-replay test is red and the monitor exit for a pending clause changes from 0 to 22). Fix inside this PR: carry Pending and Indeterminate through QSL's temporal and monitor path as the catalog categories (FND-002), or at minimum restate the qsl-replay assertion to what FR-285 requires and route pending clauses to Category::Pending. The test change and Cargo.lock bump are otherwise correct.
