---
id: SR-11077
title: "Code review of quire-spec-language PR #851: QSL inventory over one checked unit"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@301d558a627b02613e17900301758998600a9e20; src/checking/source_inventory.rs, tests/it/source_inventory.rs, Cargo.lock"
review_set: subset
---
# Code review of quire-spec-language PR #851

## Summary

Ticket: QSL-774. Code review with the rust-review lane folded in, scoped to
`git diff origin/main...HEAD` (3 files). The contract is Contract IR
`crates/quire-contract-model/src/source_obligation.rs` at f7dd6130 (the rev
Cargo.lock now resolves), read with `git show`, checkout untouched.

Examined:
- `Traversal::next_row` emits every requirement row, then every pending
  row, and only when the iterator is exhausted evaluates
  `stop_reason(types)`: `None` gives `Ok(None)`, else `Err(cause)`. There is
  no other return path, so `Complete` needs the full traversal of the named
  scope plus no exhaustion and no unfinished declaration. Rows emitted before
  the stop stay in the outcome (IR `from_enumerator` keeps them) (examined)
- IR `from_enumerator` issues `Complete` only on `Ok(None)`, checks
  owner/scope, each row's owner and source, the stop owner, and
  `observed_count == items.len()` (examined)
- `CheckedUnit::new` compares three independent labels: the selected
  `FormalSource`'s `RawSourceRef`, the type report's single namespace unit's
  `RawSourceRef`, and `CheckedGraph::source()`. `NotOneUnit` is checked
  first, so `stop_reason` can only name declarations of the one selected
  unit (examined)
- Naming: `UnitLabel`, `UnitLabelMismatch`, and the module/fn docs say
  "INTERIM, NOT A DIGEST-BOUND MATCH" and name the QSL-781 expiry (examined)
- Owner/scope/source passed to IR: owner `Qsl` and scope
  `QslCheckedRequirementsAndPendingObligations` are the enumerator's fixed
  identity; source is `FormalSource::identity()` held in the unit, not
  hard-coded (examined)
- Tests: the matched test compares the reported requirement keys and pending
  count against `graph.requirements()` and `types.declarations()` read
  directly, not through `Traversal`, so a skipped row fails. Exhausted test
  asserts dimension, limit, prior, requested, declaration and span against
  the native `Exhaustion`; unfinished test asserts the exact typed cause with
  the declaration handle (examined)
- Consumers: `git grep qsl_inventory|SourceInventoryEnvelope` finds no
  caller of the old three-argument form outside historical `reviews/` text
  (examined)
- Cargo.lock: one line, quire-contract-model c9bf2631 -> f7dd6130; IR's
  Cargo.toml files are unchanged across that range; QSL Cargo.toml unchanged
  (examined)
- rust-review lane: no `unwrap`/`expect`/`panic!` in `src`; no integer
  conversions; boxed iterator is bounded by retained records (examined)
- Evidence: clippy --all-targets --all-features log exits 0; 8 tests pass
  (logs qsl774-clippy.log, qsl774-test2.log). Full `make ci` not yet run on
  this head; the lead runs it (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS.** `Complete` is reachable only through IR's `from_enumerator` after
QSL's own traversal of the named scope ends with no stop cause. The label
match is named and documented as interim, not digest-bound.

Recorded, not reported:
- low: test `a_type_report_of_another_source_is_refused_with_both_labels`
  asserts `UnitLabelMismatch::Graph` (formal and report both "other"); the
  `TypeReport` variant is covered by `a_formal_source_that_is_not_the_reports_unit_is_refused`.
- low: `observed_count == items.len()` assertions are tautological; IR
  refuses a mismatch itself.
- low: the unfinished test does not assert retained rows; the exhausted test
  asserts only `items >= requirements`.
- low: a `Refused` declaration yields no stop cause (pre-#841 semantics; IR
  has no variant for it).
- low: tests were written first but never run red.
