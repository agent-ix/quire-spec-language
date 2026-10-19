---
id: SR-12132
title: "QSL-584 PR #958 gap analysis: FR-246, FR-247, FR-248 and QSpec FR-419 criteria"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@402ae5b3f63a936eb6e5fc40244340129ad36989; FR-246-AC-1, FR-246-AC-2, FR-247-AC-1..FR-247-AC-4, FR-248-AC-1..FR-248-AC-4, QSpec FR-419-AC-1..AC-6, TC-701, TC-702, TC-703; qsl-analyze/tests/it/schedulability.rs, qsl-analyze/src/schedulability, qsl-replay/src/certificate/closed_form"
review_set: subset
---

## Summary

Ticket: QSL-584. PR quire-spec-language#958. This is a planless gap analysis scoped to the diff, so plan completion was not assessed. The criteria were read from `spec/functional/FR-246-*.md`, `FR-247-*.md` and `FR-248-*.md`, and from QSpec FR-419 (cited by id only; read-only checkout). The bindings were read from the `#[trace(...)]` arguments in `qsl-analyze/tests/it/schedulability.rs`, where a criterion id beside a TC binds that criterion. For each binding, the review asked what source change would make the test fail.

The PR body correctly leaves FR-248-AC-4, FR-246 routing, spans, the unit refusal, and FR-419-AC-5 and AC-6 uncovered, because no task-set front end exists yet (QSL-861). It also discloses the FR-419 and FR-247 evidence-shape conflict (STD-159). It overclaims in one place: it says it "Covers FR-246-AC-1", and the trace tags claim four criteria that the tests only partly verify.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-246-AC-1 is bound by `ctl_is_admitted_under_both_policies_and_its_premises_are_its_identity`, and the PR body says it "Covers FR-246-AC-1". Two of the AC's three clauses have no implementation: routing to EN-7 under `schedulability`, and a different obligation identity. The test asserts `ScheduleSubject` struct inequality, which is not an obligation identity, and calls no router. The criterion is reported covered while its routing and identity halves fail. | qsl-analyze/tests/it/schedulability.rs:150-163, spec/functional/FR-246-check-task-sets-and-schedulability-claims.md:93 |
| FND-002 | medium | FR-246-AC-2 is bound by `a_task_set_or_claim_that_breaks_a_rule_is_refused_with_its_code`. The AC requires refusals "at the span" and the `3 m` deadline refusal. Neither exists without the front end. The PR body says "partly", but the tag marks the whole criterion covered. | qsl-analyze/tests/it/schedulability.rs:165-223, spec/functional/FR-246-check-task-sets-and-schedulability-claims.md:94 |
| FND-003 | medium | QSpec FR-419-AC-2 is bound by `a_third_wcet_of_six_misses_under_fixed_priority_and_edf` and by `ctl_settles_proved_..._settles_refuted`. The AC's fixed-priority evidence, "scheduling points 4, 6, 8, 12 and demands 9, 10, 12, 13", is not produced or asserted. The engine returns RTA iterates `6, 10, 13`. The conflict is disclosed (STD-159), but the tag claims the criterion. | qsl-analyze/tests/it/schedulability.rs:262-277, qsl-analyze/tests/it/schedulability.rs:379-402 |
| FND-004 | medium | FR-248-AC-1 requires settlement "with fixpoints (1, 3, 12) in the record", and the WCET-6 claims to "settle refuted with their evidence". `settle` returns a bare `TerminalValue` with no record and no evidence, so that half is unverified. The tests assert fixpoints on the engine outcome instead. `a_stopped_analysis_settles_incomplete` is also bound to FR-248-AC-1, although it tests the `Stopped` map row, which AC-1 does not mention (a wrong binding). | qsl-analyze/tests/it/schedulability.rs:379-402, qsl-analyze/tests/it/schedulability.rs:513-525, spec/functional/FR-248-settle-a-closed-form-verdict-after-recomputing-its-evidence.md:100 |

## Verdict

FIX, for coverage inflation. Narrow the four tags so that none claims a criterion the test only partly verifies, and correct the "Covers FR-246-AC-1" line in the PR body. Alternatively, implement the missing halves; that is QSL-861's front-end work for FND-001 and FND-002. The code soundness gap on EDF jitter and blocking is recorded in SR-12131 FND-001. No criterion covers it, and no test states `J` or `B` under an EDF claim.

Criteria examined with no finding (oracle strength confirmed):

- FR-247-AC-1: the fixpoints `1, 3, 12`, the full `5, 9, 12, 12` job list, and the EDF `Demand { busy_period: 12, qpa_sequence: [8] }` are asserted by equality. The response claim holds at 12 and misses at 11 with iterates `5, 9, 12`.
- FR-247-AC-2: `6, 10, 13`, `13/12`, and the jitter-and-blocking `5, 7` are asserted by equality, and the checker agrees (`recompute` standing in for `check_closed_form`).
- FR-247-AC-3: low `1, 2`, high `4`, `SufficientTestFailed` at `C(HI) = 6`, and the low-mode `Miss` at `5, 7`.
- FR-247-AC-4: the default sequence `16, 15, 14, 9, 7`, the `Stopped(ResourceExhausted, MaxDemandPoints(3))` result with the value 3, and the cancel stop.
- FR-248-AC-2: fixpoint 11 gives `Iterate { index: 2 }` and `ReplayParity`; the miss iterate 12 gives `Iterate`; and job 2 gives `BusyPeriodClosed { job: 1 }` with parity, while job 0 with `[2]` gives `Refuted`. The shape, raised-WCET, shortened-QPA and wrong-L tampers each disagree.
- FR-248-AC-3: AMC gives `Proved{AmcRtb}`, `SufficientTestFailed`, and `Refuted` for the low-mode miss.
- QSpec FR-419-AC-1, AC-3 and AC-4: these are covered within the same tests.
- FR-248-AC-4 and QSpec FR-419-AC-5 and AC-6 are untagged, which is correct.

Recorded lows, not findings:

- FR-247-AC-2 and QSpec FR-419-AC-4 name `check_closed_form`. The tests call `recompute`, its recomputation core, and the entry itself does not exist.
- FR-247-AC-1's "utilization exactly 1" under EDF is not asserted.
- No tamper test covers AMC high-mode evidence, an EDF demand witness, or a stated utilization.

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | QSpec FR-419-AC-1 is now bound only by `ctl_is_schedulable_under_fixed_priority_and_edf`, which asserts the engine outcome and never settles. The fix round removed the tag from `ctl_settles_proved_..._settles_refuted`, which was the only bound test that asserted "settles `proved`". The AC's settlement half, and its "utilization exactly 1" under EDF, are now unverified by any binding, while the PR body lists the criterion as bound. Fix: re-tag the settle test with `QSpec-FR-419-AC-1` (it asserts `Proved{FixedPriorityRta}` and `Proved{EdfQpa}` with the fixpoints 1, 3, 12) and assert the EDF utilization of 1, or drop the criterion from the tag and from the PR body. | qsl-analyze/tests/it/schedulability.rs:227-243, qsl-analyze/tests/it/schedulability.rs:381-405 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 6885f2141: the `#[trace("TC-701", "FR-246-AC-1")]` tag was removed. The doc comment now says routing and identity are not implemented and no criterion is bound, and the PR body no longer claims FR-246-AC-1. |
| FND-002 | fixed | 6885f2141: the `FR-246-AC-2` tag was removed. The doc comment says spans and the `3 m` refusal are not covered, and the PR body lists AC-2 as not covered. |
| FND-003 | fixed | 6885f2141: `QSpec-FR-419-AC-2` was removed from both tests, and the PR body lists it as not bound. |
| FND-004 | fixed | 6885f2141: `FR-248-AC-1` was removed from the settle test and from `a_stopped_analysis_settles_incomplete`. The settle test's doc comment says the evidence half is not asserted. |
| FND-005 | still-open | Found in this pass. QSpec FR-419-AC-1's settlement half and EDF utilization 1 have no bound test, while the PR body lists the criterion as bound. |
