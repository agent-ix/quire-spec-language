---
id: SR-17208
title: "Code review — QSL-891 state-graph NoInstance settlement"
type: SpecReview
analysis: code-review
scope: "qsl-analyze/src/model_check/state_graph.rs, qsl-analyze/tests/it/model_instances.rs, qsl-analyze/tests/it/state_graph.rs, qsl-analyze/tests/it/state_graph_settle.rs, qsl-replay/src/model_check/graph_settle.rs, qsl-replay/src/model_check/state_graph.rs, FR-169, TC-594"
review_set: subset
review_date: "2026-10-25"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: references
---

## Summary

Reviewed QSL-891's FR-169 NoInstance path across the state-graph run, settlement map, specification, and tagged integration tests. No implementation or traceability defects were found in the scoped change.

## Verdict

**CONDITIONAL** — the scoped Rust and state-graph gates pass; `cargo deny check` remains red on the unchanged transitive AGPL dependency allowlist.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Rust review

The change is additive and exhaustively maps `StateGraphOutcome::NoInstance` to the existing `InconclusiveCause::NoInstance` row. Empty-instancing items keep the marker through no-initial-state, pre-exploration stop, and normal completion paths. If no other item or deadlock-freedom request needs graph work, the engine returns before sampling or exploring. The settlement branch runs neither certificate checking nor evidence replay. The new test uses an actual checked claim and verifies the resulting record and zero replay calls. The existing instancing tests were updated to consume the item-level marker.

No applicable AssuranceProfile was found under `spec/`.

## Code-test alignment

FR-169-AC-16 is tagged on `a_claim_with_no_instances_settles_no_instance_without_exploration`. It asserts the NoInstance marker, empty graph, inconclusive terminal value, label, basis, category, execution, method metadata, and no evidence replay. Existing cases retain coverage for `NoInitialState` and populated instances.

## Scoped gap review

The direct criterion-to-test-to-code check covered only FR-169-AC-16 and the touched files; no repository-wide matrix or reverse-gap sweep was run, per the assigned scope. The separate FR-167-AC-4 provider-manifest contract is not included in this implementation slice and is routed as a QSpec wire-contract change.

## Gates

- `cargo fmt --all -- --check` — passed.
- `cargo clippy -p qsl-analyze -p qsl-replay --all-targets --all-features -- -D warnings` — passed.
- `cargo test -p qsl-analyze --test it state_graph` — passed, 51 tests.
- `quire validate` — the changed FR-169 document and four scoped SpecReview documents validated; only existing module-registration notices remained.
- `cargo deny check` — failed on the transitive `agent-ix-extraction-frontend` dependency's AGPL-3.0-or-later license not being in the allowlist; `Cargo.lock` and dependency declarations are unchanged by this patch.
