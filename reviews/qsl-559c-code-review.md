---
id: SR-12237
title: "Code review of quire-spec-language PR #1119: binder environments, send records, Displaced and fence"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@9b307d66d18a2a01eae4a3b71886e30883240271; PR #1119: qsl-eval/src/simulation/protocol.rs, qsl-eval/src/simulation/protocol/{channel,footprint,identity,memory,program,settle,steps}.rs, qsl-eval/src/simulation/subject.rs, qsl-analyze/src/model_check/check.rs, qsl-replay/src/model_check/{counterexample,replay}.rs, qsl-semantics/src/check/protocol_clause/types.rs, qsl-analyze/tests/it/{protocol_constructs,protocol_footprints,protocol_system,fill}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-217
    type: reviews
---
# Code review of quire-spec-language PR #1119

## Summary

Ticket: QSL-559 (follow-up PR 3c). This is a code review with the rust-review
lane folded in. It covers `git diff origin/main...HEAD` only. It was checked
against the merged spec on origin/main: ADR-027 PS-2, PS-5, PS-8, ST-3, SE-5,
TS-2, PD-4, PX-1, PX-2, FT-1 and FT-5, and FR-205, FR-206, FR-211, FR-215,
FR-216 and FR-217. QSpec FR-426 was cited by id only. Soundness was checked
first.

The narrow tests were run through `locked-build.sh`: `cargo test -p qsl-analyze
--test it` filtered to `protocol_footprints::`, `protocol_constructs::` and
`protocol_system::`. The result is in the Verdict.

These parts were checked and found clean:

- **Environments are part of the state key, and the key stays finite.**
  `Instance::environments` is keyed by (binder node, holder thread) and
  encoded into `observations`. Each thread has at most one entry per node, and
  `normalize` drops every entry whose holder `reaches` no reader. `reaches`
  returns false for a thread that is not in `threads`, so an environment
  leaves with its thread. Pruning depends only on the holder's control and on
  its `all`-joining ancestors. Those ancestors rest at their joins while the
  holder is live, so another thread's step cannot change the pruning. This
  keeps commutation.
- **PS-5 copy rules.** A fork copies the parent's environment into each branch
  thread. An `all` join copies only the entries that `escapes` accepts. These
  are entries of nodes inside the branch body and not under a `choice` case, a
  `repeat` body, an `await` or an inner join of another policy. This matches
  S3's `Out` and PS-5's exclusions. A rebinding replaces only the binder's own
  entry, so other threads keep their copies.
- **Removed behaviour.** The old nearest-ancestor lookup, the heir copying in
  `bind`, the fork's stale clearing and the `a-reader-sees-one-binding-of-a-node`
  fault are deleted. This matches FR-205-AC-7: each `u` reads its own copy and
  no state refuses.
- **Ordinal held while an environment names a binder under it: deliberately
  not built. This is correct.** Merged PS-2 holds an ordinal only while its
  parallel instance is live or a live thread's path extends a branch path.
  `free_ordinal` does exactly that. An environment entry is keyed by its live
  holder and is a copy, so reusing an ordinal cannot alias an entry. PS-5
  says so directly: "an ordinal's reuse never changes what another thread
  reads". Counters are kept only for running threads, so dropping the fork's
  clearing is safe.
- **Footprints.** A `Binder` entry meets its holder's `Control` and any
  `Subtree` above it, which matches FT-1's "the environments are part of
  `ctl`". An `all` join reads the `ctl` of each joined branch. The end of the
  last running branch of a settled instance writes the whole `Instance`.
  `binders_read` now names the stepping thread's own entry. A `send` or
  `receive` is visible to an anchor that names its channel (FT-5, PB-2).
- **The commutation test is not vacuous.**
  `every_independent_pair_of_steps_commutes` adds two environment fixtures at
  indices 9 and 10: a re-forked `Q` reading a binder that was copied at a join,
  and a binder copied through two nested `all` joins. It asserts `checked > 0`
  for each fixture. It compares post-state sets in both orders, and those sets
  are keyed by the full state key, environments included.
- **Send record.** `SendRecord {channel, payload, outcome, displaced}` is
  carried on the `send` transition. It is not part of the identity: the
  manual `PartialEq` compares kind, thread, node and members, and `enc()`
  leaves it out. It is not part of the state key either: the binder holds the
  payload alone. The outcomes are `accepted`, `rejected` and `lost`, with
  `displaced` set to the removed entry for `loss head` or `loss tail`, as ST-3
  requires.
- **Replay refusals (PX-2).** `send_records_in_place` runs before any step is
  replayed. It refuses `invalid_runtime_input`/`invalid-value` for a send with
  no record, or for another step that has one. `protocol_result` compares the
  recorded record with the replayed one, and a difference refuses
  `stale_dependency`/`content-mismatch`, naming the step and both records.
  The deadlock recomputation reads the replayed transitions, which carry
  `removed`.
- **Displaced (PD-4).** The cause is read from the reported prefix. It is
  `Displaced` when the last removal of an entry of the named send on this
  channel and instance was a loss: a `lose`, a `head` or `tail` displacement,
  or a `lost` send. `SubjectSystem::blocked` takes the prefix. `FinishWaiting`
  is separated from `JoinWaiting`.
- **Fence.** `ControlStep::Fence` rests as `Rest::Node`. Its step is gated
  only by `GatedStep::Fence`, it moves the thread on and does not change the
  model. Its wait cause is `Gate`. It is uncounted in the projections. The old
  refusal test now uses `memory tso`.
- **Rust.** No new `unwrap`, `expect`, indexing or panic was found in
  production code. The loops in `escapes` and `reaches` climb strictly toward
  the root and end with a fault or at the root. The new `match` arms are
  exhaustive with no wildcard arm.

## Verdict

Narrow tests: 42 passed, 0 failed. Changes requested: one high finding (FND-001). The fence's transition
identity leaves out the `ordering` member that FR-215 and TS-2 require, and a
test fixes the wrong shape in place. Soundness of environments, pruning,
footprints, replay and Displaced is clean. Two lows are recorded and do not
block.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A `fence` step's transition identity has no `ordering` member. FR-215 "Transition identity" says the identity SHALL carry "for a `fence` and an access with an ordering, the ordering". ADR-027 TS-2 and QSpec FR-426's identity table (row `ordering`: `fence`) say the same. `fence()` builds `ProtocolTransition::control(StepKind::Fence, …)`, which has no further member. `a_fence_moves_its_thread_and_changes_nothing_under_sc` asserts the identity `{"type","step":"fence","node","thread"}`, so the test fixes the defect in place. The ordering is available in `CheckedMemory::fences`. Fix: add an ordering member to the fence identity and assert it in the test. | qsl-eval/src/simulation/protocol/steps.rs:371-389; qsl-eval/src/simulation/protocol/identity.rs:366-375; qsl-analyze/tests/it/protocol_constructs.rs:353-357 |
| FND-002 | low | `SendRecord::with_outcome` is public API that only a test calls. It exists so the test can forge a wrong record. `ProtocolTransition::carrying` is also public and is set outside the crate only by tests. No wire decoder exists yet that would need it. Either keep `carrying` for the coming decoder and remove `with_outcome`, or gate both behind a test-support feature. | qsl-eval/src/simulation/protocol/identity.rs:288-291; qsl-eval/src/simulation/protocol/identity.rs:450-453 |
| FND-003 | low | `Instance::environment(holder)` scans every entry of every thread, because the map is keyed with the node first. `fork` calls it once per fork, and `join` once per joined branch. This costs time only and gives no wrong result. Keying by (thread, node) would allow a range lookup. | qsl-eval/src/simulation/protocol.rs:322-330 |

## Dispositions

Round 1, reviewed at dccdc13b9735e5ad6df04b5ce5b7a457c538aa2e. Narrow tests:
42 passed. A default `cargo check -p qsl-eval -p qsl-replay` (no feature) builds.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | dccdc13b9: `ProtocolTransition::fence(thread, node, ordering)` with `Members::Fence { ordering }` encodes `"ordering"`. The value is `AccessOrdering::label()` from `CheckedProtocol.memory.fences`, and the test asserts `"ordering": "seq_cst"` and `"acquire"`. A commutation fixture with one fence per branch was added. |
| FND-002 | fixed | dccdc13b9: `with_outcome` and `carrying` are `#[cfg(any(test, feature = "test-support"))]`. Production uses `pub(crate) recording`. Only qsl-analyze's dev-dependency enables the feature (resolver 2), and the default build has neither method. |
| FND-003 | fixed | dccdc13b9: `environments` is `BTreeMap<ThreadPath, BTreeMap<ProtocolNodeId, Bound>>`. `environment` is a single lookup, and empty environments are removed in `normalize`. The key's `observations` is still sorted by `map_enc`. |
