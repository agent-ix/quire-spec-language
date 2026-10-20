---
id: SR-13596
title: "Gap analysis of QSL-553 E14c1 PR #1115 against FR-177 (AC-1..AC-6), FR-184 (AC-1..AC-3) and TC-602"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@5198fd7ba; PR #1115 (git diff origin/main...5198fd7ba): qsl-eval/src/hyper/{joint,witness,product,seam,mod}.rs, qsl-eval/src/automaton/{monitor,translate}.rs, qsl-analyze/src/model_check/hyper.rs, qsl-replay/src/model_check/{limits,hyper_*,replay}.rs, qsl-semantics/src/check/hyper_clause.rs, qsl-analyze/tests/it/hyper_witness.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-177
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-184
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-602
    type: reviews
---
# SR-13596: Gap analysis of QSL-553 E14c1 PR #1115 against FR-177 (AC-1..AC-6), FR-184 (AC-1..AC-3) and TC-602

## Summary

Ticket: QSL-553 (E14c1). Tools: `quire matrix --scope . --format tsv` (quire 0.36.2, engine 0.50.2), exit 0. Plan completion: not assessed.

Matrix at 5198fd7ba:

| AC | Status | Bound by | Honest? |
| --- | --- | --- | --- |
| FR-177-AC-1 | tagged | hyper_witness.rs:94, :501, :579, :702 | :94 only (FND-003) |
| FR-177-AC-2 | tagged | :154, :536 | :154 only (FND-004) |
| FR-177-AC-3 | tagged | :188 | yes |
| FR-177-AC-4 | tagged | :234, :551 | :234 only (FND-005) |
| FR-177-AC-5 | tagged | :270, :367, :420, :462 | :270 only (FND-002, FND-006) |
| FR-177-AC-6 | tagged | :345 | engine half (FND-009) |
| FR-184-AC-1 | tagged | :592 | no, partial (FND-001) |
| FR-184-AC-2 | tagged | :631, :665 | yes |
| FR-184-AC-3 | untagged | - | yes; HP-1 not built |

Every AC's primary test is truthful: each of AC-1 to AC-6 and FR-184-AC-2 has one test that asserts exactly what the AC states. AC-1 asserts the 6 states, the 2 automaton states, `X_0`/`X_1`, the position and the lasso. AC-2 asserts the 4 singleton sets and `Holds`. The trouble is the extra tags: nine tests carry an AC tag for behaviour the AC does not state, so the matrix overstates FR-177's coverage and claims FR-184-AC-1 whole.

The oracle and the mutation results are in SR-13595. Spec follow-ups (spec questions, not code defects):

- FR-177 should state the tie rule between an undefined evaluation, a refused `match` and an empty `X` at one joint position.
- FR-177 should define the canonical universal lasso, including its rotation.
- FR-177 should give a verdict for an existential member whose contract evaluates undefined or refused.
- FR-177 should say what to return for an undefined `μ_U` or universal contract when `X` is empty.
- FR-177 should say how FR-126's binding ranking applies to HP-3.
- FR-184's `count` means size-reached + 1 in AC-2 but equals the value in AC-3; one meaning should be chosen.

Declared out of scope, and confirmed not built: FR-184-AC-3 (no HP-1 engine; `max_relation_tuples` has no consumer), FR-182's terminal-record marking, FR-183's witness-set recomputation, replay counting `max_witness_set`, and `Holds{Reduced}` under FR-174.

## Verdict

**Not mergeable as is.**

- **FND-001 and FND-002 (medium).** Two bindings claim an AC the test does not meet, or contradicts.
- **FND-007 (medium).** No test covers the connect path behind `position`; mutant M10 survives the suite.
- **FND-003 to FND-006, FND-008 and FND-009 (low).** Tag inflation and the untested HP-3 paths.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `quire matrix` shows FR-184-AC-1 as tagged by `a_request_that_sets_neither_budget_runs_under_the_published_defaults`. The AC's second half, a terminal record that reports both values marked as published defaults, is not built (it is FR-182's). The test checks only `Default` and `hyper_used()`. The PR body says this half is not covered, but the tag makes the matrix say the whole AC is. Repo precedent (SR-13451 on #1092) leaves a partly covered AC untagged. Tag TC-609 only, or split the AC. | qsl-analyze/tests/it/hyper_witness.rs:590-627 |
| FND-002 | medium | `an_undefined_universal_match_of_an_hp3_clause_is_refused_not_guessed` is tagged `FR-177-AC-5`, but it checks the opposite of what FR-177's undefined-evaluation rule asks for. It asserts a `NoPartnerPath` refusal for an undefined universal conjunct, and AC-5 concerns an existential conjunct that returns `Violated`. The binding makes the matrix count a contradiction of the AC's rule as coverage of it. See code-review SR-13595 FND-003. | qsl-analyze/tests/it/hyper_witness.rs:365-385 |
| FND-003 | low | Three tests are tagged `FR-177-AC-1`, but they verify other FR-177 or FR-183 behaviour: `a_universal_variables_fairness_set_shapes_the_lasso` (fairness scoping), `the_leaky_request_run_twice_gives_equal_outcomes` (determinism) and `a_witness_exhausted_counterexample_is_refused_by_replay_until_fr_183` (a replay refusal). AC-1 states the unfair leaky lasso, the 6 states and `X_0`/`X_1`. The matrix lists 4 binders for AC-1 where 1 checks it. Tag these with TC-602 (or the FR) only. | qsl-analyze/tests/it/hyper_witness.rs:499-533, 577-588, 700-713 |
| FND-004 | low | `an_unsatisfiable_universal_match_is_vacuous` is tagged `FR-177-AC-2` (the secure vault holds over 4 states). It checks ADR-023 HM-7 vacuity, which FR-177 does not state. This tag inflates coverage. The HM-7 reading for HP-3 is the PR's own judgement call 6, and it has no AC. | qsl-analyze/tests/it/hyper_witness.rs:534-547 |
| FND-005 | low | `a_cancelled_run_stops_with_its_source` is tagged `FR-177-AC-4` (`max_witness_set` 0). It checks cancellation. This tag inflates coverage. | qsl-analyze/tests/it/hyper_witness.rs:549-575 |
| FND-006 | low | `a_letter_with_no_value_ends_the_item_with_a_path_for_the_partner` and `a_letter_with_no_value_at_position_zero_ends_the_item_with_empty_paths` are tagged `FR-177-AC-5`. They check an undefined body atom read through a stub `HyperLetters`, not AC-5's existential `match` conjunct. The behaviour is FR-177's undefined-evaluation bullet, which has no AC of its own. Tag TC-602 only. | qsl-analyze/tests/it/hyper_witness.rs:387-497 |
| FND-007 | medium | No test has an empty-`X` state whose fair cycle is reached through a connecting path, as opposed to lying on the cycle. Mutant M10, which reports `position` as the stem length (tree path plus connecting path), survives every PR test. The reviewer oracle kills M10 at once: 62 of 300 random cases disagree on `position` and the first empty `X`. In every TC-602 fixture the first empty state is already in its fair component, so `connect` is always empty. Add one fixture where it is not, for example a reset-only loop after the exhaustion with weak fairness on `reset`. | qsl-analyze/tests/it/hyper_witness.rs:1-713 |
| FND-008 | low | Several HP-3 paths have no test: an existential member's contract that evaluates refused or undefined (mutants M13 and M21 survive), more than one instance (M20 survives), strong-fairness refinement in the universal filter (M17 survives), a universal contract that is undefined in a witness state, two universal variables, and `max_automaton_states` on the witness product. These are the paths behind code-review FND-002 and FND-006. The reviewer oracle covers weak whole constraints with one universal variable, or none. | qsl-analyze/tests/it/hyper_witness.rs:1-713 |
| FND-009 | low | FR-177-AC-6 asks for a `match` conjunct that calls a function whose evaluation exhausts its meter. The test replaces the whole `StepMatch` with a stub whose `existential` charges a zero meter, so no conjunct, no function and no evaluator meter path runs. Production has no `StepMatch` implementor yet, because S3 does not lower hyper `match` blocks. The HP-2 suite uses the same stub pattern, so this is the seam's limit, not a defect of this PR alone. Say so in the test doc, or tag the test TC-602 only until the evaluator seam exists. | qsl-analyze/tests/it/hyper_witness.rs:301-363 |

## New findings (disposition pass 1)

Re-review at 952d953d5. The oracle was re-run at 952d953d5. The first oracle covered 1,300 cases (seeds 7, 11 and 23) with 0 mismatches. A second oracle, new this round, adds undefined `μ_U` and `μ_E` conjuncts (division by `i` = 0) and checks each Undefined counterexample: its position; the locus; `variables`/`state_keys`; trace lengths (`μ_U`: existential path one step short); the partner tuple being a member of X at every position; and the evaluation being undefined exactly at the last step. That gave 1,300 cases with 0 mismatches: 396 stated Undefined, 11 refused `NoPartnerPath` (every candidate at the first position had an empty X) and 7 mixed ties.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | Three round-1 paths have no test. (1) A later instance beating an earlier instance's position-0 empty X with a position-0 finding of lower rank: mutant N2 (`Some((0, _, _)) => break`) survives. (2) An Unstated undefined evaluation tied with an empty X at one position: mutant N3 (Unstated ranked 3) survives. (3) An undefined `μ_U` with two universal variables, the shape behind code-review FND-010. The other round-1 mutants are killed: N4 (sooner) by a_state_keeps_its_earliest_refused_finding, N5 (name every variable) by an_undefined_universal_match_with_a_witness_ends_the_item_with_the_partners_path, and N6 (no cap) by a_violation_at_position_zero_bounds_the_later_instances. N1 (no break at a rank-0 finding at position 0) is equivalent and only costs exploration. M14 and M18 are equivalent, as in round 0. | qsl-analyze/tests/it/hyper_witness.rs:918-960 |

## Dispositions

Round 1, at 952d953d5. 

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The #[trace] now reads TC-609 only, but the doc comment opens `/// FR-184-AC-1, its first half: ...` (hyper_witness.rs:732), and `quire matrix` still binds FR-184-AC-1 to hyper_witness.rs:738 by that prose citation, the same mechanism SR-13451 recorded on #1092. The matrix therefore still claims the AC. Reword the doc so it does not start with the AC id. |
| FND-002 | fixed | Renamed to an_undefined_universal_match_with_no_witness_is_refused_not_guessed and tagged TC-602 only; the behaviour now follows FR-177 when X is non-empty. |
| FND-003 | still-open | Two of the three are fixed (fairness lasso and replay refusal: TC-602 only, and matrix-clean). the_leaky_request_run_twice_gives_equal_outcomes is tagged TC-602, but its doc opens `/// FR-177-AC-1, last sentence of FR-176-AC-4 applied` (hyper_witness.rs:720), and the matrix still binds FR-177-AC-1 to hyper_witness.rs:724. |
| FND-004 | fixed | Tagged TC-602 only; the matrix binds FR-177-AC-2 to :154 alone. |
| FND-005 | fixed | Tagged TC-602 only; the matrix binds FR-177-AC-4 to :234 alone. |
| FND-006 | fixed | Both letter tests are tagged TC-602 only; the matrix binds FR-177-AC-5 to :270 alone. |
| FND-007 | fixed | Connect-path test added; mutant M10 killed by the_lasso_reaches_its_fair_cycle_by_a_connecting_path. |
| FND-008 | fixed | New tests: two instances (M20 killed), two universals, max_automaton_states, member contract (M13 and M21 killed), and fair_in (M17 killed). Mutation status this round: 30 mutants, 25 killed; 5 survived: M14, M18 and N1 are equivalent, N2 and N3 are untested ranking edges (gap FND-010). |
| FND-009 | fixed | The test doc now states it covers the engine half, because no S3 lowering exists; accepted. |
