---
id: SR-1640005
title: "Gap analysis (round 2) of PR 2291: FR-520-AC-11 per-binder charge log (QSL-1640)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@6bc6489f951546e8c26919f24a51d6fe07e3e7f9; QSL-1640; PR #2291; spec/functional/FR-520-*.md, spec/test-cases/TC-1315-*.md, qsl-eval/src/value/expression/protocol_slot/charges.rs, qsl-eval/src/value/expression/evaluate.rs, qsl-eval/src/value/expression/evaluate/invariants.rs, qsl-eval/tests/it/protocol_slots/acceptance/state_compensation.rs, qsl-eval/tests/it/protocol_slots.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-520
    type: reviews
---
# Gap analysis (round 2) of PR 2291

## Summary

Ticket: QSL-1640. Planless gap analysis: plan completion was not assessed. Each clause of FR-520-AC-11 and each scenario in TC-1315 row 12 was mapped by hand to the tests tagged `FR-520-AC-11`:

- 12 integration tests in `state_compensation.rs`.
- One dispatch test in `protocol_slots.rs`.
- Four unit tests in `invariants.rs`.

The following are covered by value-asserting tests that would fail if the accounting broke:

- Presence and absence of entries, including the `Unavailable` binder.
- Conservation against the meter's consumed counters, with a sum for cumulative counters and a maximum for high-water ones.
- Equal supplies give equal logs.
- `accounting_version`.
- A literal-only prefix and a literal-only infix.
- Two-binder equality and its commuted form.
- A single-binder operation after another binder was read.
- Call-before-read, one-argument and two-argument calls, a call over two binders, let, dispatch, and capture restore with a non-empty caller interval.

FR-520-AC-9 is now tagged on the reader-backed parity test, and its helper compares the log. No stubs, `TODO`s or orphan code were found in the diff.

## Verdict

**Gap.** Two TC-1315 row-12 scenarios are covered only in a form that cannot detect the defects SR-1640004 found:

- The iteration element bindings are tested only on the first element.
- No connective-over-two-binders case exists, and the closest test pins the wrong value.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-1315 row 12 "the query, `seq` and fold element slots carry the owner of their source" is tested only by unit tests that run a single `iterate` step with `awaiting: false`, before any body result has been popped. No test binds a second element after a body result over another binder, so SR-1640004 FND-002 passes every test. Add an integration test: a pure helper `forall(x in xs(n): x * 2 + m > 0)` over a one-element and a three-element source, called with two binders. Assert that the source binder's entry grows with the element count. | qsl-eval/src/value/expression/evaluate/invariants.rs:2612-2662 |
| FND-002 | medium | No test covers a connective (`and`/`or`/`implies`) whose operands come from two binders as an operation over two binders. The base fixture test asserts `activated` `[..,2,2]`, which contains the `and` connective's own charge, so it pins SR-1640004 FND-001 rather than detecting it. Add a case `target = 5 and not activated` that asserts the connective's retain charge is unattributed and `activated` holds only `not`'s charge. | qsl-eval/tests/it/protocol_slots/acceptance/state_compensation.rs:817-830 |

## Coverage

Examined:

- FR-520-AC-11.
- TC-1315 row 12, each scenario.
- The FR-520 outcome-map row for per-binder accounting.
- FR-520-AC-9 (the tag and the comparison helper).
- Every test tagged FR-520-AC-11 in the diff.

The four `invariants.rs` unit tests are bound correctly to FR-520-AC-11 and TC-1315, but they cover only the first element.

## Dispositions

- FND-001: fixed. Unit tests for query, fold and seq elements after the first (`invariants.rs`) and the integration test `every_element_of_a_query_over_one_binder_is_charged_to_that_binder`.
- FND-002: fixed. `a_connective_over_two_binders_is_unattributed_and_leaves_the_right_binder_its_own_operation`; the wrong pinned values were corrected.
