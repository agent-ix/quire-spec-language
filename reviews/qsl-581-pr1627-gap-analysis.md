---
id: SR-1627002
title: "gap analysis of PR #1627 (QSL-581b, FR-244 zone certificate)"
type: SpecReview
analysis: gap-analysis
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1627 git diff origin/main...32411b40c; FR-244 AC-1 to AC-4, FR-239-AC-1; qsl-analyze/tests/it/zone_search.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-244
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: reviews
---
# SR-1627002: gap analysis of PR #1627 (QSL-581b, FR-244 zone certificate)

## Summary

Ticket: QSL-581. Head reviewed: 32411b40c. The method was a manual AC-to-test check against the diff, scoped to FR-244 and FR-239-AC-1. Plan completion: not assessed.

| AC | PR claim | Measured |
| --- | --- | --- |
| FR-239-AC-1 | fully tagged | Covered. `no_late_reply_holds_at_four_and_is_refuted_at_three` asserts `Holds` with a certificate at T=4, `Violated` at position 3 at T=3, and deadlock-freedom `Holds` under both. |
| FR-244-AC-1 | partial | Partial. Tested: a reachability part, no components, the obligation, `admitted` = `Quiescent(0)` (`Idle`), and equal reruns. Not tested: "every edge's target zone covering its successor" (only index bounds are asserted) and "FR-245 accepts it". The FR-245 half is carried by FR-245-AC-1 on QSL-582, so that deferral is sound. |
| FR-244-AC-2, AC-3 | deferred to QSL-583 | The deferral is sound, because they need FR-240's components and divergence part. It is not recorded on QSL-583, whose ticket implements only FR-237, FR-240, FR-241 and FR-242. |
| FR-244-AC-4 | covered | Partial. The `locked` node, `ping` edges staying in locked nodes, and no quiescent node are asserted. The AC's "and no cycle of positive total delay" is not asserted, and under the PR's `cycle_delays` rule the `ping` loop would count as one (SR-1627001 FND-001). |

Spec-gap claims, checked in Linear: STD-493 holds items 1 to 4 and a comment for item 5 (the FR-239-AC-5 atom). Neither of the two gaps this PR's code and PR body cite to STD-493 (one initial node vs one per initial state, and `NoDivergenceWitness`) is on STD-493. A Linear search for `NoDivergenceWitness` finds nothing. The one-initial-node gap is real: `TimedSystem::initial()` returns several states and FR-244 names one `NodeIndex`. The `NoDivergenceWitness` gap is not real: it is invented semantics, which SR-1627001 FND-002 reproduces as an implementation defect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-244-AC-4 is tagged as covered, but `a_locked_node_is_listed_with_its_edges` never asserts the AC's clause that the `ping` edges reach "no cycle of positive total delay". Under the implementation's `cycle_delays`, the `(B, x <= 1)` `ping` self-loop would pass as a positive-delay cycle (SR-1627001 FND-001). Add the assertion once the witness rule is fixed, for example by checking that no `admitted`-eligible cycle passes through the locked node. | qsl-analyze/tests/it/zone_search.rs:672-702 |
| FND-002 | medium | Spec gaps are cited to STD-493 but not filed there. `ZoneCertificate::initial` (a Vec; FR-244 names one `initial: NodeIndex`) and `ZoneUndecided::NoDivergenceWitness` both say "spec gap STD-493", and the PR body says they were filed. STD-493 and its comments do not contain them. File the real one (several initial nodes) on STD-493. Drop `NoDivergenceWitness` instead of filing it (SR-1627001 FND-002). | qsl-analyze/src/zone_check/certificate.rs:39-43; qsl-analyze/src/zone_check/search.rs:134-138 |
| FND-003 | medium | The FR-244-AC-2 and AC-3 deferral to QSL-583 exists only in the PR body. QSL-583's "Implements" list omits FR-244, and QSL-581's scope-split comment does not move these ACs. So when QSL-581 closes, they are owned by no ticket. Record them on QSL-583, as was done for FR-239-AC-4. | spec/functional/FR-244-emit-a-zone-certificate-with-every-proof.md |
| FND-004 | low | FR-244-AC-1's "every edge's target zone covering its successor" is not asserted. The test checks only that `from` and `to` are in range. It can be asserted now without FR-245, by recomputing each edge's successor and checking `alu_covered_by` against the target zone with the listed `lu_bounds`. | qsl-analyze/tests/it/zone_search.rs:625-668 |
| FND-005 | low | `a_proof_with_no_quiescent_state_is_admitted_by_a_cycle` checks only that the cycle's shape closes. It does not check that the cycle is time-divergent, so it cannot fail on SR-1627001 FND-001's Zeno witness. Add the Zeno-plus-divergent model (FND-001's probe) and assert that the witness is the divergent loop. Add the urgent-phase model (FND-002's probe) and assert `Holds`. | qsl-analyze/tests/it/zone_search.rs:704-741 |

## Verdict

**Not mergeable** as one unit with SR-1627001. On the gap side: AC-2 and AC-3 are soundly deferred but need a home on QSL-583 (FND-003). AC-4 is tagged as covered while one clause is untested (FND-001). The STD-493 claims are not true as filed (FND-002). Correct: FR-239-AC-1 is now fully covered, and the FR-245 half of FR-244-AC-1 belongs to FR-245-AC-1 on QSL-582. Tracker note, not a code finding: QSL-581 is in state Done while #1627 is open and slices 581c and 581d remain.

## Coverage

Method: manual AC-to-test trace over the PR diff and FR-244, FR-245 and FR-240 text. `quoin matrix` was not run, because the review is scoped to the PR.

## Dispositions

Round 1, against fix commit a7fa78711.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
