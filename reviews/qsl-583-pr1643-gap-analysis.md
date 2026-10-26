---
id: SR-1643002
title: "gap analysis of PR #1643 (QSL-583b, FR-240 zone liveness and FR-241 lassos)"
type: SpecReview
analysis: gap-analysis
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1643 git diff origin/qsl-583-zone-liveness...d853a40f8; FR-240-AC-1, FR-240-AC-2, FR-240-AC-4, FR-241-AC-3, FR-241-AC-4, FR-236-AC-3; TC-695, TC-696, TC-691; qsl-analyze/tests/it/zone_liveness.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-240
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-241
    type: reviews
---
# SR-1643002: gap analysis of PR #1643 (QSL-583b, FR-240 zone liveness and FR-241 lassos)

## Summary

Ticket: QSL-583. Head: d853a40f8. Method: a manual check of each AC against its tests, plus probes. Plan completion: not assessed.

| AC | PR claim | Measured |
| --- | --- | --- |
| FR-240-AC-1 | covered | Covered in substance: Rpc with T = 4 holds, a Zeno self-loop is not reported, and a resetting variant is refuted and replays. The fixture differs from the AC: the AC's loop is `reply` on itself, guarded `x < 1` with no reset, while the test adds an `echo` step (FND-003). |
| FR-240-AC-2 | covered | Covered: `Serve` holds under `fair weak serve`, and with no fairness it is refuted by an `idle` lasso with a positive loop delay. |
| FR-240-AC-4 | partial (TT-4 half) | Partial, as claimed. `Stall` under the timed profile gives `NoAdmittedBehaviour`. The TT-1 half belongs to FR-239's vacuity. The test is correctly untagged for the AC. |
| FR-241-AC-3 | covered | Covered: the `idle` lasso has a positive loop delay and `x = 2 > 1` at entry. |
| FR-241-AC-4 | partial (solver only) | Partial, as claimed. `an_unsatisfiable_loop_has_no_delays` covers the solver. End to end needs a forged symbolic lasso, which is deferred. The PR names "QSL-583c, or the slice that builds forged symbolic lassos" as owner, but QSL-583's comments do not record it (FND-004). |
| FR-236-AC-3 | covered | Covered: a lasso with positive loop delay, replayed. FR-237's replay checks the discrete closure and that never-reset clocks are above the model's largest constant (3 here). |

**Deferrals.**
- **Certificate components to 583c:** confirmed in the code. FR-245's checker refuses a liveness clause as `UnsupportedItem`, so its `Holds` settles inconclusive.
- **MightyL to 583d:** consistent with the `UnsupportedForm` refusal of timed intervals.
- **STD-493 fd83efcd:** it exists and lists the three gaps. Gap 1 (idle tail vs step-cycle order) is real, because both are valid refutations of AC-2's `Serve`. Gap 2 (tick-source lassos with a never-reset clock) and gap 3 (unfair idle tail and vacuity) are real ambiguities.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test has a fairness-constrained step whose target has a time invariant, which is FR-232's third clause of enabledness. That is why SR-1643001 FND-001, a wrong `Holds`, passes the suite. Add the SR-1643001 repro: idle-tail and step-loop variants, under weak and strong fairness, each asserting `Violated` with a replayed lasso. | qsl-analyze/tests/it/zone_liveness.rs |
| FND-002 | low | No test compares `translate_position_negation` with the timed evaluator. The PR rests on "a timed-profile clause with no interval reads alike over positions". My probe (nine bodies, 426 lassos each, 0 disagreements) shows it is cheap to add as a regression test in qsl-eval's `timed_trace` tests. | qsl-eval/src/automaton/translate.rs:526-570 |
| FND-003 | low | FR-240-AC-1's variant is "`reply` may also loop on itself with guard `x < 1` and no reset". The test instead adds an `echo` step with that guard. The behaviour checked is equivalent, but the fixture is not the AC's. Use a `reply` self-loop, or reword TC-695. | qsl-analyze/tests/it/zone_liveness.rs:83-113 |
| FND-004 | low | The end-to-end half of FR-241-AC-4 is deferred to "QSL-583c, or the slice that builds forged symbolic lassos", but no ticket records it. Record it on QSL-583 (583c) with the AC id so it has one owner. | QSL-583 |

## Verdict

**Not mergeable**, together with SR-1643001: FND-001 here is the missing test for SR-1643001's high finding. Coverage otherwise matches the claims. FR-240-AC-1, AC-2, FR-241-AC-3 and FR-236-AC-3 are covered, and FR-240-AC-4 and FR-241-AC-4 are partial, as stated.

## Coverage

| Test | Tag | Binding |
| --- | --- | --- |
| `a_response_claim_holds_and_a_zeno_loop_is_not_reported` | FR-240-AC-1 | correct (fixture differs, FND-003) |
| `fairness_is_read_over_time_on_the_split_graph` | FR-240-AC-2, FR-241-AC-3 | correct |
| `a_liveness_claim_over_no_admitted_behaviour_is_undecided` | TC-695 | correct (partial AC-4, untagged) |
| `a_liveness_refutation_is_a_timed_lasso` | FR-236-AC-3 | correct |
| `an_unsatisfiable_loop_has_no_delays` | TC-696 | correct (partial AC-4, untagged) |
