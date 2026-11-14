---
id: SR-2026111321481802
title: "gap analysis of PR2701 realizability over the qsl-eval arena"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-571-AC-1..6, FR-572-AC-5, FR-574-AC-3/4, FR-575-AC-2, QSpec FR-175-AC-1..6 and the tests PR2701 tags to them; PR2701 76d8a5a1ce34b00a49bcdce11ffe1decf5e92d2f"
review_set: subset
---

## Summary

Ticket: QSL-1896 (ADR-036 slice 4). Plan completion: not assessed. This is a planless audit over `quoin matrix --json` (quoin 0.29.4) at the PR head, scoped to the criteria the PR tags. FR-571-AC-1 to AC-5 are tagged and backed; FR-571-AC-6 has no binder. Every tagged test calls the new API (`realize`, `replay_strategy`), so none can pass without the code. Tags use `#[trace]` only, with no `/// Trace:` lines, and every tag names an AC that its test exercises at least in part.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-571-AC-6 has no tagged test and none can exist: the `bounded-static` arena has no owner branch, so its two `inconclusive` cases never arise. TC-1411 step 6 cannot be carried out either. The criterion must be restated as the profile fact (see SR-2026111321481803 FND-001 for the wording), and the existing environment-defeat test, which asserts one `defeats_per_branch` entry, then tags it. | spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md:123 |
| FND-002 | medium | Realizability counterexamples have no replay arm. PA-20 (SM-7) settles a refutation only after it replays, but only the projection arm exists (`qsl-replay/src/spine/protocol_structure.rs:229-257`). Deferring this to slice 5 is acceptable because the kind stays unserved, so no refutation reaches settlement. However, QSL-1897's description names settlement through PA-20 and not this arm. Add it to QSL-1897 before merge so the deferral has an owner. | qsl-replay/src/protocol_analysis/strategy.rs:99-133 |
| FND-003 | low | FR-571-AC-1's second half ("a table edited to fail the walk makes the result non-conclusive") is backed only through `replay_strategy` returning `Lost`. The `realize` mapping from walk failure to `Inconclusive(StrategyReplayFailed)` is not exercised. FR-571-AC-3's last sentence (a run limit settles `failed`) is also tagged but untested; one call with a one-state `RunBudget` would back it. `max_history`, `Unforced` and `NotClosed` are also untested, which is acceptable for this slice. | qsl-analyze/tests/it/protocol_realizability.rs:225-260, 263-288 |
| FND-004 | low | No test covers soundness against FND-001 and FND-002 of SR-2026111321481801: no fixture has two initial states, and none has a role with two enabled actions under `parallel` near the horizon. The tests cover one false-proved guard (an edited table fails the walk) and one false-refuted guard (an open range gives `CutCount`, not `proved`). | qsl-analyze/tests/it/protocol_realizability.rs:50-118 |

## Scope and evidence

Matrix binders were read from `quoin matrix --json` at the PR head. The table below records each test-to-AC binding and the AC text.

- FR-571-AC-1 (spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md): realizable protocol yields a witness whose replay walk passes; an edited table is non-conclusive. (examined)
- FR-571-AC-2 (spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md): hidden choice yields IndistinguishableHistories; environment defeat yields EnvironmentDefeat per owner branch. (examined)
- FR-571-AC-3 (spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md): absent premise inconclusive, declared-empty decides, horizon inconclusive, run limit failed. (examined)
- FR-571-AC-4 (spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md): equal observation sequences get equal actions; no common winning action is refuted or inconclusive. (examined)
- FR-571-AC-5 (spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md): quantifies over every admitted outcome; one admitted loss defeats; open range cut is inconclusive. (examined)
- FR-571-AC-6 (spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md): avoidable histories and cut-dependent defeats are inconclusive. (examined)
- FR-572-AC-5 (spec/functional/FR-572-key-protocol-analysis-results-structurally.md): two runs on one key return the same result. (examined)
- FR-574-AC-3 (spec/functional/FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md): deciding_element is the violated obligation's canonical node string. (examined)
- FR-574-AC-4 (spec/functional/FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md): trace_position for both realizability forms. (examined)
- FR-575-AC-2 (spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md): a qsl-eval-refused construct yields evaluator-unsupported with its cause. (examined)

## Verdict

FR-571-AC-1 to AC-5, FR-572-AC-5, FR-574-AC-3/4 (realizability halves) and FR-575-AC-2 are backed. FR-571-AC-6 is a spec defect, not a missing test, and the spec-review amendment closes it. The replay-arm deferral needs QSL-1897 updated before merge. After that, together with the code-review blockers, the PR is mergeable.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | `a_capture_tells_initial_states_apart_for_every_role` cannot fail for the reason it states. The `End` entry it finds under a lone `Captures` class comes from the terminal class `[Captures([])]` (the instance has been removed), not from the `big = false` initial state, where Warehouse never acts. The test passes unchanged under SR-2026111321481801 FND-009, a false `proved`. It needs a counterpart that must stay `refuted`: the hidden choice with a declared capture its guard does not read. | qsl-analyze/tests/it/protocol_realizability.rs:275-311 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-571-AC-6 now states the profile fact and is testable, and `an_admitted_loss_is_an_environment_defeat` tags it and asserts one `defeats_per_branch` entry. TC-1411 step 6 is rewritten to match. |
| FND-002 | deferred | QSL-1897 now names the realizability counterexample replay arm (ADR-036 PA-20, ADR-018 SM-7) in scope. The kind stays unserved until slice 5, so no refutation settles without it. |
| FND-003 | fixed | `an_unstated_premise_or_the_horizon_is_inconclusive` adds a `max_states: 1` run that settles `Failed(RunLimit { limit: "max_states", value: 1 })`. `replay_strategy` now returns the gate's outcome, and the edited table settles `Inconclusive(StrategyReplayFailed)`. |
| FND-004 | fixed | A two-initial-state capture fixture and a `forced_pair` unit test for the cut case are added. The capture test is defective (FND-005). |
| FND-005 | fixed | The capture test now requires two single-item classes `[Captures{instance: 0, ..}]` with different payloads, mapped to End and Receive, and that every class starts with the bound captures. Under FND-009 the terminal class would carry no Captures item, the End entry would be missing and the match would panic. Under FND-001 (no captures) the result is refuted and `witness` panics. The new `a_capture_the_guard_does_not_read_leaves_the_hidden_choice_refuted` must stay `IndistinguishableHistories` for Warehouse, and fails under FND-009's false `proved`. |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | FR-575-AC-1 requires each construct row to yield its slug "for projection and for realizability". The new `activation-on-each` row is tested only through `realize` (`an_activation_on_each_is_unsupported`). The shared `unsupported_construct` makes projection hold, but no test pins it. Add an `on each` case to projection's profile-construct table test. | qsl-analyze/tests/it/protocol_realizability.rs:504-523; qsl-analyze/tests/it/protocol_projection.rs:514-520 |
