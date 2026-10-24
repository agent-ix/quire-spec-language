---
id: SR-12242
title: "Gap analysis of QSL-559 E15 PR 3a ProtocolSystem<Sc>: compensation templates (PR #1129)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@8a7cdfbff3ffcf0b4f04e17b68ab5532c4119364; qsl-eval/src/simulation/protocol/**, qsl-analyze/tests/it/{protocol_compensation.rs,protocol_footprints.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-207
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-211
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: reviews
---
# SR-12242: Gap analysis of QSL-559 E15 PR 3a (PR #1129)

## Summary

Ticket: QSL-559. PR quire-spec-language#1129, head 8a7cdfbff. Plan completion: not assessed.

The criteria in scope were checked against the merged spec on origin/main. Each binding was read against its test body.

- FR-207-AC-1, AC-3 and AC-4, FR-205-AC-5 and FR-216-AC-4 are tagged.
- FR-207-AC-2 and the compensation part of FR-211-AC-5 are tested and left untagged ("Part of").
- FR-216-AC-5's template-visibility part is asserted in the FR-216-AC-4 test. AC-5 itself was tagged by an earlier PR, and its role part waits on part b.

The tagged tests and the compensation tests passed, as did `every_independent_pair_of_steps_commutes`. They were run narrowly through `locked-build.sh`.

## Verdict

**No major gap.** Every tagged binding asserts its criterion, with one exception: FR-216-AC-4's read-footprint clause is not exercised (FND-001). The mapping gap in the state key is a code finding (SR-12241 FND-001), not a coverage one. The probe that found it is a ready test for it.

Bindings checked:

- `pay_reaches_the_nine_states_of_its_table` → FR-207-AC-1: correct. The fixture's recovery predicate is the constant `true`, where §7.1 has `r.bal = 0`. The AC asserts states and transitions only, and those are identical.
- `the_retry_relation_and_the_count_decide_a_retry` → FR-207-AC-3: correct, both relations.
- `the_commit_closes_the_registration_and_stops_new_ones` → FR-207-AC-4: correct, including the second effect after `Ok`.
- `a_compensation_thread_takes_the_smallest_free_ordinal` → FR-205-AC-5: correct, both orders required.
- `compensation_steps_read_and_write_their_registrations` → FR-216-AC-4: incomplete (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-216-AC-4 is tagged as fully covered, but its clause "recomputing its post-state while changing a location outside its `R` leaves the post-state's change confined to its `W`" is not tested. `assert_changes_are_written` checks only that each step's change from the unperturbed pre-state lies in `W`. No test perturbs a location outside `R` and recomputes. Either add the perturbation (for example, change `c.w` or another thread's binder, then step again) or mark the tag partial. | qsl-analyze/tests/it/protocol_compensation.rs:618-653, qsl-analyze/tests/it/protocol_compensation.rs:689-704 |
| FND-002 | medium | FR-207-AC-2 can be covered only in part until the recovery-view spec gap is decided: "recovered at t7 because `bal = 0`" and "`unrecovered` with the refund leaving `bal = 1`". The PR records this, but the gap has no owning ticket ("Linear refuses new issues"). The same goes for the commit-occurrence key member, the attempt-record key member and the compensation thread's registration member (SR-12241 FND-001, FND-002 and FND-004). All four need one QSpec FR-425/FR-427 and ADR-027 PS-8/ST-12 spec ticket. | spec/functional/FR-207-run-compensation-templates-as-protocol-threads.md, qsl-analyze/tests/it/protocol_compensation.rs:371-407 |
| FND-003 | low | No test exercises the refusals `DetachedEffect` and `UnboundAttemptRecord`. Only `RecoveryView` is asserted. | qsl-eval/src/simulation/protocol/program.rs:344-348, qsl-eval/src/simulation/protocol/program.rs:705-713 |
| FND-004 | low | `every_independent_pair_of_steps_commutes` asserts `checked > 0` only for the `settled` fixture (index 14). It does not assert that a pair involving `CAttempt` or `CEnd` is among the pairs checked. The probe found that such pairs are (`(Attempt, CAttempt)`, `(Attempt, CEnd)`, `(Join, CEnd)`), so today the test is not vacuous. An assertion would keep it so. A fixture with two interleaved compensation threads (SR-12241's probe) would add `(CAttempt, CAttempt)` and `(CEnd, CEnd)`. | qsl-analyze/tests/it/protocol_footprints.rs:781-803 |

## Dispositions

Round 1, head 0d4ac0c4b. The focused tests ran through `locked-build.sh` and passed: the `protocol_` and `weak_memory` filters, 77 tests.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 0d4ac0c4b |
| FND-002 | deferred | STD-233 owns all four spec gaps: items 1(a)-(c) and 2. The PR body lists them as remainders. FR-207-AC-2 stays untagged until the recovery-view rule exists. |
| FND-003 | fixed | 0d4ac0c4b |
| FND-004 | fixed | 0d4ac0c4b |

Notes:

- **FND-001.** `a_location_outside_the_reads_changes_no_step`, tagged FR-216-AC-4, explores §7.1 from `c.w = 0` and from `c.w = 1`. It asserts that no step reads `c.w`, and that both explorations take identical steps with identical changes over the nine states.
- **FND-003.** The `DetachedEffect` refusal is deleted: the placement is now stepped and tested by `an_effect_waits_for_its_attempts_binder`. `UnboundAttemptRecord` is tested by `a_retry_over_attempt_records_is_refused`.
- **FND-004.** The `settled` fixture (index 14) now asserts that both a `CAttempt` pair and a `CEnd` pair are among the independent pairs checked. The crosswise fixture was added to the commutation units.
