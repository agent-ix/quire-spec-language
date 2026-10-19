---
id: SR-13072
title: "Gap analysis of QSL-551 part 1 FR-175 lockstep tuple evaluator against FR-175 AC-1..3 and TC-600 (PR #1021)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@429b36783; spec/functional/FR-175-evaluate-a-body-over-a-tuple-of-behaviours.md, spec/test-cases/TC-600-evaluate-a-body-over-a-tuple-of-behaviours.md, qsl-eval/src/temporal/tuple.rs, qsl-eval/tests/it/hyper_tuple.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-175
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-600
    type: reviews
---
# SR-13072: Gap analysis of QSL-551 part 1 FR-175 lockstep tuple evaluator against FR-175 AC-1..3 and TC-600 (PR #1021)

## Summary

Ticket: QSL-551 (part 1, FR-175 only; FR-176/182/183 are deferred to a follow-up, per the PR body). `quire matrix --scope .` (quire 0.36.2, engine 0.50.2; quoin 0.28.3) lists FR-175-AC-1..3 as tagged: AC-1 at hyper_tuple.rs:181, AC-2 at :195 and :219, AC-3 at :242. The refusal test carries `TC-600` with no AC. No spec file changed, so FR-171..174/180 are not touched; FR-172 is read only as the source of `CheckedHyperClause`. The coder's note on FR-175-AC-2 was assessed: the spec wording is at fault, not the evaluator (FND-001). Plan completion: not assessed.

## Verdict

**Gaps.** AC-3 is met (stutter flags at joint positions 2 and 3, the terminal read, and the interval). AC-1 is met on synthetic observations rather than ADR-023 §8.1's fixture (FND-004). AC-2 cannot be met as written, because its counterexample does not exist (FND-001, a spec defect for the planner, not this PR). Its tests also miss the case where the evaluator is wrong (FND-003, see SR-13071 FND-001).

FR-175-AC-2 assessment: for two loops of lengths 2 and 3, joint-loop offset k reads loop positions (r + k) mod 2 and (s + k) mod 3. The equalities at offsets 0..3 join all five loop observations x_r-y_s, x_{r+1}-y_{s+1}, x_r-y_{s+2} and x_{r+1}-y_s into one connected component. So if the outputs are equal at offsets 0..3, they are equal everywhere, and they can never first differ at offset 4 (the fifth position) or offset 5. This holds for every stem, not only for pure loops. The latest first difference possible is offset 3, for example a=[0,1] and b=[0,1,0] at joint position 3. That position is the one a reading over only the longer loop (period 3) never reaches, so it is the version that exercises the period of 6.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Spec defect: FR-175-AC-2's counterexample cannot exist. With loop lengths 2 and 3, the equalities at joint-loop offsets 0..3 connect all five loop observations, so outputs that are equal there are equal at every offset. No pair of such lassos first differs at the fifth position of the joint loop, with any stems. TC-600 Expected Results Step 2 repeats the claim. Reword both to "first differ at the fourth position of the joint loop (offset 3), which a reading over the longer component loop alone never reaches". Route to the planner via Linear; it is not fixed in this code PR. | spec/functional/FR-175-evaluate-a-body-over-a-tuple-of-behaviours.md:84 |
| FND-002 | medium | `a_tuple_of_two_and_three_position_loops_is_read_over_a_joint_period_of_six` reinterprets the AC. Its first difference is at joint position 4, which is joint-loop offset 1 after a stem of 3. A reading with period max(2,3)=3 also finds that difference, so the violated half does not show the period-6 reading (only the holding half's `joints.max() == 8` does). Once FND-001 is fixed in the spec, use a pair that first differs at joint-loop offset 3. | qsl-eval/tests/it/hyper_tuple.rs:195-215 |
| FND-003 | medium | `a_past_operator_gives_the_same_value_at_a_loop_position_and_its_unrolled_copy` uses two loops of length 2, not the 2-and-3 pair TC-600 Step 2 names. It compares only `Holds` versus not (`value == Holds`), never `trace_position`. Its `once[1,1]` read is never true only in an unrolled copy. So it passes while the compact and unrolled encodings disagree on `trace_position` for a past-operator body (SR-13071 FND-001: `Violated{at:1}` against `Violated{at:2}`). Compare the full outcomes, including `at`, on a case whose violation needs loop history. | qsl-eval/tests/it/hyper_tuple.rs:217-235 |
| FND-004 | medium | The AC-1 test does not use ADR-023 §8.1's fixture. It feeds hand-picked observations ([0,5] against [0,6] and [0,5]) through a counters model. There is no `a` from (0,0) or `b` from (1,0) each taking `step(0)`, and no secure model. It does show the lockstep read and `trace_position` 1, but the AC's named counterexample and its secure-model `true` are not exercised. Either drive the vault traces, or reword the AC to a synthetic pair. | qsl-eval/tests/it/hyper_tuple.rs:179-189 |
| FND-005 | low | FR-175 Outputs names an undefined result at the first joint position whose atom is undefined, with its cause. No tuple test returns `AtomReading::Undefined`, so that branch is not exercised through `Lockstep`. No AC requires it, so this is low. | spec/functional/FR-175-evaluate-a-body-over-a-tuple-of-behaviours.md:47-52 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | `locating_the_false_position_charges_no_second_evaluation` is tagged FR-175-AC-2, but it verifies FR-175's "Work" rule. No AC states that rule. The tag inflates AC-2's binder count in the matrix. Tag it `TC-600` alone (as the refusal tests are), or get the planner to add a Work AC. | qsl-eval/tests/it/hyper_tuple.rs:339-341 |
| FND-007 | low | `an_atom_with_no_value_makes_the_body_undefined_at_the_first_such_joint_position` is tagged FR-175-AC-2, but it verifies FR-175's Outputs undefined branch. No AC states that branch. Tag it `TC-600` alone, or get the planner to add an AC. | qsl-eval/tests/it/hyper_tuple.rs:371-373 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | A spec defect, not fixable in this code PR. It went to the planner as a QSL-551 comment (2a15bff7) asking for a spec ticket under QSL-370 that rewords FR-175-AC-2 and TC-600 Step 2 to joint-loop offset 3. The new AC-2 test documents the mismatch and tests offset 3. |
| FND-002 | fixed | fixed at dc63ebd86 |
| FND-003 | fixed | fixed at dc63ebd86 |
| FND-004 | fixed | fixed at dc63ebd86 |
| FND-005 | fixed | fixed at dc63ebd86 |
