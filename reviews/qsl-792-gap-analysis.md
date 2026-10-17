---
id: SR-12052
title: "QSL-792 PR #925 gap analysis: FR-366 type-level invariants, TC-1050 and TC-1051"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ed3f27d1c4aeaa45ac3c5900387e50fe94bd59e9; FR-366-AC-1..FR-366-AC-9, FR-106-AC-48, TC-1050, TC-1051; qsl-eval/tests/it/type_invariants.rs, qsl-replay/src/spine/lifecycle/tests.rs, qsl-eval/src/value/expression/value_invariants.rs, qsl-semantics/src/model/type_invariant.rs"
review_set: subset
---

## Summary

Ticket: QSL-792. PR quire-spec-language#925. A planless gap analysis scoped to the diff. Plan completion was not assessed. Each criterion was traced to its tagged tests by reading them. No matrix was run, because the brief was read-only. Author-recorded gaps are not re-reported: FR-106-AC-48 admission, the state-machine half of FR-366-AC-9, the TC-1051 step 6 fixture, and step 1 history ordering.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-366-AC-8 test denies by charge point and occurrence, and never compares the charge log or the denial cause. A mutant that charges a record-field read, or adds any other charge per value, still passes. TC-1051 step 9 says the test catches exactly that ("reading a record field charges nothing") | qsl-eval/tests/it/type_invariants.rs:1175-1243 |

## Verdict

FIX (one test-oracle gap). Every other criterion in scope has a test that can fail:

- FR-366-AC-1: `a_type_level_clause_is_an_invariant_a_guard_or_unevaluated` covers TC-1050 steps 1 to 4, including the comparison under reversed order.
- FR-366-AC-2: `an_invariant_is_checked_and_a_pre_read_refuses_at_its_clause` asserts the code, cause, clause key and `clauses[0]`, and that there is exactly one refusal.
- FR-366-AC-3: `an_invariant_must_be_boolean_parse_and_name_its_position` covers the non-Boolean root, the parse refusal, `ocl` not being parsed, and position 1 with `Stable` before `NonNegative`, which also checks that `k` is the document position and not the key order. The spine test `select_parses_type_invariants_and_refuses_text_that_does_not_parse` covers the I1 path.
- FR-366-AC-4: `every_reachable_money_value_has_one_result_at_its_path` checks all six exact paths, their order, and the outcome.
- FR-366-AC-5: `a_result_depends_on_its_value_alone` covers reversed field listing, repeat determinism including the charge log, and a change to `history[0]`.
- FR-366-AC-6: `an_event_type_no_value_reaches_has_no_result`.
- FR-366-AC-7: tests for the subtype value, the nested `Pair`, the event parameter and `a_reference_is_not_followed`.
- FR-366-AC-9, process half: `a_process_invariant_has_one_result_for_each_run`, which includes the `UnknownClause` check.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed ab8b210e3 | The AC-8 test now asserts the unlimited charge log equals `[ordering.operands, ordering.arithmetic, ordering.result-retain]` × 6, and asserts `resource_exhausted`/`insufficient-next-charge` on the denied run |
