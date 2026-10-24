---
id: SR-12224
title: "Gap analysis of QSL-559 E15 PR 1 ProtocolSystem<Sc> against FR-205, FR-206, FR-210, FR-211, FR-215, FR-217 (PR #1102)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@5e4ce0ded; spec/functional/FR-205-*.md, FR-206-*.md, FR-210-*.md, FR-211-*.md, FR-215-*.md, FR-217-*.md (AC-1..AC-6), spec/decisions/ADR-027-*.md (context); qsl-eval/src/simulation/protocol/**, qsl-analyze/src/model_check/check.rs, qsl-replay/src/model_check/{replay,settle,counterexample}.rs, qsl-replay/src/certificate/closure.rs, qsl-route/src/deadlock.rs, qsl-analyze/tests/it/{protocol_system.rs,fill.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-210
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-211
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-217
    type: reviews
---
# SR-12224: Gap analysis of QSL-559 E15 PR 1 ProtocolSystem<Sc> against FR-205, FR-206, FR-210, FR-211, FR-215, FR-217 (PR #1102)

## Summary

Ticket: QSL-559, PR 1 of 3. PR 1 covers sequence, parallel, attempt, finish and finite replay. PR 2 covers channels, timeouts and `repeat`. PR 3 covers compensation, roles and `on each`. Planless.

The matrix was computed by `quire matrix --scope . --format json` (quire 0.36.2) at 5e4ce0ded. Of the 26 criteria of FR-205, FR-206, FR-210, FR-211, FR-215 and FR-217, 13 are tagged and 13 are untagged. FR-207, FR-208, FR-209 and FR-212 are cited by QSL-559 but have no PR 1 scope.

Tagged bindings. Each test was read against its AC:

- FR-205-AC-2: `a_binder_lives_exactly_until_its_last_reader_steps`. Correct and complete. It checks the initial key's empty members, `roles`, the root at `fork`, the post-fork `control`, sorted maps, byte-equal re-serialization, and binder lifetime in the `Read` variant.
- FR-206-AC-1: `fill_reaches_exactly_the_seven_states_of_its_table`. Correct and complete.
- FR-206-AC-2: `an_attempt_is_the_model_application_over_its_selected_clauses`. Correct and complete. It covers the three `bump` successors equal to `ModelSystem`'s, an attempt with the omitted precondition, and a constraint no record satisfies, which waits with cause `Constraint`.
- FR-206-AC-6: `finish_waits_for_every_other_thread_of_its_instance`. Correct and complete.
- FR-210-AC-1, AC-2, AC-4: the three projection tests. Correct and complete.
- FR-211-AC-2: `a_protocol_deadlock_names_its_blocked_threads_and_settles_refuted`. Correct. The `terminal any` row is checked through `protocol_deadlock_freedom_items`, the helper the request writer would call (see FND-006).
- FR-211-AC-3: `the_repaired_fill_is_deadlock_free_over_ten_states`. Correct and complete.
- FR-215-AC-1: `an_attempt_identity_serializes_its_step_thread_node_operation_and_arguments`. Correct. The order assertion depends on SR-12223 FND-006.
- FR-215-AC-3: `sc_is_the_identity_of_the_memory_seam`. Correct and complete.
- FR-217-AC-2: `a_protocol_replay_refuses_a_stale_or_disabled_step`. Incomplete, see FND-001.
- FR-217-AC-3: `a_protocol_replay_recomputes_the_blocked_threads`. Correct and complete.

Untagged criteria, by owner:

- PR 2 (channels, timeouts, `repeat`): FR-205-AC-3, FR-206-AC-4, FR-206-AC-5 (the `repeat` and `await` rows), FR-210-AC-3.
- PR 3 (`on each`): FR-205-AC-4 (the `on each` row) and FR-211-AC-4.
- In PR 1's scope but left untagged: FR-205-AC-1, FR-211-AC-1, FR-215-AC-2, FR-217-AC-1 (FND-002 to FND-005).
- Owned by no named PR: FR-206-AC-3's `join predicate`, FR-206-AC-5's `choice` and `check`, FR-215-AC-4's `event` record root, and FR-217-AC-4's lasso replay (FND-006).

The `Part of` doc comments on the untagged tests state honestly what is left out. Untagged here means not yet claimed, not falsely claimed.

## Verdict

**CONDITIONAL.** No criterion is tagged falsely, and no high finding. One tagged criterion, FR-217-AC-2, is not fully met, because the refusal names one digest, not both. Four PR 1-scope criteria remain untagged, which is acceptable inside a three-PR ticket only if they are taken up by a named PR. Several constructs belong to no PR. No stub was found, and no coverage inflation: every tagged test asserts concrete states, identities, causes or settlements that fail if the step relation changes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-217-AC-2 is tagged, but its first row requires that a changed post-state digest refuses `stale_dependency`/`content-mismatch` "naming the step and both digests". `ModelTraceRefusal::PostState { step, recorded }` names the step and only the recorded digest, and the test asserts exactly that variant. FR-217 "Behavior" says the same. Either the refusal carries the reached digest(s) or the AC changes. | qsl-replay/src/model_check/replay.rs:224-229, qsl-analyze/tests/it/protocol_system.rs:1134-1144 |
| FND-002 | medium | FR-205-AC-1 is in PR 1's scope (the protocol subject) but untagged. The subject refusals are tested in `a_protocol_subject_refuses_its_bindings`. The obligation identity is not: two subjects differing only in `over` or in the clause's `terminal` member must have different identities. No code in the diff builds a protocol subject's obligation identity, which FR-205 "Subject" requires to bind `terminal`, `scheduling` and the resolved memory models. | qsl-analyze/tests/it/protocol_system.rs:463-500 |
| FND-003 | medium | FR-211-AC-1 is untagged. The diff adds no test that `Fill` checks with all three `terminal` forms and gets pairwise different identities, or that a second `terminal` refuses `ambiguous-name` and a binder-reading `terminal when` refuses. The S3 side may already exist from E15a. The PR needs either the test or a pointer to the PR that owns this AC. | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md:115 |
| FND-004 | medium | FR-215-AC-2 is untagged. Its first half is asserted in the FR-206-AC-2 test. The second half is not tested: a postcondition the evaluator leaves undecided yields `ContractUndetermined`, and `check_protocol` settles V-6 `UndecidedSuccessor`. The new `ProtocolExpressionUndetermined` path (an undecided binder or finish constraint, added to `is_refused_contract` and the closure checker's bad-state rule) also has no test. | qsl-eval/src/simulation/protocol/steps.rs:355-362, qsl-eval/src/simulation/model.rs:262-282 |
| FND-005 | medium | FR-217-AC-1 is untagged although PR 1 delivers finite replay. Reproduction, digests, the empty enabled set and live instance at s2, and equal blocked threads are all asserted across the FR-211-AC-2 and FR-217-AC-2 tests. The last sentence, "the envelope's steps serialize with protocol transition identities and the blocked list", has no implementation: `TemporalCounterexample` has no wire encoding, and `BlockedThread` has no `Encode`. | qsl-replay/src/model_check/counterexample.rs:150-160, qsl-eval/src/simulation/protocol/identity.rs:349-358 |
| FND-006 | medium | No QSL-559 PR owns several constructs, and they are refused today as `not-yet-implemented`. They are `join predicate` (FR-206-AC-3), `choice` and `check` (FR-206-AC-5), `event`, `effect` and `commit` with their ranged record roots (FR-215-AC-4's `RequiresBound` row), `fence`, and protocol lasso replay against FR-212's classes (FR-217-AC-4). The ticket's split names only channels, timeouts and `repeat` (PR 2) and compensation, roles and `on each` (PR 3). Without an owner, these ACs can be left untagged when the ticket closes. | qsl-eval/src/simulation/protocol/program.rs:108-144, qsl-replay/src/model_check/replay.rs:438-441 |
| FND-007 | low | Reverse gap: `check_protocol`, `check_protocol_closure`, `replay_protocol_trace`, `settle_protocol_check` and `protocol_deadlock_freedom_items` have no production caller. FR-211 PD-3's "the request writer SHALL add one DeadlockFreedom item per distinct protocol subject" exists only as a helper that tests call. This matches how the model-subject counterparts are wired today, so it is wiring to land with the request path, not dead code. | qsl-route/src/deadlock.rs:152-166, qsl-analyze/src/model_check/check.rs:438-530 |

## Coverage

- Criteria examined: 26. 13 tagged, 13 untagged. 12 bindings are correct and complete, and 1 (FR-217-AC-2) is incomplete.
- Untagged by owner: 6 belong to PR 2 or PR 3, 4 are in PR 1's scope, and 4 have no owning PR (some ACs are split across owners).
- Stubs and inflation: none found. No `todo!`, `unimplemented!`, placeholder `Ok(Default::default())` or tautological assertion in the diff.
- Semantic review: done as part of SR-12223 (intent↔test↔code per tagged AC, above), not as a separate fan-out pass.
- Plan completion: not assessed

## Dispositions

Disposition pass 1 at head cf4888b16 (fix commit e11ea8f4c and a merge of main). No new findings.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e11ea8f4c: `ModelTraceRefusal::PostState` carries `reached`, the digests the step's identity does reach, beside `recorded`. The FR-217-AC-2 test asserts both. |
| FND-002 | deferred | The protocol subject's obligation identity needs the compiled `package_id` to bind the `terminal` member, which FR-383's emission does not do yet. FR-205-AC-1 stays untagged until then. Its refusal half is tested. |
| FND-003 | deferred | The checking half of FR-211-AC-1 is now tested by `fill_checks_under_each_terminal_form`, untagged. The pairwise-different identities wait on the same FR-383 binding as FND-002. |
| FND-004 | deferred | The undefined contract and constraint paths are now tested by `an_undefined_contract_or_constraint_ends_an_undefined_evaluation`, untagged. The V-6 `UndecidedSuccessor` row needs a fixture whose evaluation is refused but neither undefined nor incomplete, and this suite has none. FR-215-AC-2 stays untagged. |
| FND-005 | deferred | No wire form exists yet for `TemporalCounterexample`, whether model or protocol. The envelope serialization row of FR-217-AC-1 lands with the counterexample wire encoding. FR-217-AC-1 stays untagged. |
| FND-006 | deferred | The coordinator assigns these constructs to QSL-559 PR 2 and PR 3 as remainders. They stay refused `not-yet-implemented` until then, and their ACs stay untagged. |
| FND-007 | deferred | The protocol entry points get production callers when the request path wires protocol subjects, as the model-subject counterparts will. This PR adds no request-writer change. |
