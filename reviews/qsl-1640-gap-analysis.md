---
id: SR-1640002
title: "Gap analysis of PR 2291: FR-520-AC-11 per-binder charge log (QSL-1640)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0bd98250e0bfe4c9bfc353a23534e19126172115; QSL-1640; PR #2291; spec/functional/FR-520-*.md, spec/test-cases/TC-1315-*.md, qsl-eval/src/value/expression/protocol_slot/charges.rs, qsl-eval/tests/it/protocol_slots/acceptance/state_compensation.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-520
    type: reviews
---
# Gap analysis of PR 2291

## Summary

Ticket: QSL-1640. Done by hand with `quire matrix --scope .` (quire 0.38.3): FR-520-AC-11 is `tagged` by four tests (state_compensation.rs lines 739, 771, 806, 844) where main reported it uncovered. FR-520-AC-3, AC-4 and AC-9 are `untagged` on this head; that is pre-existing (the PR does not touch them) and out of scope, except that AC-9 says "charge log included" and no AC-9-tagged test exists. A probe (compare `BinderCharges` inside the in-process versus reader-backed comparison helper) passed, so the property holds today but is unguarded.

Quoin `gap-analysis` plan completion: not assessed (planless). Code with no owning requirement: the `BINDER_ACCOUNTING_VERSION` constant and the `BinderCharges` API are owned by FR-520-AC-11 and the outcome map row; no orphan found. No stubs, placeholders or `TODO` in the diff.

## Verdict

**Gap**: the AC-11 tests are value-asserting and non-vacuous for per-binder presence, conservation, sum versus maximum, version and capture restore (four of six mutations killed), but the unattributed clause and the root-frame gate have no failing test, and AC-11's attribution content is under-specified (see SR-1640003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-520-AC-11's clause "charges made by no binder are recorded under none" has no test that can fail: with the unattributed routing deleted (mutation M4) all four AC-11 tests still pass because every fixture charge occurs after a binder read. Add a slot whose expression charges before the first binder read (a literal-only prefix) and assert the nonzero `unattributed()` entry. | qsl-eval/tests/it/protocol_slots/acceptance/state_compensation.rs:773-805 |
| FND-002 | medium | The root-frame gate in `route_binder_read` (a callee frame's local slot must not be read as a binder) is untested (mutation M6 survives). Add a slot expression calling a model function whose parameter slot index equals a binder slot index, and assert the callee reads do not create or switch binder entries. | qsl-eval/src/value/expression/evaluate.rs:1819-1823 |
| FND-003 | low | FR-520-AC-9 requires equal results "charge log included" for the in-process and read-back package; no test is tagged FR-520-AC-9 and the existing comparison helper `without_location` discards the new `BinderCharges` (`_`). The probe shows equality holds; make the helper compare the log and tag a test with FR-520-AC-9. | qsl-eval/tests/it/protocol_slots/acceptance.rs:2882-2889 |

## Coverage

Examined FR-520-AC-9, FR-520-AC-11 and TC-1315 row 12 against the matrix and the four tagged tests; FR-520 outcome map row for `EvaluationReport` per-binder accounting against `charges.rs`.
