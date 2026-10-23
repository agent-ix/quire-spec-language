---
id: SR-16324
title: "Gap analysis of quire-spec-language PR #1394 hybrid emission"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1017 PR #1394; FR-447-AC-5, AC-12 through AC-16; qsl-package/src/emit/tests/hybrid.rs and checked_v2/tests.rs; qsl-semantics/src/check/lowering/hybrid.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: reviews
---
# Gap analysis of PR #1394

## Summary

Ticket: QSL-1017. Traced the FR-447 emission and read-back criteria to tests and Rust code. The tagged tests cover the named example structures and read-back, but two observable obligations lack automated assertions.

## Verdict

**CONDITIONAL.** Both findings are test gaps; the code-review artifact separately reports the fairness wire mismatch. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-12 requires the emitted nodes to equal QSpec FR-481 `restored_nodes` up to the source map. The tagged test inspects selected members and admission, but never loads `hybrid-node-vectors.json` or compares the complete emitted node set. An omitted or miskeyed node outside the asserted subset passes. Add a `QSPEC_DIR` conformance assertion over the full node set, following existing QSpec fixture tests. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:606; qsl-package/src/emit/tests/hybrid.rs:394 |
| FND-002 | medium | The emitter's new `fair S` single-target branch has no test. The tagged AC-12 fixture uses only `fair Moving::Stopped`, so the QSpec wire mismatch in FND-001 of the code review passes every test in this PR. Add distinct `fair S` and `fair S::T` cases and assert each member's declaration and name. | qsl-semantics/src/check/lowering/hybrid.rs:433; qsl-package/src/emit/tests/hybrid.rs:61,457 |

## Coverage

Examined FR-447-AC-5, AC-12, AC-13, AC-14, AC-15 and AC-16; the tagged tests in `qsl-package/src/emit/tests/hybrid.rs` and `qsl-package/src/checked_v2/tests.rs`; the emitter and I2 read-back. Existing QSpec fixture tests under `QSPEC_DIR` establish a pattern for the full vector assertion. The computed matrix was attempted but remained running for several minutes under concurrent repository reviews; this pass used a manual trace-tag and assertion audit. Plan completion: not assessed. Optional repository-wide semantic review was not run.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Commit `03ed8c629` adds `braking2_restored_nodes_match_qspec_vectors`, which loads QSpec's `hybrid-node-vectors.json` through `QSPEC_DIR`, compares the complete restored node ID set and each node's content, ignoring occurrences in the source map. |
| FND-002 | fixed | Commit `03ed8c629` adds `fairness_member_preserves_the_source_mode_or_target_form`, which writes both `fair Moving;` and `fair Moving::Stopped;` and asserts the different declaration nodes, names, package identities and read-back. |
