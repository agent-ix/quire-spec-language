---
id: SR-2139001
title: "Code review (rust-review lane) - QSL-1414 compensation clock rows, PR 2139"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-package/src/emit.rs; qsl-package/src/emit/tests/protocol_requirements/fixture_w.rs; qsl-semantics/src/check/protocol_obligation.rs"
review_set: subset
---

## Summary

Ticket: QSL-1414. PR 2139 head cb6da7049109fa20eedd5a3b7085a54fdd6c87e5, merge-base 6cb54a816ef08168ec554542c474a1d720bc8105 (origin/main has since moved to 347e9769e with two unrelated commits).
`temporal_binding_rows` now also emits one row per `compensate` node keyed by the node's own id, named by the template clock text, in the one sorted array. `CheckedCompensation` gains `span` and `clock`. The four refusal paths (missing, duplicate, surplus, profile mismatch) are reached for compensation windows and each is asserted with the exact span. No `#[allow]`, no pins, no ignored tests. `cargo test -p qsl-package --lib tc134`: 6 passed.
Control claim verified: `a_temporal_follow_up_carries_its_extent_bounds_at_its_occurrence` fails on this branch with `MissingTemporalClockSelection` at span 172..272, which is its `temporal C` clause; the fixture has no protocol or compensate, so the change cannot cause it (that test calls `package()` with no selections).
IR-1262 pin test `tc134_strict_reader_refuses_compensation_rows_until_ir_1262` is intentional: its doc comment names IR-1262 and says the refusal becomes an admission and the test is restated; the fall-through panic message says the same. It asserts code, cause and the exact row locus, so it is not vacuous.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | low | The "template has no written node" skip and the "no temporal profile" skip are untested; a surplus selection naming such a template is the only protection and is never exercised | qsl-package/src/emit.rs:5604-5612 |
| FND-002 | low | Compensation node lookup scans every node per template and rebuilds the qualified-name path already built at emit.rs:4646 | qsl-package/src/emit.rs:5618-5634 |

## Dispositions

| FND | outcome | reason |
|-----|---------|--------|
| FND-001 | fixed | Round 1, head a9f4db7dc: tc134_template_without_a_clock_profile_writes_no_row and tc134_template_without_a_written_node_writes_no_row exercise both skips and the surplus refusal for each. |
| FND-002 | fixed | Round 1, head a9f4db7dc: compensation nodes are indexed once into a HashMap keyed by qualified name; per-template lookup is a map get. |
