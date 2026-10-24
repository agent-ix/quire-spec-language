---
id: SR-13602
title: "Gap analysis of QSL-554 E14c2 PR #1121 against FR-178 (AC-1..AC-6) and TC-603"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b45e62a97; PR #1121 (git diff origin/main...b45e62a97): qsl-eval/src/hyper/{projected,product,joint,mod,witness}.rs, qsl-eval/src/temporal/{tuple,outcome,mod}.rs, qsl-eval/src/simulation/model.rs, qsl-analyze/src/model_check/hyper.rs, qsl-analyze/src/model_check/hyper/projected.rs, qsl-replay/src/model_check/{hyper_counterexample,hyper_outcome,hyper_replay,replay}.rs, qsl-replay/src/spine/clause.rs, qsl-analyze/tests/it/{hyper_projected,hyper_world,hyper}.rs, qsl-eval/tests/it/hyper_tuple.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-178
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-603
    type: reviews
---
# SR-13602: Gap analysis of QSL-554 E14c2 PR #1121 against FR-178 (AC-1..AC-6) and TC-603

## Summary

Ticket: QSL-554 (E14c2). Tools: `quire matrix --scope . --format tsv` (quire 0.36.2, engine 0.50.2), exit 0. Plan completion: not assessed.

Matrix at b45e62a97:

| AC | Status | Bound by | Honest? |
| --- | --- | --- | --- |
| FR-178-AC-1 | tagged | hyper_projected.rs:92 | yes |
| FR-178-AC-2 | tagged | hyper_projected.rs:119 | yes, on a fixture the AC does not name (FND-001) |
| FR-178-AC-3 | tagged | hyper_tuple.rs:621, :635, :651, :666, :683 | :621, :635, :651 (FND-003) |
| FR-178-AC-4 | tagged | hyper_projected.rs:204, :229, :244 | :204 only (FND-001, FND-003) |
| FR-178-AC-5 | tagged | hyper_projected.rs:261, :294 | yes |
| FR-178-AC-6 | tagged | hyper_projected.rs:324 | engine half (FND-006) |

Every AC has a primary test that asserts what the AC states and can fail: each one's assertions are killed by at least one mutant (code review SR-13601). AC-1 asserts the lockstep refutation and the 18-state exhaustive hold. AC-2 asserts the kind, the instance, `trace_position` 1, the lengths 1 and 2, equal visible counts, every mark against its operation, and the re-executed outputs. AC-3 asserts `false` and `Pending`. AC-4 asserts `BoundReached{depth: 1}` and equal counterexamples. AC-5 asserts the cause, the locus, the position and the unmarked paths, after skip moves too. AC-6 asserts `MatchUndetermined` for an incomplete and a refused `match`.

The oracle and the mutation results are in SR-13601. Spec follow-ups (spec questions, not code defects):

- The FR-178-AC-2, AC-4 and TC-603 fixture, and 'total lengths free' in ADR-023 PA-6 and QSpec FR-398 (gap FND-001).
- FR-178 should say whether HM-7 `VacuousMatch` applies to HP-6 (judgement call 2).
- FR-183 should say how an HP-6 `Undefined` counterexample is replayed: projected positions and skipped steps (code FND-001).
- FR-178 should state the tie rule between a rejecting state, an undefined letter and an undefined `μ_U` at one depth. HV-8's 'first product state in breadth-first order' is what the code does.

Declared out of scope, and confirmed not built: FR-183's HP-6 replay (PA-7; refused `HyperProjected`), FR-182's settlement, and `Holds{Reduced}` under FR-174.

## Verdict

**Mergeable for coverage once FND-001's spec fix is routed.** FND-001 is a spec contradiction to send to /specify, not a code change. The rest are low.

- **FND-001 (medium).** FR-178-AC-2 and AC-4 cannot hold on §13's idle vault. The PR's mid-operation start is the right workaround, but the ACs and ADR-023 PA-6 must name it and say 'total lengths free'.
- **FND-002 to FND-006 (low).** The rejecting-state confirmation is untested (P09, P16 and P21 survive); four tests are over-tagged; the HV-8 tie, a bad position 0, two instances and two aliases are untested (P13, P14, P30, P28 and P27 survive); the horizon-refusal test is weak; and the AC-6 test is a seam stub.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The spec is contradictory: FR-178-AC-2 and AC-4 cannot hold on ADR-023 section 13's vault as section 13 defines it, idle starts. FR-178 makes the counterexample the first breadth-first path to a rejecting state. From idle starts no `mix` is enabled, so that path is one visible move of each run at depth 1: equal total lengths, against AC-2. AC-4's `max_depth` 1 then finds it and returns `Violated`, not `BoundReached{depth: 1}`. The PR's `from_idle_starts_the_leak_is_one_visible_step_away` and `a_counterexample_at_the_horizon_is_found_and_one_beyond_it_is_not` show both facts, and the oracle confirms them. The author's mid-operation start (the secret-1 run starts with `busy` set, so its first step is a skipped `mix`) is the smallest fixture that satisfies both ACs, and the tests are sound. But the tests tagged AC-2 and AC-4 check a fixture the ACs do not name. The root cause is ADR-023 PA-6 and QSpec FR-398, which say 'the traces of unequal total length' as if every projected counterexample had it. QSpec FR-400 says 'total lengths free'. Spec fix, needing /specify: (1) in FR-178-AC-2, AC-4 and TC-603's fixtures, name the start, for example '§13's vault with the secret-1 run starting busy (h = 1, busy = 1)'; (2) in PA-6, FR-178 Outputs and QSpec FR-398 (Meaning, AC-2), write 'total lengths free, unequal when a trace takes skipped steps', as FR-400 does. Not a coder edit. | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md:AC table |
| FND-002 | low | No test can see the confirmation of a rejecting state through the three-valued evaluator, and its new stopped path has no test. Mutants P09 (confirmation removed), P16 (the confirmation reads skip-move states as projected positions) and P21 (an evaluator that runs out of meter during confirmation is ignored) all survive the suite. `FirstHit::Stopped`, the new path that turns that case into `Stopped{EvaluationMeter}`, has no test. `from_idle_starts_the_leak_is_one_visible_step_away` says in its doc that the engine confirms every rejecting state, but it asserts nothing about confirmation. Add a test with a counting `HyperLetters` decorator that asserts the evaluator re-read the projected prefix positions, and one whose letters return `Incomplete` on the re-read, which must give `Stopped` with `EvaluationMeter` and never `Violated`. | qsl-analyze/tests/it/hyper_projected.rs:180-200 |
| FND-003 | low | The ACs are over-tagged. `an_atom_with_no_value_in_a_prefix_is_undefined_at_its_projected_position` and `a_prefix_of_no_position_is_refused` are tagged FR-178-AC-3, but AC-3 states only the false and the no-false prefix. They check FR-178 'Three-valued prefix evaluation' and the evaluator's empty-trace contract. `a_counterexample_at_the_horizon_is_found_and_one_beyond_it_is_not` and `a_run_that_completes_inside_its_horizon_holds` are tagged FR-178-AC-4, but they check FR-178's `max_depth` bullet on the idle fixture, not AC-4's statement (AC-2's request at depth 1, and determinism). The matrix lists 5 binders for AC-3 and 3 for AC-4. Tag these four TC-603 only, as #1115's fix round did for the same pattern (SR-13596 FND-003 to FND-006). | qsl-eval/tests/it/hyper_tuple.rs:664-694; qsl-analyze/tests/it/hyper_projected.rs:225-256 |
| FND-004 | low | Several HP-6 paths have no test. No test has a rejecting monitor state and an undefined `μ_U` at one depth, so the HV-8 tie (the first product state in breadth-first order) is unpinned: P13 and P14 survive, and either flips the verdict kind on such a tie. No test has a body that is already false at projected position 0 (P30, a rejecting initial state read as accepting, survives), two instances (P28 survives) or two aliases (P27 survives). The oracle covers position 0 (it has `P0` premises) and saw 25 ties in 1,540 cases. It accepts either kind on a tie, because it does not replay the engine's sort order. Add one fixture for each: a tie at one depth with the expected winner asserted, an initial pair that already differs in `l` (traces with no steps, `trace_position` 0), two vault instances, and an operation of a second alias in the skip list. | qsl-analyze/tests/it/hyper_projected.rs:1-378 |
| FND-005 | low | The `HorizonNotDecided` refusal is tested only for an HP-2 clause, by `a_lockstep_item_with_a_horizon_is_refused`, which is untagged and asserts that the message `contains("HP-6")`, not the variant. An HP-3 item with a horizon has no test. Match `ModelCheckRefusal::Hyper(HyperRefusal::HorizonNotDecided { form: Some(HyperForm::Universal) })`, and add the `ForallExistsSafety` case. The refusal is the PR's own, with no AC, so a TC-603 tag is optional. | qsl-analyze/tests/it/hyper_projected.rs:343-362 |
| FND-006 | low | FR-178-AC-6 asks for a `match` conjunct that calls a function whose evaluation exhausts its meter. The test swaps the whole `StepMatch` for the `Exhausted` stub, which charges a zero meter, so no conjunct, no function and no evaluator meter path runs. This is the same seam limit as #1115's FR-177-AC-6, because S3 does not lower hyper `match` blocks yet. #1115 fixed it by saying in the test doc that it covers the engine half. Do the same here. | qsl-analyze/tests/it/hyper_projected.rs:318-341 |

## Dispositions

Round 1 at c711805a1 (the fix commit on top of main with #1116 merged). Delta reviewed: `git diff 6f1e6af3a c711805a1`. Gates all pass: fmt, clippy -D warnings, the four crates, root `it` (975 passed), string-edge and the three arch-lints; 4,878 tests passed, 0 failed. The HP-6 oracle re-run agrees on 1,540 of 1,540 cases (seeds 7, 11, 23, 41), with 0 mismatches, and every `Projected` (1,216) and `Undefined` (84) counterexample is now refused by replay as `HyperProjected`. Mutation: 31 mutants (the 27 of round 0 plus N1 to N4 on the new checks), 28 killed. P09, P13, P14, P16, P21, P27, P28 and P30 are now killed. 3 survive: P24 and P25 are equivalent, as in round 0, and P26 (the `Projected`-kind refusal removed) is now covered by the `align skip` refusal for every counterexample the engine emits; it differs only for a `Projected` counterexample handed in with a lockstep clause, which no test replays.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | A spec change (FR-178-AC-2 and AC-4, TC-603's fixture, ADR-023 PA-6, QSpec FR-398's 'unequal total length'). It needs /specify, not this PR's code. No ticket id exists yet because Linear refuses new issues, so the lead owns routing it. The PR's tests are correct against the intended fix. |
| FND-002 | fixed | A read-logging HyperLetters test asserts the evaluator re-reads the projected positions and never the skip-move state, and an Incomplete-on-re-read test asserts Stopped{EvaluationMeter}. Mutants P09, P16 and P21 killed. The from_idle doc no longer claims confirmation. |
| FND-003 | still-open | Three of the four are clean: the two horizon tests and a_prefix_of_no_position_is_refused now carry TC-603 alone, and `quire matrix` binds FR-178-AC-4 to hyper_projected.rs:227 alone. But an_atom_with_no_value_in_a_prefix_is_undefined_at_its_projected_position (hyper_tuple.rs:665-670) is tagged TC-603 while its doc still opens `/// FR-178-AC-3, undefined: ...`. `quire matrix` still binds FR-178-AC-3 to hyper_tuple.rs:670 by that prose citation, the same mechanism #1115's round 1 hit. Reword the doc so it does not open with the AC id, for example 'FR-178 Three-valued prefix evaluation, undefined: ...'. |
| FND-004 | fixed | Tests added for the HV-8 tie in both orders, a body false at projected position 0 (empty traces, trace_position 0), two instances (pinned at 174 product states, and a budget of exactly 174) and the other-alias operation text. Mutants P13, P14, P27, P28 and P30 killed. |
| FND-005 | fixed | It matches HyperRefusal::HorizonNotDecided{form} for both HP-2 (Universal) and HP-3 (ForallExistsSafety), tagged TC-603. P10 killed. |
| FND-006 | fixed | The doc states it covers the engine half with a seam stub, as #1115 did. |
