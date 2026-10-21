---
id: SR-13625
title: "Gap analysis of QSL-551 PR #1128 against FR-182, FR-183 and FR-184 (hyper settlement, WitnessExhausted replay, max_depth)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@6d17c8704; PR #1128 (git diff origin/main...6d17c8704): qsl-replay/src/model_check/{hyper_settle,hyper_replay,hyper_outcome,outcome,replay,settle}.rs, qsl-analyze/src/model_check/hyper.rs, qsl-analyze/src/model_check/hyper/projected.rs, qsl-analyze/tests/it/{hyper_settle,hyper_replay,hyper_witness,hyper_world,hyper,hyper_projected}.rs; spec/functional/FR-{176,177,182,183,184}, ADR-023 HV-3"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-183
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-184
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: reviews
---
# SR-13625: Gap analysis of QSL-551 PR #1128 against FR-182, FR-183 and FR-184 (hyper settlement, WitnessExhausted replay, max_depth)

## Summary

Ticket: QSL-551 (E14 remainder). Tools: `quire matrix --scope . --format tsv` (quire 0.36.2, engine 0.50.2), exit 0. Plan completion: not assessed.

Matrix at 6d17c8704:

- **Tagged by this PR, as the PR body claims:** FR-182-AC-8 (hyper_settle.rs:205), FR-183-AC-1 (hyper_replay.rs:81, :256), FR-183-AC-4 (hyper_replay.rs:190, :277), FR-183-AC-6 (hyper_witness.rs:1250). FR-184-AC-2 stays tagged (hyper_witness.rs:769).
- **Untagged, and declared not delivered in the PR body:** FR-182-AC-1 to AC-7 and AC-9, FR-183-AC-2, AC-3, AC-5 and AC-7, FR-184-AC-1 and AC-3, FR-176-AC-1 and AC-2. The reasons hold: `proved` rows need FR-163 (QSL-908), AC-4 to AC-6 and FR-184-AC-3 need HP-1, AC-9 needs an empty-universe run, FR-183-AC-2/3/5/7 need the Projected, StepTuple and HP-6 replays, and FR-184-AC-1's proved case needs a `proved` record. The settlement rows that are built are tested under TC-607 and TC-609 only, which is honest.
- FR-177-AC-7 to AC-12 are untagged on main as well; they are outside this PR.

Bindings checked: the four new AC tags assert what their criteria say, with the caveats in FND-005 and FND-006 (assertions looser than the criterion, not wrong tags).

Requirement to code:

- FR-182's rows: V-4 only through replay, V-5 with depth and coverage, V-6 causes including `MatchUndetermined` and `VacuousMatch` and `ReplayParity`/`ReplayRefused`, V-7 for a stopped run and for cancellation, `failed` for a replay fault, the method, and the five limits with their origin. All built through FR-127's row builders. `Holds` is refused (`ProductClosureCheckerNotBuilt`), not settled.
- FR-183 `WitnessExhausted`: built, with two defects found by the code review (SR-13624 FND-001, FND-002).
- FR-184: the engine stop's count is one past the budget; the replay stop has none (SR-13624 FND-004).
- `max_depth` for HP-2 and HP-3: built; no AC exists (PR spec gap 1); HP-3's V-5 coverage is not what HV-3 states (SR-13624 FND-003, FND-001 here).

Mutation results (33 mutants, 25 killed, 8 survived, 2 equivalent) are in SR-13624; the 6 non-equivalent survivors are FND-002 to FND-004 here.

## Verdict

**Coverage is honest, but the spec needs two decisions and the tests miss four branches.** Every AC the PR claims is tagged by a test that can fail, and every AC it does not claim is untagged with a stated reason. The spec gaps the PR lists are real. Two more come out of the code review: HV-3's coverage for HP-3, and FR-183's rule for a replay whose path holds an undetermined `μ_E` or a position past a rotated lasso. The surviving mutants show four untested branches of the new replay code and one untested horizon-ranking case.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Spec gap, HV-3 for HP-3: 'no counterexample whose universal lasso has length at most `k`' is not what a product-depth horizon guarantees for HP-3, since FR-177's canonical rotation reports a universal lasso shorter than the product path to the empty set and around its loop (SR-13624 FND-003; 15 of 168 HP-3 refutations in the oracle). ADR-023 HV-3 and FR-182's V-5 bullet need to state the HP-3 bound the engine can meet (product joint steps), or require the engine to cover every universal lasso of at most `k` steps. FR-176 and FR-177 also need a `max_depth` AC (the PR's gap 1). | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:289; spec/functional/FR-182-settle-a-hyper-verdict.md:100-103 |
| FND-002 | medium | Spec gap, FR-183 `WitnessExhausted`: it says to recompute `X_0` to `X_position` 'by FR-177's rule' but does not say (a) that `position` may exceed the rotated lasso's step count and is read around the loop (SR-13624 FND-002), or (b) that a refused or incomplete `μ_E`, an undecided member contract, or an undefined letter on the path settles no value or `inconclusive` rather than shrinking `X` (SR-13624 FND-001). Both need a sentence in FR-183 (and QSpec FR-400's reader rule for (a)). | spec/functional/FR-183-replay-a-hyper-counterexample.md:146-158 |
| FND-003 | medium | Untested replay branches. Mutants R7 (the `Undefined` replay's `μ_U` reads every trace, not the universal ones) and R8 (a `μ_E` that `Fails` on an `Undefined` path ignored) survive: FR-183-AC-6's test has no existential step that `μ_E` rejects and its matcher reads only the first labels. R11 (step to any successor with the same transition, post-states unchecked) survives: no witness replay fixture has a nondeterministic step. Add an `Undefined` HP-3 envelope whose existential step `μ_E` rejects (expect `inconclusive`), a `μ_U` seam that reads its label count, and a nondeterministic `step` (`self.l = 0 or self.l = 1`) whose envelope takes the second post-state. | qsl-replay/src/model_check/hyper_replay.rs:350-379, 589-600, 744-749 |
| FND-004 | medium | A test that does not test what its doc says: `a_witness_exhausted_counterexample_that_breaks_a_replay_rule_is_refused` lists 'a clause that is not forall-exists', but its envelope (two traces both named `a` over the `NonInterference` clause) is refused by the variable-name rule before the form check; mutant R5, the form check removed, survives. Build the envelope with the clause's own variables (`a`, `b`) so only the form differs. | qsl-analyze/tests/it/hyper_replay.rs:397-400 |
| FND-005 | low | FR-183-AC-4 says the two hand-built envelopes settle `inconclusive`, `Verdicts`; the tests assert `Inconclusive(_)`, which a `NoValue` cause also passes. Match `DisagreementCause::Verdicts { .. }`. | qsl-analyze/tests/it/hyper_replay.rs:190-210, 277-300 |
| FND-006 | low | FR-183-AC-1's second sentence names the recomputed sets, `X_0 = {((1, 0), g)}` and an empty `X_1`; the replay test asserts only `Reproduced` at position 1, and the sets are asserted by the FR-177 tests on the engine's product, not on the replay's. Mutant R9 (one position short) is killed, so the empty `X_1` is covered; `X_0`'s member is not. State that in the test doc, or expose the replay's recomputed sets to the test. | qsl-analyze/tests/it/hyper_replay.rs:249-266 |
| FND-007 | low | Untested or unreachable: mutant S10 survives, since no liveness replay returns `ModelTraceSettlement::Stopped`, so the new arm in FR-127's `settle` is reachable only if one does; and mutant E5 (the HP-3 ranking cap takes the larger of ranking and horizon) survives, since no test runs two instances under a horizon. Add a two-instance horizon test for HP-2 and HP-3, and either a test or a comment that the liveness arm exists for exhaustiveness. | qsl-replay/src/model_check/settle.rs:342-345; qsl-analyze/src/model_check/hyper.rs:724-727 |

## New findings (disposition pass 1)

Round 1 re-review at 09480caa4. `quire matrix` is unchanged for the claimed ACs, but the new tests carry AC tags whose criteria they do not test (FND-008 to FND-010). The ACs stay covered by their round-0 tests, so coverage is not inflated, but the bindings are wrong. Three branches of the new replay code are untested (FND-011).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | Wrong trace: three new tests are tagged `FR-183-AC-1`, but none of them tests what AC-1 states, which is §8.1's leaky `Lockstep` and §8.2's leaky `WitnessExhausted{position: 1}` with its sets. They test a rotated lasso read around its loop, a longer or late loop, and a nondeterministic step's post-state. That behaviour has no AC (PR spec gap 0b). Tag them `TC-608` alone. | qsl-analyze/tests/it/hyper_replay.rs:459, 657, 698 |
| FND-009 | low | Wrong trace: four new tests are tagged `FR-183-AC-4`, but they assert `NoValue` for a set that lost a member to a refused or undefined `μ_E`, an undefined member contract, or a universal contract. AC-4 states the two `Verdicts` envelopes and the `max_witness_set` 0 stop. The dropped-member rule has no AC (spec gap 0b). Tag them `TC-608` alone. | qsl-analyze/tests/it/hyper_replay.rs:500, 542, 556, 854 |
| FND-010 | low | Wrong trace: `an_undefined_witness_path_through_a_rejected_existential_step_is_not_reproduced` is tagged `FR-183-AC-6`, but it asserts `Verdicts` for a path through a step that `μ_E` rejects. AC-6 states FR-177-AC-5's counterexample reproducing, and the refusal when the existential trace is removed. Tag it `TC-608` alone. | qsl-analyze/tests/it/hyper_replay.rs:802 |
| FND-011 | low | Three branches of the new `WitnessExhausted` replay are untested. Mutant N4 survives: a refused (not undefined) universal contract on the path is not a drop. Mutant N5 survives: an undefined `μ_U` on a sibling step is not a drop. Mutant N14 survives: a step past the last one always reads the last step, because every engine counterexample in the tests that goes past its steps has a loop whose steps carry the same transition. Add a seam whose universal contract is refused (meter) on the path, a `μ_U` with no value on a sibling step, and a counterexample past its steps around a loop of two different transitions. | qsl-replay/src/model_check/hyper_replay.rs:654-662, 670-682 |

## Dispositions

Round 1, at 09480caa4. Gates, oracle and mutants are in SR-13624's Dispositions.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Spec change, not this code PR. The code takes the conservative reading (`BoundCovers::ProductDepth` for HP-3), and the PR body lists it as spec gaps 0 and 0a. No Linear ticket tracks the HV-3 and `max_depth` AC change yet (`ix-board search` found none), so the lead should file one. |
| FND-002 | deferred | Spec change, not this code PR. The code reads past positions around the loop and settles `NoValue` for a dropped member, and the PR body lists both as spec gap 0b. No Linear ticket tracks the FR-183 sentence yet, so the lead should file one. |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | still-open | New this round: no fix yet. |
| FND-009 | still-open | New this round: no fix yet. |
| FND-010 | still-open | New this round: no fix yet. |
| FND-011 | still-open | New this round: no fix yet. |

What each fix was checked against:

- **FND-003.** Mutants R7, R8 and R11 are killed by the new tests.
- **FND-004.** Mutant R5 is killed. The envelope is now a `Lockstep` clause's own `a`/`b` counterexample.
- **FND-005.** Both FR-183-AC-4 tests match `DisagreementCause::Verdicts`.
- **FND-006.** The test reads `X_0` off the engine's `WitnessProduct` initial state, which is the same code the replay runs, and the test doc says so.
- **FND-007.** Mutants S10 and E5 are killed (an injected liveness stop, and two-instance horizon tests for HP-2 and HP-3).
