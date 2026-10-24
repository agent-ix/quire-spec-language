---
id: SR-12143
title: "Code review of quire-spec-language PR #973: E5 check_model phase 1"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@fcb94559221fd86a5610e71f091a5396eaa5b302; PR #973 diff vs origin/main: qsl-analyze/src/model_check/{check.rs,mod.rs}, qsl-analyze/tests/it/{counter.rs,deadlock_item.rs,model_check.rs,model_settle.rs,graph_core.rs,main.rs}, qsl-eval/src/{model_subject.rs,simulation/expansion.rs,simulation/model.rs,simulation/model/{terminal.rs,walk.rs,contracts.rs},value/expression/mod.rs,value/mod.rs}, qsl-replay/src/model_check/{counterexample.rs,limits.rs,outcome.rs,replay.rs,settle.rs}, qsl-replay/src/{execute.rs,outcome.rs,proof_result.rs,lib.rs}, qsl-route/src/{deadlock.rs,lib.rs}, Cargo manifests, tests/it/family_outcome_layering.rs"
review_set: subset
---
# Code review of quire-spec-language PR #973

## Summary

Ticket: QSL-526. This is a code review with the Rust lane of the EN-1 first phase. The phase covers `check_model` for TP-1 and the deadlock-freedom item, the FR-124 request-writer item, FR-127 settlement and FR-128 replay of deadlock-item counterexamples. It was checked against FR-124, FR-125, FR-126, FR-127 and FR-128. The leader ran the gates (check, clippy, tests). The reviewer ran no cargo command, as briefed.

The engine is sound for this phase. The search is breadth-first in canonical order. The first bad state ends the run, and the counterexample is that state's tree path. A horizon cut settles `BoundReached`, a named limit or a cancellation settles `Stopped`, and neither ever settles `Holds`. A refused contract settles `Undecided`. The settlement map settles `Refuted` only from a `Reproduced` replay. A parity result, a refusal or a fault never settles `Refuted`. One blocking defect is at the replay boundary: a deadlock-item counterexample is recognised by the payload's shape, and the envelope's clause and occurrence identity is never checked.

## Verdict

**FAIL**: FND-001 is high. A refutation can be reproduced against an envelope whose identity is not the deadlock-freedom item's. That includes an authored claim's identity, so the derived item and authored claims collide at replay. FND-002 and FND-003 are medium and FND-004 is low. They are recorded and do not block.

Checked and clean:

- First-bad-state ordering. Initial states sort by key bytes and successors by transition bytes, then by post-state key bytes.
- The counterexample prefix and `initial_index`, which match the replay's `system.initial()` order.
- Horizon handling. `horizon_cut` is set only when a horizon state has successors.
- `max_states`, `max_transitions`, `max_automaton_states`, the evaluation meter and `max_candidates` are each named in `ReachedLimit`.
- The exhaustive `AutomatonError` match, which covers E6's new variants as faults.
- `NoInitialState` is settled before exploring.
- The requires-bound pre-check.
- The replay's step selection by post-state digest, and its NotEnabled and PostState refusals.
- The `undefined` replay's first-undefined-position rule.
- The `IncompleteCause::LimitReached` wire members.

No vendoring and no file-tracking ceremony were found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `replay_model_trace` decides whether a counterexample is for the deadlock-freedom item from the payload alone (`names_deadlock_item`: `kind: Deadlock`, or an undefined non-atom with no `over_binding`). It never resolves or compares the envelope's `clause_node` or `occurrence_key`. FR-128 Behavior says the executor SHALL resolve the clause by its `clause_node` and `occurrence_key` and refuse `stale_dependency`/`content-mismatch` when either differs. As written, any envelope reproduces a deadlock payload: an authored TP-1 claim's identity, or the fabricated `[7; 32]` node the tests use. So an authored claim can be "reproduced" as refuted by a deadlock that does not refute it, which is the identity collision FR-124 forbids. An authored clause with an empty binding set has an undefined-contract counterexample (`over_binding: None`). It is also replayed as the deadlock item. Fix: give the derived item a defined envelope identity, refuse a deadlock payload under any other identity, and add a test where an authored-clause envelope carrying a `Deadlock` payload refuses `content-mismatch`. | qsl-replay/src/model_check/replay.rs:201-207; qsl-replay/src/model_check/replay.rs:238-250; qsl-analyze/tests/it/model_settle.rs:179-188 |
| FND-002 | medium | `settle_model_check` takes the replay as a caller-supplied closure (`impl FnOnce(&TemporalCounterexample) -> Result<ModelTraceReplayResult, ReplayRefusal>`). It settles `Refuted` on any `Reproduced` result the closure returns. It does not compare the result's `package_id` with anything, and it does not check that the reproduced value matches the counterexample it was given. The module doc says "The map replays a counterexample itself", but the map does not: a caller can settle `refuted` without a replay (for example `\|_\| Ok(Reproduced{..})`), and SM-7 then rests only on caller discipline. Either have the map call `replay_model_trace` with the request and subject it needs, or correct the doc and check the result's `package_id` against the counterexample's envelope. | qsl-replay/src/model_check/settle.rs:5-6; qsl-replay/src/model_check/settle.rs:86-95; qsl-replay/src/model_check/settle.rs:157-171 |
| FND-003 | medium | `ModelSystem::terminal_class` reads every `terminal` declaration in the package, in alias order, and returns `Intended` on the first `Any` or true `When`. `qsl_route::deadlock::opts_out` likewise drops the item when any declaration in the package is `Any`. FR-124 is per state model: "the `terminal when` predicate declared for the state model", and "When a subject's state model declares `terminal any`". The package admits one declaration per model alias. In a package that selects two aliases, `terminal any` on one would mark every terminal state intended and suppress the other alias's deadlock item. A real deadlock would then settle `Proved`. Confidence is low: `ModelSystem` builds one `StateModel` from all `model_selections()`, so which declaration governs a multi-alias subject may be a spec gap rather than a code bug. Either way, no test covers more than one alias. | qsl-eval/src/simulation/model/terminal.rs:78-98; qsl-route/src/deadlock.rs:101-108 |
| FND-004 | low | The visited set `known` is keyed by the model state key alone, while the module doc and `Node` describe product states (model state × monitor state). This is sound only because both phase-1 monitors have a single non-rejecting state. If TP-2 or TP-3 reuse this loop with a multi-state bad-prefix monitor, distinct product states would merge and violations could be missed. Key `known` by (state key, monitor state), or document the single-state precondition where the monitor is built. | qsl-analyze/src/model_check/check.rs:392-449; qsl-analyze/src/model_check/check.rs:296-313 |

## Dispositions

Round 1, reviewed at 72346678bf8568550ea3362a10a53f53960c293a.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 72346678b. `replay_model_trace` now calls `resolve_item`, which compares the envelope's `clause_node` and `occurrence_key` with `deadlock_freedom_clause(package_id)`. The node is the recompiled `package_id` and the occurrence is role `deadlock-freedom`, ordinal 0. A deadlock-shaped payload under any other identity refuses `ItemIdentity` (`stale_dependency`/`content-mismatch`, naming both identities). So does an authored payload under the derived identity. The new test `a_counterexample_resolves_its_item_by_clause_node_and_occurrence_key` asserts four cases: reproduction under the derived identity; `ItemIdentity` under an authored identity, with exact fields; `ItemIdentity` under the derived node with another ordinal; and `ItemIdentity` versus `UnknownTemporalClause` for a `Formula` payload under each identity. The zero-binding undefined-contract case under an authored identity now refuses as well. |
| FND-002 | fixed | 72346678b. `settle_model_check` takes the run's `package_id` and settles a replay result whose `package_id` does not match it as `Inconclusive(ReplayRefused(stale_dependency))`, ahead of the `Reproduced` arm. The module doc now says the caller's replay supplies the result and the map checks its package. A new assertion in `a_disagreeing_or_refused_replay_never_settles_refuted` covers the other-package case. |
| FND-003 | deferred | Medium. Recorded without a fix under this PR's review policy (only major bugs block). `terminal.rs` and `deadlock.rs` are unchanged. Which `terminal` declaration governs a subject that selects several model aliases needs an FR-124 ruling, because `ModelSystem` builds one `StateModel` from all `model_selections()`. |
| FND-004 | deferred | Low. `check.rs` is unchanged. This is sound for phase 1, whose monitors have one non-rejecting state. The visited-set key must carry the monitor state before TP-2 or TP-3 reuse the loop. |
