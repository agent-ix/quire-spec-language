---
id: SR-12184
title: "Gap analysis of quire-spec-language PR #1025: QSL-536 EN-1 state-graph engine (FR-166, FR-167, FR-168)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@c4534444398992fa0af83915f91820ba21818d30; FR-166-AC-1..AC-7, FR-167-AC-1..AC-5, FR-168-AC-1..AC-9; TC-613, TC-614, TC-618; qsl-analyze/tests/it/{state_graph,graph_world}.rs and the production files they test (qsl-analyze/src/model_check/state_graph{,/decide,/explore,/sample}.rs, qsl-eval/src/simulation/{sample,trace}.rs, qsl-eval/src/simulation/model/state_predicate.rs, qsl-replay/src/model_check/{limits,state_graph}.rs)"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1025

## Summary

Ticket: QSL-536. PR quire-spec-language#1025. A planless gap analysis,
scoped to the diff. Plan completion: not assessed. Criteria were traced to
tagged tests by reading. No test was run (static brief). `quoin matrix
--json` did not finish within the review window, so the binding list below
comes from reading the tests.

Trace bindings in the diff. Each one was read against its AC and judged on
whether the test can fail on the behaviour the AC states:

- FR-167-AC-1 (`each_worked_example_explores_once_to_its_graph`): correct.
  It checks 9/18 against an independent BFS, `Start, Mid, Lost, Won`, 4/3
  and 5/5.
- FR-167-AC-2 (`a_horizon_and_a_state_limit_leave_their_nodes_open`,
  `an_undecided_contract_conjunction_leaves_only_its_node_open`): correct.
  The second test only takes the undefined-guard branch (SR-12183 FND-007).
- FR-167-AC-3 (`the_deadlock_item_and_the_subject_refusals`): correct.
- FR-167-AC-5 (`the_explored_graph_is_a_function_of_the_request`): correct.
- FR-166-AC-1 (`default_phase_zero_samples_a_witness_for_each_instance`):
  correct. It checks that the last state is the first with
  `versionNumber` 2, seed 0, trace < 64, and that the outcome's witness
  equals the sampled one.
- FR-166-AC-2 (`phase_zero_off_and_a_target_no_walk_reaches`): correct.
  Walks are 32 steps, a declared departure.
- FR-166-AC-3 (`trace_indices_and_seeds_name_each_walk`): correct.
- FR-166-AC-4
  (`an_overflowing_trace_index_and_a_cancellation_stop_before_any_walk`):
  correct. If phase 0 did not poll, exploration would insert the initial
  nodes before its own poll, and `nodes.is_empty()` would fail.
- FR-168-AC-1, AC-3, AC-4, AC-5, AC-6
  (`phase_zero_changes_the_source_never_the_outcome_kind`), AC-9: correct.
  AC-9's premise about canonical order is unasserted (SR-12183 FND-008).
- FR-168-AC-2 + AC-6 (`always_possible_traps_and_holds`): correct. The test
  runs AC-2's three requests twice and compares the outcomes, which is the
  second half of AC-6.
- TC-613 / FR-168-AC-7 (`a_sampled_witness_never_hides_an_undefined_target`):
  correct.
- TC-614 / FR-168-AC-8
  (`a_stopped_exploration_leaves_sampled_witnesses_unchecked`): correct.
- TC-618 (`the_walk_budget_bounds_and_counts_each_walk`): tagged with the
  TC only, because 4,096-step walks are not run (declared). Steps 1 and 2
  can fail. Step 3 cannot fail on its walk-length claim (SR-12183 FND-004).

Declared, not findings: FR-166-AC-5/6/7 and their settlement halves
(FR-169), FR-170 replay, FR-167-AC-4 (reductions and manifests),
`ConstraintBoundary`, `Holds{Reduced}`, `RunEnd::Stopped`'s `Option`
limit, and instances read from request bindings (QSL-886 / #1019 moves
these to `ModelSystem::instances`).

## Verdict

FIX, through SR-12183 FND-001. The gaps here are medium and recorded:
FND-001 is the missing test that would have caught SR-12183 FND-001.
FND-002 and FND-003 are spec and output gaps against QSpec FR-391 and
FR-168's Outputs, to route to the FR-169 ticket and the spec owner.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-168 "Partial runs" is tested for `possible` (AC-8, AC-9) and `always possible` (AC-4), but not for `unique path`. No test runs a `unique path` item with an open node. That is exactly where the engine returns an unsound `Trapped` (SR-12183 FND-001). Add a test with the §7.3 job under a horizon: `NoDecision` for the sequenced job, and no `Trapped` for either job. | qsl-analyze/tests/it/state_graph.rs:855-889; spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md:155-163 |
| FND-002 | medium | FR-168 Outputs require a `StateGraphCertificate` beside `Witnessed` and `Holds{Exhaustive}`: the explored states and the distances or path counts (ADR-022 GC-1). `StateGraphOutcome::Witnessed`/`Holds` carry none, and the PR does not declare this departure. FR-169 checks the certificate, so route it to the FR-169 ticket. | qsl-replay/src/model_check/state_graph.rs:96-149; spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md:72-75 |
| FND-003 | medium | QSpec FR-391-AC-14: an undefined guard, `terminal when` predicate or state constraint at an explored node refutes every state-graph item over the subject (`undefined-evaluation`). QSL FR-168 "Undefined evaluation" covers only the claim's own predicates. The engine marks such a node open, `UndecidedSuccessor`, so the items get `NoDecision` or `WitnessedUnchecked` instead of a refutation. That is safe, never a wrong `proved`, but it departs from QSpec. This is a spec gap for the QSL spec owner. | qsl-analyze/src/model_check/state_graph/explore.rs:230-242; spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md:139-153 |

## Dispositions

Round 1, reviewed at 8ed0af489ea63d039ad0c10f6db88d763bb214ea (fix commit
8ed0af489). The new `unique path` horizon test fails on the pre-fix trap
rule (see SR-12183 Dispositions). The PR body now declares FND-002 and
FND-003.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8ed0af489 |
| FND-002 | deferred | Declared in the PR body: the certificate is FR-169's, built and checked with the settlement. Remaining work: QSL-891 (E11 remainder: FR-169 state-graph settlement). |
| FND-003 | deferred | Declared in the PR body as a spec gap (QSpec FR-391-AC-14 has no counterpart in QSL FR-168). The engine is safe here: it never returns a wrong `proved`. Route it to the QSL spec owner. The PR names no ticket for it yet. |
