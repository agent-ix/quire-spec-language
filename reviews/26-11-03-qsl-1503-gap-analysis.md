---
id: SR-2224002
title: "Gap analysis of quire-spec-language PR 2224: FR-524, FR-526, FR-516 and the legacy FR-043/044/052/053 rows against the tagged tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d49930ea0; PR #2224: FR-524-AC-1..31, FR-526-AC-1..19, FR-516-AC-1..24, FR-523-AC-3 and AC-14, FR-502-AC-12, FR-043-AC-8..12, FR-044, FR-052-AC-9..12, FR-053-AC-7..9 against qsl-eval/tests/it/temporal_activation.rs, qsl-eval/tests/it/temporal_relation.rs, qsl-replay/src/spine/clause/tests/temporal.rs and the evidence module"
review_set: subset
---

## Summary

Ticket: QSL-1503. This is a planless gap analysis of the PR diff, read from `quoin matrix --repo . --json` (quoin 0.29.4) and from the test bodies. Plan completion: not assessed. Each tagged test was read for what it asserts, and six mutations were run against the evidence module (see SR-2224001) to check that the assertions can fail.

Matrix result for the ids in scope. Untagged: FR-516-AC-1, AC-5, AC-14, AC-18; FR-524-AC-23; FR-526-AC-17 (Test) and AC-13 (Inspection, method without symbol). Every other id of FR-524, FR-526, FR-516, FR-043, FR-044, FR-052 and FR-053 is tagged. FR-502-AC-9 to AC-11 and FR-523-AC-10, AC-11, AC-15, AC-19 are untagged but belong to other tickets.

These claims hold, each with a value-asserting test that would fail on the old or a wrong behavior:

- FR-524-AC-1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 to 17, 18, 19, 21, 22, 26, 27 to 30: the tests assert identities, receipts in arrival order, environment equality, fault subject and kind in FR-524's order, the instances/captures/visit charge vector [4, 2, 2, 2], and the four dispositions over all eleven scope combinations. Limits noted in SR-2224001 FND-010 (AC-20 ledger assertion).
- FR-524-AC-2, 25, 31 and FR-516-AC-2, 10 to 13, 15, 16, 19 to 24: instance-bound valuations read only the assessed instance, `also_held_by` is asserted, the refusals carry the stated cause with an empty charge log, `trigger-required`/`trigger-unknown`/false-guard are distinguished, and the activation position 0/anchor distinction changes the verdict.
- FR-526-AC-1, 2, 4 to 12, 14 to 16, 18, 19: causes, lineage numbers, latest-entry replacement, determinism with the charge log, and the ceiling stop are asserted. AC-3 and the second half of AC-14 hold by construction (see SR-2224001 FND-010). AC-12's completes half is FND-001 of SR-2224001.
- FR-526-AC-17 is implemented by compile_fail doctests on `RetainedInstance`, `ResultEntry` and `EvidenceRequest`, which pass (42 doctests green).
- FR-502-AC-12: `two_clauses_with_one_body_keep_separate_progress_bindings` asserts one shared clause node, two occurrences, retained watermarks 100 and 50, and the later lower watermark under `Bounded` refused with `Watermark`. It fails if `occurrence` leaves the binding. The evidence-side test `two_occurrences_of_one_body_keep_separate_instances_results_and_progress` does the same for instances and results.
- FR-043-AC-8 to 12, FR-044-AC-1 to 9, FR-052-AC-9 to 12, FR-053-AC-7 to 9 are tagged by the legacy tests and map to the spine ACs above; their spine halves are covered where the ACs above are.

The author's deviations are sound. The call takes `EvidenceSubject` because the in-memory CheckedPackage carries no `temporal_bindings` clock rows, so `CheckedPackageEvaluation::evaluate_temporal_evidence(&CheckedNodeId, ...)` cannot authenticate the binding; FR-516-AC-2's "package does not hold the clause" and FR-516-AC-18's read-back equality are the two criteria this removes, and both need the package-level entry. FR-524-AC-23 restates FR-337-AC-8 (TC-1267), which owns the S3 refusal, so deferring it to TC-1267 narrows nothing; the AC should be re-pointed at FR-337-AC-8 in the spec. Nothing in the PR defers, narrows or drops ticketed behavior: no ticketed row was removed and no refusal was substituted for a verdict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-516-AC-5 (each progress axis, the trigger scope and the completeness record echoed as supplied, and a change to one changing only its field) has no tag and no test. The result echoes them (call.rs `conclude`), but nothing varies one axis and asserts that the others and the truth stay put, which is the FR-043-AC-10 independence claim on the route. | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md; qsl-eval/src/temporal/evidence/call.rs:1250-1290 |
| FND-002 | medium | FR-516-AC-14 (a sibling clause evaluated in the same package after the counter stop keeps its own result) has no tag and no test, and the author's list of uncovered criteria names only AC-18. `a_shared_meter_stops_the_second_instance_at_the_visit_charge_point` stops a second call of one clause, not a sibling clause that must still succeed. | qsl-eval/tests/it/temporal_activation.rs:1170-1195 |
| FND-003 | medium | `each_counter_completes_at_its_usage_and_one_below_is_incomplete` is tagged FR-523-AC-3, which fixes `usage` at formula_nodes 2, instances 1, positions 3, valuations 3, captures 0, evaluation_steps 8 and visited 8. No test asserts any of those numbers through the call; the test proves FR-516-AC-7. Its tag overclaims FR-523-AC-3. | qsl-eval/tests/it/temporal_activation.rs:1069-1105 |
| FND-004 | medium | `a_shared_meter_stops_the_second_instance_at_the_visit_charge_point` is tagged FR-523-AC-14, which sets the ceiling to the first call's usage plus one and requires the second result to name t2. The test uses the measured charge count as the ceiling and checks only that the stop is a Meter stop; it does not name t2. | qsl-eval/tests/it/temporal_activation.rs:1170-1195 |
| FND-005 | medium | FR-516-AC-8 says the call's `temporal.visit` charges are FR-501-AC-12's plus one per delivery or origin. `an_event_position_clause_is_evaluated_through_the_progress_carrier` (tagged AC-8) asserts only the truth, so the charge half is untested. The same missing vector means the question whether silent samples are charged on the call meter has no test to settle it. | qsl-eval/tests/it/temporal_activation.rs:1197-1224 |
| FND-006 | medium | FR-516-AC-17 requires both a regressing watermark and a conflicting completeness revision to be refused through the call. Only the watermark is tested, and the progress-ledger-unchanged claim is `kept.len() == 1`. | qsl-eval/tests/it/temporal_activation.rs:1107-1123 |
| FND-007 | low | Matrix gaps on criteria the code does meet: FR-516-AC-1 and FR-526-AC-17 are covered by compile_fail doctests that the matrix does not see (no tag), and FR-526-AC-13 is an Inspection criterion with no symbol, although the code uses `checked_add`. Add a traced `#[test]` that names each doctest's subject, or record the Inspection symbol, so the matrix agrees with the code. FR-516-AC-18 (read-back package equality) and FR-524-AC-23 (re-point at FR-337-AC-8) are acknowledged by the author. | qsl-eval/src/temporal/evidence/ledger.rs:140-158; qsl-eval/src/temporal/evidence/request.rs:123-135 |
| FND-008 | low | FR-502-AC-12 states that renaming `Slow` to a third name changes neither entry's key, and that the later lower watermark under `Fast` is refused with retained 100. The test uses `Twin` only, never renames it, and asserts the refusal's dimension but not the retained value. | qsl-replay/src/spine/clause/tests/temporal.rs:3105-3165 |
| FND-009 | low | FR-524-AC-28 and AC-31 through the call: `Missing` and `Refused` trigger evidence (codes `trigger-evidence-missing`, `trigger-evidence-refused`) and a closed scope with an incomplete record are tested only through `activate`. The call test covers Inactive and Unknown(open) only. | qsl-eval/tests/it/temporal_activation.rs:820-866; qsl-eval/src/temporal/evidence/call.rs:480-503 |
| FND-010 | low | FR-524-AC-12 says the faulted instance evaluates no position. The call-level test covers a Missing capture only (truth None, activation Unavailable); a Null capture and an evicted capture reach the same code but are asserted only at `activate` level. | qsl-eval/tests/it/temporal_activation.rs:497-564; qsl-eval/tests/it/temporal_activation.rs:820-866 |

## Verdict

The spec rows named in the ticket are implemented, and 20 of the 24 FR-516 criteria and all of FR-524 and FR-526 except the deferred AC-23 are tagged and mostly value-asserting. The gaps are in tests, not in behavior: FR-516-AC-5 and AC-14 are neither tagged nor named by the author, two FR-523 tags overclaim (FND-003, FND-004), and the charge vector that would pin FR-516-AC-8 is absent. The PR is not mergeable until FND-001 to FND-006 are closed in this PR; FND-007 to FND-010 are low. No ticketed behavior is narrowed.

## Dispositions

Round 1 is at head ea4694cb0.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | each_axis_is_echoed_as_supplied test varies four axes, then the decision closure alone, and asserts the other fields and truth stay and the settlement changes. |
| FND-002 | fixed | a_sibling_clause_keeps_its_result_after_a_counter_stop stops one clause on positions=0 and evaluates a sibling on the same meter with truth True. |
| FND-003 | fixed | an_origin_clause_over_three_positions test asserts formula_nodes 2, instances 1, positions 3, valuations 3, captures 0, evaluation_steps 8, visited 8, history_span 5 and 8 meter charges, as FR-523-AC-3 states. |
| FND-004 | fixed | a_shared_meter_one_unit_short test uses ceiling usage+1, asserts the first result equals the unmetered one, the second is a Meter stop naming t2, and the fresh-meter replay equals the first. The silent-sample clause variant of AC-14 is not exercised. |
| FND-005 | fixed | the AC-3 test asserts the origin plus seven charges (8 meter units) and the silent test asserts the silent-sample rule; the exact FR-523-AC-21 vector is FND-011 of SR-2224001. |
| FND-006 | fixed | a_conflicting_completeness_revision test refuses Complete{through:40} after 100 with progress-contradiction, an empty charge log, and asserts the retained watermark and completeness. |
| FND-007 | still-open | FR-516-AC-1 and FR-526-AC-17 are now tagged and pass the matrix. FR-526-AC-13 is still method-without-symbol and FR-524-AC-23 is still untagged: its spec row still reads 'Test (TC-1319)' and was not re-pointed at FR-337-AC-8 (TC-1267) as promised. Re-point the row or tag it. |
| FND-008 | fixed | RenamedTwin test asserts the key is unchanged by a rename and the refused Watermark carries the retained value. |
| FND-009 | fixed | trigger_evidence_and_capture_faults_reach_the_call covers Missing/Refused trigger evidence, the incomplete record and Null/evicted capture faults through the call. |
| FND-010 | fixed | same test asserts the faulted instance evaluates no position (Unavailable, no truth) for Missing, Null and evicted captures. |
