---
id: SR-60017
title: "Code review — QSL-987 state-graph claim emission"
type: SpecReview
analysis: code-review
scope: "qsl-semantics/src/check/check.rs, qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/lowering/state_graph.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/refusal.rs, qsl-semantics/src/check/state_graph_claim.rs, qsl-semantics/tests/it/state_graph_claims.rs, qsl-package/src/emit/tests.rs, qsl-package/src/emit/tests/state_graph.rs; FR-165-AC-1"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-165", type: references }
---

## Summary

Reviewed QSL-987's state-graph S4 lowering and emission against FR-165-AC-1, including the checked claim identity, requirement record, `over` parameter and emitted source locations. The review found one source-location defect for an omitted `from`; it is fixed and covered by a parsed-source round-trip test.

## Verdict

**CONDITIONAL** — the medium finding was fixed and regression-tested in this review; no open findings remain.

## Findings

| ID      | Severity | Summary                                                                                     | Refs                                                                 |
| ------- | -------- | ------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| FND-001 | medium   | An omitted `always possible` `from` synthesized `true` at an origin without a source span, causing `emit_checked` to refuse the package as `UnlocatedOccurrence`. | qsl-semantics/src/check/mod.rs:1688; qsl-package/src/emit/tests/state_graph.rs:221 |

## Dispositions

| ID      | Outcome | Reason |
| ------- | ------- | ------ |
| FND-001 | fixed   | Map the implicit predicate occurrence to its claim span; a parsed-source test verifies emission and QSL reader admission for claims with and without `from`. |

## Coverage

- Scoped criterion check: `FR-165-AC-1` is trace-tagged by `a_checked_claim_has_a_state_graph_node_and_requirement_record` and the two QSL-987 emitter tests in the files named above.
- `cargo test -p qsl-package --all-features`: passed (294 unit tests, 3 integration tests, 12 doctests).
- `cargo test -p qsl-semantics --all-features`: passed (774 unit tests, 1,236 integration tests, 30 doctests; 1 ignored unit test).
- `cargo clippy -p qsl-semantics -p qsl-package --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- A repository-wide Quoin matrix was not run: the assigned review scope is limited to touched files, and the available matrix command accepts repository/module scope rather than a changed-file list. The trace tags for the touched `FR-165-AC-1` tests were checked directly.
- Semantic review: skipped; it was not opted into.
