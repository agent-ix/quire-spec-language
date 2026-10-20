---
id: SR-13471
title: "Gap analysis of quire-spec-language PR #1094: FR-155 to FR-162 partial-order criteria against tests and code"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@6fc8961654f89a1ae3159b5bac84886252a4db18; PR #1094: FR-155-AC-6/AC-7, FR-156-AC-1..AC-6, FR-157-AC-1..AC-6, FR-160-AC-1/AC-3/AC-6/AC-7, FR-162-AC-1..AC-5 against qsl-analyze/tests/it/{partial_order,reduction_differential,reduction,graph_core}.rs, qsl-eval/tests/it/footprints.rs and qsl-analyze/src/model_check/ample.rs, qsl-eval/src/simulation/{footprint.rs,model/walk.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-155
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-156
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-157
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-160
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-162
    type: reviews
---
# Gap analysis of quire-spec-language PR #1094

## Summary

Ticket: QSL-564. Plan completion: not assessed.

Method: I read each criterion of FR-155 to FR-162 that the PR claims or
implements. I read the computed matrix (`quoin matrix --json`, quoin 0.28.3,
at the PR head) and grepped the `#[trace]` tags. The matrix marks
FR-160-AC-1, FR-162-AC-4 and FR-157-AC-4 `untagged`, and every other
criterion in scope `tagged`. For each
claimed criterion I read the test against the AC text. Where the AC names
behaviour, I ran the real engine and mutated the code (SR-13470 Summary has
the probe and mutation evidence).

Clean (examined, test asserts the AC):

- FR-155-AC-6 (result subject: `set, pick`, reduced equal to unreduced).
- FR-156-AC-1 (ample sets per state, 7 model states, 6 product states per
  instance).
- FR-156-AC-3 (ring proviso, prefix `step, step, bump, step, step, bump`).
- FR-156-AC-5 and AC-6 (four §7.6 vectors: operations and kind).
- FR-157-AC-1, AC-2, AC-5 and AC-6.
- FR-160-AC-3 (`ReductionHorizon{2}`, wire `reduction-horizon`;
  `BoundReached{1}` under symmetry alone).
- FR-160-AC-6 (19 cases, 52 runs; catches mutations M1, M2, M4, M5, M6).
- FR-160-AC-7 (`max_states` 3 to V-7; the completed proof `Uncertified`).
- FR-162-AC-1, AC-2, AC-3 and AC-5 (model half).

Partly verified (findings in SR-13470):

- FR-156-AC-2: path not pinned (FND-005).
- FR-156-AC-4: the test does not exercise the disabled-member closure
  (FND-004).
- FR-157-AC-3: operations not pinned (FND-010).
- The "over N states" counts (FND-007).

FR-157-AC-4's `ProtocolSystem` half is out of scope: QSL-562 and QSL-559 are
not built. Its model half is `each_granularity_leaves_the_visibility_and_verdict_unchanged`,
tagged TC-579 only, and the PR says so.

`Location::affects` is code with no owning requirement sentence. It goes
against FR-155 "Meeting" and "Independence" as written. It is recorded once,
as SR-13470 FND-002 (spec follow-up), not repeated here.

## Verdict

Changes requested: one high finding and two low findings. The high finding is
an AC clause with no test, which the code fails.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-155-AC-7's candidate clause ("`spawn` ... writes `AnyMembership{counters}`, which meets `Membership{counters, d}`, so the candidate seeded by `spawn` holds the disabled `bump(d)`") is not tested, and the code fails it. The one test tagged FR-155-AC-7, `footprints_follow_the_object_types_of_the_identitys_triples`, covers footprints only. `a_transition_over_an_object_not_yet_created_has_a_footprint` (TC-574) asserts only `!independent(spawn, bump(d))`. `Por::candidate_seeded_by` cannot return `bump(d)` while `d` is absent, because `identities_of` lists only objects the state holds. The real-engine probe shows the same omission: the candidate seeded by `spawn(y)` is `{spawn, spin}`, without the disabled `probe(y, d, x)`. Root cause and soundness impact: SR-13470 FND-001. | qsl-eval/tests/it/footprints.rs:1361-1379; qsl-eval/tests/it/footprints.rs:1590-1595; qsl-eval/src/simulation/model/walk.rs:218-238 |
| FND-002 | low | `a_completed_partial_order_proof_settles_as_a_reduced_proof` asserts FR-160-AC-1's partial-order row (§7.2's instance settles `Proved{Reduced{[PartialOrder{BreadthFirstRevisit}]}, Uncertified}`), but its tag claims only FR-160-AC-7. No test in the repository carries FR-160-AC-1, so the matrix shows that criterion uncovered. Add FR-160-AC-1 to the tag. | qsl-analyze/tests/it/partial_order.rs:1094-1117 |
| FND-003 | low | FR-162-AC-4 is claimed by no test. Its second sentence ("Two runs of AC-3 retain equal edge lists") is asserted in `symmetry_and_partial_order_reduce_together_on_canonical_states` (`graph.edges() == again.edges()`), which claims only FR-162-AC-3. The PR says the `max_states` 5 half is "the existing constraint test", but that test carries no FR-162-AC-4 tag either. | qsl-analyze/tests/it/partial_order.rs:983-1056 |

## Dispositions

Round 1, reviewed at `4afad2ca6e5572595130bdc82f936cfc0642793a`.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 4afad2ca6: `the_candidate_holds_transitions_over_objects_no_state_holds_yet` claims FR-155-AC-7. It asserts that the candidate seeded by `spawn(y)` holds `touch(d)` for the absent `d`, and that the candidate seeded by `cancel(x)` holds `probe(y, d, x)`. It fails with the absent rows removed. The subject is the `makers` analogue, not the AC's `Counter`/`bump(d)` unit, but it is the same clause. |
| FND-002 | fixed | 4afad2ca6: `a_completed_partial_order_proof_settles_as_a_reduced_proof` now claims FR-160-AC-1. That is honest for the partial-order row the test asserts. AC-1's symmetry rows stay with their own tests. |
| FND-003 | fixed | 4afad2ca6: FR-162-AC-4 is claimed by `symmetry_and_partial_order_reduce_together_on_canonical_states`, which asserts equal edge lists over two runs, and by `boundary_states_count_toward_max_states`, which asserts `max_states` 5 with boundary states counted. Both claims are honest. |
