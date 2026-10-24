---
id: SR-13082
title: "Gap analysis of QSL-530 E9 part 1 liveness SCC phase against FR-125, FR-126 (TP-4), FR-128, FR-133 (PR #1024)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@31912a26a; spec/functional/FR-125-*.md, FR-126-*.md, FR-128-*.md, FR-133-*.md, spec/decisions/ADR-018-*.md (sections 2-6), spec/test-cases/TC-520, TC-521, TC-523, TC-534, TC-536, TC-537, TC-539; qsl-analyze/src/model_check/liveness.rs, qsl-analyze/tests/it/liveness.rs, qsl-replay/src/model_check/counterexample.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-133
    type: reviews
---
# SR-13082: Gap analysis of QSL-530 E9 part 1 liveness SCC phase against FR-125, FR-126 (TP-4), FR-128, FR-133 (PR #1024)

## Summary

Ticket: QSL-530 (E9), PR 1 of 2. PR 2 is declared to carry lasso replay (FR-128, TC-523/539), the `strong-would-exclude` hint (FR-133, TC-534) and the TP-4 `Holds` certificate (after E8b, QSL-531). `quire matrix --scope .` (quire 0.36.2, engine 0.50.2; quoin 0.28.3; the run prints DuplicateArchetype / DuplicateInverseEdge module warnings and exits 0). The only criterion this PR newly tags is FR-126-AC-3 (`liveness::a_behaviour_that_ends_is_the_lasso_whose_loop_is_the_terminal_stutter_step` and the existing deadlock test), and both tests assert what AC-3 says. The PR claims nothing beyond its scope: replay, the hint and `Holds` are not claimed, and FR-125-AC-8..12 are untouched. Semantic review (intent vs test vs code) was done for the TP-4 path at the dispatcher's request. Plan completion: not assessed.

## Verdict

**FAIL at ticket scope (mechanical: untagged criteria remain), expected for PR 1 of 2.** The PR-1-relevant gaps are FND-001 (TP-4 criteria testable now but untagged, and not listed for PR 2) and the spec gaps FND-003..FND-005, which go to the planner as Linear notes. The coder's six open questions are judged in the Linear comment: (1) Holds before E8b, plan gap (FND-005), the refusal is correct; (2) c = a vs c = b, spec gap (FND-004); (3) binding-first vs one product, spec gap (FND-004) plus a code defect in the budgets (SR-13081 FND-002); (4) `Dead` state, spec gap (FND-003) with a code consequence (SR-13081 FND-003); (5) horizon edges, code agrees with EN-1 except the stutter-lasso length (SR-13081 FND-004); (6) CX-5 ordering, code defect (SR-13081 FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TP-4 criteria that PR 1's code can meet now carry no AC tag, and PR 2's declared list (replay, hint, Holds) does not name them: FR-126-AC-1's two `Violated` sentences (no granularity over the example unit; `each` over the shipped unit at examples/config-version, which exists), AC-4's `Restless` lasso, AC-5's TP-4 horizon and limits over the example (tested here only over Counter, tagged TC-521 alone), and AC-7's determinism over AC-1's requests. The example unit is the shipped unit with ADR-018 section 6's `Cycles` post clause, buildable as a fixture the way the Counter tests build theirs. Either tag them in this PR or add them by name to PR 2's scope on QSL-530. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:352-358 |
| FND-002 | low | Untagged at ticket scope and not touched by this PR: FR-128-AC-1..6 and FR-133-AC-1..3 (declared PR 2), and FR-125-AC-1, 2, 4, 7..12 and FR-126-AC-2, 6, 8, 9 (untagged on main already). Recorded so the ticket is not closed on PR 2 with these still open. | spec/functional/FR-128-replay-a-model-counterexample.md; spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md |
| FND-003 | medium | Spec gap (reverse gap): the code adds behaviour no requirement owns. (a) A `Dead` property state when the negation automaton has no move, explored so every position's letter and contracts are still read (needed by FR-125 "every other claim SHALL read every position" and UE-1), plus an extra all-live acceptance set. FR-126 defines the product state as (model state, automaton state) with FR-338's canonical structural key, which has no dead state, and AC-1 counts 15 product states, which the dead branch changes (SR-13081 FND-003). The planner should say how positions past the negation's death are read and whether they are product states (and in the PC-4 certificate). (b) `LivenessRefusal::{StrongFairness, GuardedActivation, UnknownOperation, ComponentCertificateNotBuilt}`: FR-126's `ModelCheckRefusal` lists only the admission failure and `RequiresBound`. | qsl-analyze/src/model_check/liveness.rs:77-105, 259-272, 1143-1154 |
| FND-004 | medium | Spec gap: no rule ranks counterexamples across `over` instances. FR-126-AC-1, TC-521 Step 1 and ADR-018 section 6 expect the no-granularity lasso "bound to c = b", but `c = a` is violated as well (a loop of `upd(b)` keeps va at 0), and FR-125 orders instances a then b. The PR checks one product per instance in instance order, so it would return `c = a`. The same choice changes UE-4's ranking: an undefined letter at depth 5 for the first instance is returned before one at depth 1 for the second, while the TP-1 engine (check.rs) reads every instance at each state and returns the shallowest. The planner should state the cross-instance order (instance order first, or shortest first across instances) and correct AC-1 / TC-521 / ADR-018 section 6 to it. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:352; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:413; qsl-analyze/src/model_check/liveness.rs:607-635 |
| FND-005 | low | Plan gap: FR-126's TP-4 `Holds` needs FR-339's `ComponentCertificate` (E8b, QSL-531), but QSL-530 is blocked only by QSL-528 and QSL-524. Add QSL-531 as a blocker of the `Holds` half (or split it to its own ticket) so PR 2 is not started before it can finish. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:334-341 |
| FND-006 | low | The TP-4 path skips `ModelCheckItem::instances`, which the TP-1 path runs: no `ClauseIdentity::resolve` (clause node and occurrence key, the `over` parameter for the replay `ContextBinding`) and none of its `ClauseRoot` refusals. CX-2 puts the clause node and occurrence key in the FR-070 envelope that FR-128 replays, so PR 2 needs it. Note for PR 2, not a defect of PR 1's outcomes. | qsl-analyze/src/model_check/liveness.rs:130-162; qsl-analyze/src/model_check/check.rs:241-247 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | d5f75958d adds `example_units.rs` over examples/config-version and the Health unit. FR-126-AC-1's two `Violated` rows, AC-4's Restless lasso and AC-7's determinism are tested and tagged TC-521. AC-1 and AC-4 stay untagged only because their `Holds` rows wait on QSL-895. AC-5 is tagged on the example run. |
| FND-002 | deferred | PR 2 (FR-128 replay, FR-133 hint) and QSL-895 (Holds); the other untagged criteria predate this PR. |
| FND-003 | deferred | STD-203: the planner is adding an AC for positions read past the negation's death, and for the `LivenessRefusal` variants. The `Dead` state itself is gone; its replacement pass is SR-13081 FND-009/FND-010. |
| FND-004 | deferred | STD-203 rules that the cross-binding ranking is the shortest counterexample, with ties going to canonical binding order. The PR lands binding-first as an interim, and `example_units.rs` asserts the symmetric shape without pinning `c = b`. |
| FND-005 | deferred | The Holds wiring is QSL-895, which is blocked by QSL-531 (E8b). |
| FND-006 | fixed | d5f75958d: the liveness path resolves `ClauseIdentity` as the safety path does; `ClauseRoot` refusals and root cancellation are mapped. |
