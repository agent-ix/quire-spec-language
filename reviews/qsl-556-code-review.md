---
id: SR-12304
title: 'Code review of PR #1131: HP-5 through the possible family (FR-181), code-review + rust-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@62b9d71fd; git diff origin/main...62b9d71fd (PR #1131, branch qsl-556-hp5): qsl-analyze/src/model_check/hyper.rs, qsl-analyze/src/model_check/hyper/single.rs, qsl-analyze/src/model_check/mod.rs, qsl-analyze/src/model_check/state_graph.rs, qsl-analyze/src/model_check/state_graph/sample.rs, qsl-analyze/tests/it/hyper_single.rs, qsl-analyze/tests/it/hyper_world.rs, qsl-analyze/tests/it/main.rs, qsl-eval/src/automaton/mod.rs, qsl-eval/src/automaton/translate.rs, qsl-eval/src/hyper/mod.rs, qsl-eval/src/hyper/product.rs, qsl-replay/src/lib.rs, qsl-replay/src/model_check.rs, qsl-replay/src/model_check/existential_outcome.rs, qsl-replay/src/model_check/state_graph.rs; ticket QSL-556'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-181
  type: reviews
---

## Summary

Ticket: QSL-556. PR: quire-spec-language#1131, head 62b9d71fd. Method: code-review with the rust-review lane folded in. This is a re-review after the merge with main and the fix round for the earlier review (SR-13627 code-review, SR-13628 gap-analysis, posted to QSL-556 only, at 4943555dd). The gap analysis is SR-12305 and the spec review of the FR-181 edits is SR-12306.

Soundness was checked first. No path returns `Witnessed` without a concrete lasso that is fair and satisfies the body, a completed exploration, no open node and no undefined letter. A sampled witness is a real lasso of the model that the clause evaluator reads true and whose loop passes `check_loop`, so it is valid existential evidence and not a statistical one. `Trapped` needs a part with roots and no open member. `Undefined` is a reachable product state. Everything else is `NoDecision` or `WitnessedUnchecked`, which settle no `proved`.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| check_single_existential, Run, Decision | qsl-analyze/src/model_check/hyper/single.rs | examined |
| NotSingleExistential, BodySense::Negated wiring | qsl-analyze/src/model_check/hyper.rs | examined |
| shared phase 0 helpers | qsl-analyze/src/model_check/state_graph.rs, state_graph/sample.rs | examined |
| translate_itself, Selection::Itself, Counting::Every | qsl-eval/src/automaton/translate.rs | examined |
| BodySense, HyperProduct::new | qsl-eval/src/hyper/product.rs | examined |
| ExistentialOutcome, ModelLasso, LassoWitness | qsl-replay/src/model_check/existential_outcome.rs, state_graph.rs | examined |
| re-exports | qsl-analyze/src/model_check/mod.rs, qsl-eval/src/{automaton,hyper}/mod.rs, qsl-replay/src/lib.rs | examined |
| tests and oracle | qsl-analyze/tests/it/hyper_single.rs, hyper_world.rs | examined |
| QSL-923 tableau fix (main) | qsl-eval/src/automaton/formula.rs | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Phase 0 draws the same walks again for every instance. A walk does not depend on the binding: only `witness` reads the instance. With N instances the walk and expansion cost is N times what it needs to be. Draw the walks once per initial state and check each candidate per instance. | qsl-analyze/src/model_check/hyper/single.rs:341-375 |
| FND-002 | low | `Decision::lasso` runs `analyze` over the whole fairness graph once per initial state with no sampled witness, restricted by `within`. A subject with many initial states whose parts overlap pays the SCC decomposition again for each. One decomposition over the expanded, non-open states, and a per-part choice of the first fair accepting component, would do. | qsl-analyze/src/model_check/hyper/single.rs:791-813, 1036-1063 |

## Verdict

Mergeable: no major finding, two low efficiency findings.

Measured at 62b9d71fd:

- `cargo test -p qsl-analyze --test it`: 690 passed, 808.8 s wall. The FR-181 tests (`hyper_single`, 26 tests) pass. `cargo clippy -p qsl-analyze -p qsl-replay -p qsl-eval --all-targets --all-features -- -D warnings` and `cargo fmt --check` are clean.
- Ten hand mutants of single.rs, each run against `hyper_single` with the oracle skipped. All were killed: (A) open states kept in `within`; (B) phase 0's undecided-contract check removed; (C) loop entry at the highest rank; (D) the last fair accepting component; (E) every walk with no candidate counted as stopped; (F) a run with no graph ignoring its sampled witnesses; (G) the trap/Undefined tie given to the trap; (H) each instance given the full `max_states`; (I) `Witnessed` ignoring open states; (J) a trap that ignores closedness.
- I reverted QSL-923's `or` hunk in `Ir::reachable` (qsl-eval/src/automaton/formula.rs) in scratch. The brute-force oracle then fails on `holds(v.l @ b = 1) or always holds(v.l @ b = 0)`: it expects `[true, true]` and gets `Trapped`. So the new disjunctive bodies test the old FND-001. The source was restored.

Merge resolutions: `HorizonNotDecided` is gone. That matches main, where #1128 implements the horizon and QSL-551b deleted the refusal. `NotSingleExistential` is kept. `translate_itself` passes `Counting::Every`, which is right because HP-5 subjects are model subjects. The re-exports are the union of both sides and compile.

The changed expectation of `a_contract_with_no_value_leaves_its_state_open`, from `WitnessedUnchecked` to `NoDecision{Completed, {UndecidedSuccessor}}`, is right. The old witness was a terminal stutter loop at `(0, 1)`. A stutter step exists only at a terminal state, and whether `(0, 1)` is terminal is unknown, because `step`'s guard has no value there. So the lasso is not shown to be a behaviour at all. With no witness for the one initial state and an open node, FR-181 Outcomes leaves only `NoDecision`. Excluding every open state from the fairness search is conservative: a loop through real transitions with no fairness set would also be dropped. That loses a `WitnessedUnchecked`, but both outcomes are inconclusive, so no verdict changes.

Test cost: the brute-force oracle takes 114 s in debug on one thread. The `it` binary already has four other tests that run past 60 s, and the whole binary takes about 13.5 minutes, so the oracle is not on the critical path. The repo puts a test behind `#[ignore]` plus a make target (`test-deep-format`) only for memory-heavy cases. Keeping the oracle in the default run is acceptable.

## Dispositions of SR-13627 (code-review at 4943555dd)

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | cb9fc9cc2 (QSL-923 on main, #1136), merged in 2a81de08c; the oracle now has the disjunctive and `implies`-shaped bodies, and reverting the QSL-923 hunk fails it. |
| FND-002 | fixed | 2a81de08c: `.filter(\|id\| !self.open.contains_key(id))` in `Decision::lasso`; the test `a_loop_through_an_undecided_state_is_not_a_witness` is the reproduction, and removing the filter fails it and `a_contract_with_no_value_leaves_its_state_open`. |
| FND-003 | fixed | 2a81de08c: `ModelLasso { path, loop_entry }`, the stutter derived from `loop_entry == path.steps.len()`. |
| FND-004 | fixed | 2a81de08c: `Run::undecided(witnesses, end)` with an empty open set. |
| FND-005 | fixed | 2a81de08c: mutants B (M05c), C (M08c), D (M17), E (M20) and F (M34) are killed by the new tests. |
