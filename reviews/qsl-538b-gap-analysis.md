---
id: SR-12208
title: "Gap analysis of QSL-538 E8d remainder against FR-169-AC-8 to AC-11 and the package_id binding (PR #1058)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@344aec0b80462523e685c49f270076ae02a31992; spec/functional/FR-169-*.md, spec/functional/FR-282-*.md; qsl-replay/src/model_check/graph_settle.rs, qsl-replay/src/model_check/settle.rs, qsl-analyze/tests/it/state_graph_settle.rs, qsl-analyze/tests/it/state_graph_certificate.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: reviews
---
# SR-12208: Gap analysis of QSL-538 E8d remainder against FR-169-AC-8 to AC-11 and the package_id binding (PR #1058)

## Summary

Ticket: QSL-538 (E8d), the remainder after #1046. PR quire-spec-language#1058 at head 344aec0b8. The scope is the diff `origin/main...HEAD` only. Plan completion was not assessed. The tools were quire 0.36.2 (engine 0.50.2) and quoin 0.28.3. No tests were run; the lead runs the gates.

The PR adds six tests in `qsl-analyze/tests/it/state_graph_settle.rs`, and each one drives `qsl_replay::settle_state_graph`:

- **FR-169-AC-8 (TC-614).** `a_stopped_run_with_witnesses_settles_well_definedness_unchecked` runs the engine with `max_states` 2. It asserts `WellDefinednessUnchecked{sources}` with `Sampled` sources, `inconclusive`/`unsettled`/`completed`/`pending`, `limit_reached = MaxStates 2`, the run's `end` and one replay. With `witness_samples` 0 it asserts `Incomplete(LimitReached{MaxStates, 2})`, `failed`/`resource-incomplete` and no further replay. The binding is correct.
- **FR-169-AC-9, settlement half (TC-643).** `accepted_certificates_settle_proved_certified` covers three cases. (1) `ReachesTwo` under the default limits gives `Proved{Witness{sampled}, Certified}`, `decisive-witness`, success, the wire `witnessed` with the seed and trace index as decimal strings, `explicit-state`, the seed, `witness_samples`, the limits and one replay. (2) `always possible Guarded … from (x.phase != 2)` gives `Proved{Exhaustive, Certified}`, `closed-scope`. (3) Sequenced `InOneWay` gives `Proved{Exhaustive, Certified}` with no replay. The binding is correct. AC-9's last sentence (`Reduced`, `Uncertified`) has no test; see FND-002.
- **FR-169-AC-10, settlement half (TC-644).** Two tests cover all six rejected certificates named in the AC: `SuccessorMissing` at `min((1,2),(2,1))`, `RankBroken` at `(0,0)`, `RankMissing` at `(0,0)`, `PredicateUndefined` at the least `va = 2` digest, `CountBroken` at `000` and `OrderBroken` at the least non-`to` node. Each asserts `inconclusive`/`unsettled`/`CertificateRejected{rule, at}`, category inconclusive, never `proved` and zero replays. The `max_states` 2 check settles `Incomplete(LimitReached)`, `failed`/`resource-incomplete` and names `max_states` 2. The loci match FR-169-AC-10 as it now reads. The binding is correct.
- **FR-169-AC-11 (TC-644).** The settlement half asserts `CertificateRejected{BadState, va = 2}` with no replay. The AC's other clause, that the same ranks without the guard are accepted, is covered by the check-half test `an_undefined_guard_at_a_reached_state_rejects_bad_state` already on main. The binding is correct.
- **The package_id binding (TC-644, no AC).** `a_certificate_of_another_package_never_settles_proved` asserts two things. A run `package_id` other than the subject's settles `ReplayRefused(stale_dependency)` with zero replays, so the check stops at the binding. A replay result settled against another package also settles stale. The test carries no AC, which is correct: no FR-169 AC names the binding, and FR-282-AC-2 is TC-764's EN-5 case.

**Independence of expected values.** The expected states come from the test-side BFS `reachable` and the world's own arithmetic. The expected sources come from the engine's `sampled` field, which is a separate output of the run from the settled record. The package ids come from `qsl_package::emit_checked`. Nothing expected is read from the record under test.

**Each test can fail on its AC.** Each test asserts the exact `TerminalValue` together with the label, basis, execution and truth tuple. The replay counters pin the order "check, then replay". If the map replayed before the check, or settled `proved` on a rejected certificate, these tests would fail.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `settle_state_graph` ships every row of FR-169's table, but only the rows of AC-8 to AC-11 have a test. No test in the repo carries FR-169-AC-1 to AC-7. Untested: V-10 `Trapped`, V-4 `PathPair`, `Undefined` with its `undefined` record field (AC-6, AC-7), V-5 `BoundReached{depth: max_depth}` with the horizon recorded as a method parameter (AC-4), `UndecidedSuccessor`, `NoInitialState`, `Witnessed` whose replay settles `ReplayParity` and `Trapped` whose replay a limit stops (AC-5), and a `NoDecision{Stopped}` run (AC-4). If another ticket owns AC-1 to AC-7, route it there. Otherwise these tests belong in this PR, because the code they cover lands here. | qsl-replay/src/model_check/graph_settle.rs:331-367; spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:220-226 |
| FND-002 | low | FR-169-AC-9's last sentence has no test: the partial-order-reduced `possible` item settling `Proved{Reduced{…}, Uncertified}` with no certificate. No reduction exists yet, and the test module says so. This is recorded so that AC-9 is not reported fully met. | spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:228; qsl-analyze/tests/it/state_graph_certificate.rs:12-14 |
| FND-003 | low | The package-binding test covers only the `Witnessed` path. It does not cover a stale `Holds` binding, a stale replay of `WitnessedUnchecked`, `Trapped` or `PathPair`, or the two branches of `Row::bound` for a package that does not emit: `Incomplete(Cancelled)` under a cancelled handle, and `failed` otherwise. | qsl-replay/src/model_check/settle.rs:797-811; qsl-analyze/tests/it/state_graph_settle.rs:527-566 |

## Verdict

**PASS for AC-8 to AC-11 and the package_id binding. Mergeable, with no major finding.** Every tagged test binds to the right AC, asserts the full record row and can fail on the behaviour it claims. FND-001 is the real gap: the map ships the rows of AC-1 to AC-7 with no test. Close it in this PR unless another ticket owns those ACs. FND-002 waits on reduction support. FND-003 is low coverage breadth.

## Dispositions

Round 1, at 3aa1addf20576f12c5c08adecd96a5c7d3e474c7. Each outcome was checked against the fix commit's tests, not against the coordinator's claims.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 3aa1addf2. Six new tests cover the rest of the table. `an_explored_witness_proves_and_a_replayed_trap_refutes` covers AC-1 and AC-2. `a_replayed_trap_and_path_pair_refute` covers AC-1 and AC-3. `horizon_limit_undecided_and_empty_runs_settle_their_rows` covers AC-1 and AC-4: `BoundReached{3}` with `max_depth` 3 as a method parameter, `LimitReached` `max_states`, `UndecidedSuccessor` and `NoInitialState`. `a_disagreeing_or_stopped_replay_never_settles_decisively` covers AC-1 and AC-5. `an_undefined_target_settles_refuted_with_its_site` covers AC-6 at `(2, 0)` with division by zero. `a_sampled_witness_never_hides_an_undefined_refutation` covers AC-7. Each one drives engine outcomes and asserts the full row. |
| FND-002 | deferred | No reduced `StateGraphOutcome` exists at 3aa1addf2: there is no `Reduced` in `state_graph.rs` or `graph_settle.rs`. The test waits on the reduction slice. |
| FND-003 | fixed | 3aa1addf2. `a_certificate_or_evidence_of_another_package_never_settles` now covers a stale run package for `Witnessed` and `Holds` (no replay), and a stale replay for `Witnessed`, `WitnessedUnchecked`, `Trapped` and `PathPair`. It also covers `Row::bound` under a cancelled handle settling `Incomplete(Cancelled)`. Not covered: `Row::bound`'s `failed` branch for a package that fails to emit with no cancellation. No fixture produces that case, so it is left untested. |
