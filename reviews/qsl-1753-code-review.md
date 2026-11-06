---
id: SR-20261105101049
title: "Rust code review of QSL-1753 PR #2434"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2434 diff against main: Cargo.lock, qsl-package/src/emit.rs, qsl-package/src/emit/tests/hybrid.rs, qsl-package/src/reader_protocol_projection.rs, tests/it/bare_recovery_binder.rs"
review_set: subset
evaluated_revision: "d9a27c3841ae7962c90b3eccbb16c6b0b7a45d43"
review_date: "2026-11-06"
---

## Summary

The Rust changes update the quire-contract-model lock entry, adapt a literal fixture to IR's empty-dependency rule, and add the positive case-arm acceptance test. The test currently claims source/bare route value and charge parity but does not evaluate the bare slot or compare charges; this also leaves the new acceptance claim unverified.

## Verdict

**FAIL for the stated parity acceptance claim.** The focused test source compiles the authentic case-arm unit and admits/selects its bare slot, but only the source route is evaluated. The existing QSL-1767 issue describes the bare union-construction execution gap. `cargo deny check` completed with advisories, bans, licenses, and sources OK; `cargo fmt --check` completed without diagnostics. The PR gate is green, but that does not cover the unasserted bare-route behavior.

No applicable AssuranceProfile was found under `spec/assurance` or elsewhere under `spec/`. No workflow or manifest changes were present. The changed `node_value_type` fallback is restricted to collection-bounds forms and delegates to the existing structural type calculation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The case-arm test says source and bytes routes agree on values and ordered charges, but only evaluates the source slot; the bare slot is selected and its source list compared, with no bare evaluation or charge-log assertion. | tests/it/bare_recovery_binder.rs:539-542; FR-390-AC-68; QSL-1767 |

## Coverage

Reviewed all changed Rust and lock files, the local Rust conventions, changed call path, test fixture, and affected FR/TC text. The focused test implementation was inspected directly. The parent reports the focused test/module and tiered gates green on this head; PR CI gate is green. `cargo deny check` and `cargo fmt --check` were run during this review. No code or test files were modified by the reviewer.

## Findings detail

The tagged test selects `bare_slot` and verifies `bare_names == source_route_names`, then constructs supplies and calls `source.evaluate_protocol_slot` only. It never calls `bare.evaluate_protocol_slot` and never inspects a meter for the bare route. Therefore the changed AC and test-case expected results claiming equal route outcomes and ordered charges are not demonstrated. The current code has a tracked bare-route union-construction gap (QSL-1767); this test should either exercise the route and assert the stated parity once supported, or keep the spec/test claim scoped to what the route currently performs.
