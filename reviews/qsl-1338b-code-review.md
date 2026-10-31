---
id: SR-1937001
title: "code review of PR #1937 (QSL-1338 slice b): qsl-analyze model-check test speed-up"
type: SpecReview
analysis: code-review
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1937 git diff origin/main...fd15a17ef (qsl-eval/src/simulation/model.rs, model/application.rs, model/frames.rs, model/walk.rs, protocol/footprint.rs, protocol/ra.rs, protocol/steps.rs; qsl-analyze/src/model_check/hyper/single.rs, state_graph.rs, state_graph/sample.rs; qsl-analyze/tests/it/{component_certificate,example_units,fill,hyper_single,protocol_footprints,release_acquire,units}.rs)"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-120
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-229
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-166
  type: references
---
# SR-1937001: code review of PR #1937 (QSL-1338 slice b)

## Summary

Ticket: QSL-1338. Code review with the rust-review lane folded in, scoped to the PR diff at head fd15a17ef59438e21a606854b66f262676b729cd. Major bugs only, soundness first. Focused runs at the head: `cargo test -p qsl-analyze --test it -- hyper_single release_acquire strong_hint component_certificate example_units protocol_footprints` 89/89; `-- state_graph model_instances` 79/79; `-- litmus protocol_ tso_fairness weak_memory refinement_protocol` 166/166; `cargo test -p qsl-eval --lib -- simulation::protocol simulation::model` 36/36; `cargo test -p qsl-eval --test it -- model_simulation protocol_reference protocol_slots` 30/30; `cargo clippy -p qsl-eval -p qsl-analyze --all-targets --all-features -- -D warnings` clean. Reviewer probe (not pushed): with `qsl-eval/src/simulation/protocol/ra.rs` reverted to origin/main and everything else at the head, `release_acquire` passes 21/21, including the new byte-identity test.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| WalkMemo and OneInitial memo routing | qsl-eval/src/simulation/model.rs:1633-1750 | examined |
| ModelSimState fields (state, anchor, reached_by) | qsl-eval/src/simulation/model.rs:159-181 | context_only |
| ModelSystem::key, model_key, semantic_enc | qsl-eval/src/simulation/model.rs:1488; model/encode.rs:733-882 | context_only |
| AppliedMemo, apply_attempt, enabled_attempt, applied_at | qsl-eval/src/simulation/model/application.rs:305-448 | examined |
| apply_operation_matching, enabled_transitions_matching | qsl-eval/src/simulation/model/application.rs:186-270 | context_only |
| Budget::used, charge, charge_many | qsl-eval/src/simulation/model/walk.rs:35-80 | examined |
| walk_of encode removal | qsl-eval/src/simulation/model/walk.rs:244-260 | examined |
| post_conjunction evidence, complete Undecided key, post_observations, is_closed | qsl-eval/src/simulation/model/frames.rs:1160-1180,1300-1320,1360-1445,1466-1484 | examined |
| order::expand successor keying, sample loop | qsl-eval/src/simulation/order.rs:78-112; sample.rs:115-190 | context_only |
| ProtocolSystem::footprints / footprint_with | qsl-eval/src/simulation/protocol/footprint.rs:455-535 | examined |
| RaComponent view_enc, event_enc, graph_enc | qsl-eval/src/simulation/protocol/ra.rs:1039-1185 | examined |
| attempt sites now via apply_attempt / enabled_attempt | qsl-eval/src/simulation/protocol/steps.rs:400-520,1210-1275 | examined |
| hyper single Run.walks | qsl-analyze/src/model_check/hyper/single.rs:159,245-249,405-420,489-495 | examined |
| state_graph phase 0 walks | qsl-analyze/src/model_check/state_graph.rs:630; state_graph/sample.rs:88-92,205-215 | examined |
| Domain::with_versions_up_to and its callers | qsl-analyze/tests/it/units.rs:71-92; example_units.rs:55-84; component_certificate.rs:669-690 | examined |
| the_sc_graph_member_names_events_and_edges_by_depth | qsl-analyze/tests/it/release_acquire.rs:347-480 | examined |
| hyper_single oracle Expansions sharing | qsl-analyze/tests/it/hyper_single.rs:139-370,1187-1200 | examined |
| commuting_pairs via footprints; fill::Reached.key | qsl-analyze/tests/it/protocol_footprints.rs:618-645; fill.rs:553-592 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**Approve: no findings. Mergeable.**

1. Memo purity. `ModelSystem::successors` is `walk::expand_state(self, state, None)`. It reads only `state.state` and `state.anchor`. `reached_by` is read only on post-states (`judge.rs` `result_of`, `timed.rs`), and a post-state's `reached_by` is built from the parent and the transition. Each expansion draws a fresh `Budget`. The evaluation meter is a `ScalarLimits` copied into a fresh `Meter` per clause. `footprints`, `declaring` and `bounded` are pure caches. So `WalkMemo`'s key (canonical `model_key` bytes plus the anchor) covers every input. The attempt memo key is the observed state's key and anchor, `format!("{node:?}")`, the selected-transition set and the enabled-only flag. `operation` comes from `self.operation(node)` and `select` from `attempt.contracts`, so both are fixed by `node`. `apply_operation` is `apply_operation_matching(.., None, ..)`, so the two call shapes that now share entries were already one function. Budget: a stored entry is only a run that finished, and `charge` stops only on `used > max`. A hit's `charge_many(charged)` therefore stops exactly when the original incremental run would have stopped, with the same `exhausted(MaxCandidates)` stop. Errors are never stored. Findings come back as stored. They are functions of the same inputs, and the sampler adds its own depth and state key.
2. Removed encodes. Every reached successor is still keyed by the engine: `order::expand` calls `canonical_state_key(system.key(next))` for each successor before the sampler or explorer sees it, and `sorted_initial` keys the initial states. The analyze callers that call `successors` directly also key what they keep: concretise, graph, product, hyper and the state-graph explorer. The only change is for a state whose key cannot be encoded (over 16 MiB, or an `Enc::Unencodable` from a broken invariant). Its refusal moves from `stop(BROKEN)` inside expansion to `NotSimulated::KeyEncoding`, which names `identity.input_bytes`. That is what `sample`'s own doc contract says. The model's hard-coded default limit no longer overrides the request's `identity`. Candidates the frame decision rejects are no longer encoded, which is correct because they never become states. Protocol observed states (`memory.observe` views) are not reached states, and the protocol state that holds the model is keyed by the protocol engine. No path now admits a reached state that the run's own identity limits refuse.
3. RA reuse. `view_enc` and `event_enc` do the same `shared.get` lookup and the same `position` search as `self.location_enc` and `self.position`. When `message` is `None`, `Some(held.id) == None` is false, which matches `message?`. `graph_enc`'s per-id cache memoizes a pure function of `id` over `&self`. The byte-identity probe above confirms that output is unchanged. `footprints` runs `state_accesses` once on one fresh budget, where `footprint` ran it on a fresh budget each time, so the results are identical. Its only caller is a test.
4. Memory. `WalkMemo` is owned by the hyper `Run` and by the state-graph `Phase`, each with one `ModelSystem`, and is dropped with the run. It holds one expansion per distinct (key, anchor) the walks visited. `AppliedMemo` lives in `ModelSystem`, which every production caller builds per request (`check.rs`, `state_graph.rs`, `hyper/single.rs`, ...). It is bounded by the observed states of the reachable protocol states times the attempts.

Rust lane: no `unsafe` and no new panics. The `u64`/`usize` conversions saturate (`unwrap_or(MAX)`). `Mutex` poisoning goes through `PoisonError::into_inner`. No `MutexGuard` is held across a re-entrant call: `apply_operation_matching` never calls back into the memo. Clippy is clean.

Outside this diff (pre-existing, for the lead): `semantic_enc` (`model/encode.rs:733`) encodes populations only and never `ModelState.extents`. `frames.rs:858` prunes an extent pair when an object is deleted, and `slots()` lets a later creation reuse that unused key. So two states with equal populations and different extents could share a state key. The explorer's dedup already relies on key equality meaning state equality, and the new memos inherit that reliance. Reachability was not reproduced.
