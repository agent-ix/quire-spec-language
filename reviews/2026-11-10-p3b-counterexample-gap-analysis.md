---
id: SR-20261110041941
title: "Gap analysis of l5/p3b-counterexample (FR-281/FR-286 counterexample wire)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@e2c84f33e; diff origin/l5/p3-matrix...HEAD: qsl-analyze/tests/it/*, qsl-conformance/tests/it/*, qsl-replay/src/outcome/counterexample.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: reviews
---

## Summary

Ticket: QSL-12. Planless gap analysis scoped to the diff. It maps FR-281-AC-2, FR-281-AC-7 and FR-286-AC-14 to the tests the branch adds or edits, and checks the new wire code for untested branches. Plan completion: not assessed.

## Verdict

**PASS WITH FINDINGS.** The new tests run the real engines and replay, and they check that the document carries the same spelled value. None of them is tautological. The gaps: whole branches of `counterexample.rs` have no test, and the variant tests that back FR-286-AC-14 are tagged to other ACs.

## Scope

- FR-281-AC-2: examined. `clause_route.rs` checks one formula `ItemCounterexample` through both `analyze` and `provider::run`, and the trace is correct.
- FR-281-AC-7: examined (FND-004).
- FR-286-AC-14: examined (FND-001, FND-002, FND-003).
- FR-433-AC-1: examined. `a_protocol_deadlock_is_spelled_with_its_blocked_list_through_analyze` checks the fork/attempt steps, `kind: deadlock` and the blocked list, and the trace is correct.
- QSpec FR-485-AC-10: examined (FND-002).
- FR-202-AC-9: context_only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Untested branches of the new wire code. FR-286-AC-14 names a `time-lock` `Temporal` with its `final_delay`, and no test covers it. No test covers a lasso spelling: a non-empty `loop`, the `lasso.entry` split, or the appended terminal-stutter step (the FR-364-AC-1 shapes). No test covers the `send_record` step, the `Unspelled` error path, or the real `analyze`/`split_records` path that yields `MissingCounterexample`/`Unspelled`. The only test there is `from_analyze(record, None)`. `counterexample.rs` (552 lines) has no unit tests. | qsl-replay/src/outcome/counterexample.rs:288-365, qsl-conformance/tests/it/support.rs:81-84 |
| FND-002 | medium | The variant-spelling tests that back FR-286-AC-14 are tagged to other ACs. `a_refuted_protocol_clause_carries_its_spelled_counterexample` is tagged FR-281-AC-2, which is the Counter EN-1 `P` clause. `a_refuted_exact_probabilistic_item_is_spelled_and_its_path_set_replays` is tagged FR-202-AC-9, which covers the 2/3 fixtures; this test checks the spelling and a 9/10 mass. `a_refuted_contract_item_is_spelled_and_its_counterexample_replays` is tagged QSpec-FR-485-AC-10, but it checks neither `bound: true` nor `concrete`. In the computed matrix, FR-286-AC-14 is backed only by the byte-equality test. | qsl-analyze/tests/it/protocol_counted_steps.rs:407, qsl-analyze/tests/it/probabilistic_replay.rs:283, qsl-analyze/tests/it/operation_contract.rs:219 |
| FND-003 | low | `analyze_and_prove_write_the_same_refuted_item` (FR-286-AC-14) compares only the refuted item. AC-14 says "the same two records", which also covers FR-281-AC-1's proved record. | qsl-conformance/tests/it/prove_row.rs:73-84 |
| FND-004 | low | The FR-281-AC-7 test, `a_claim_that_takes_a_modulus_by_zero_...`, does not check the item's cause `UndefinedEvaluation` or that FR-285 maps the outcome to exit 10. It checks only the counterexample envelope, so the code-review FND-006 cause loss passes it. | qsl-analyze/tests/it/clause_item.rs:127-194 |
