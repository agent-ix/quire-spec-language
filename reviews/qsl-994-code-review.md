---
id: SR-17011
title: "Code review of PR #1446: tso flush fairness and liveness over tso parallels, code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2c2b756cb85b4a39dd51b30a50fabd915c7aa1b8; git diff origin/main...2c2b756cb (PR #1446, branch qsl994-tso-fairness): qsl-eval/src/simulation/protocol.rs, qsl-eval/src/simulation/protocol/{fairness,memory,tso,ra}.rs, qsl-analyze/src/model_check/{liveness,check,concretise}.rs, qsl-replay/src/model_check/replay.rs, qsl-analyze/tests/it/{tso_fairness,scheduler_fairness,protocol_system,main}.rs; ticket QSL-994"
review_set: subset
date: '2026-10-24'
---

## Summary

Ticket: QSL-994. PR: quire-spec-language#1446, head 2c2b756cb. Method: code-review with the rust-review lane. The PR adds a `memory` fairness origin and a `MemoryStep` fairness target. `Tso::fairness` now returns one `fair weak each flush` member for each branch of each tso `parallel`. The liveness engine is now generic over `ProtocolSystem<M>`, so it runs for sc and tso, and only ra is still refused `WeakLiveness`. If a liveness run explores a state where the store-buffer bound cut a store, it settles `MemoryBoundReached`. Lifted liveness steps now carry their memory component. In protocol replay, the step re-execution now runs before the fairness-set comparison.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| ProtocolFairness::memory_step, class_of MemoryStep arm, memory_branches, fairness_set | qsl-eval/src/simulation/protocol/fairness.rs | examined |
| MemoryModel::fairness signature, MemoryBranch | qsl-eval/src/simulation/protocol/memory.rs | examined |
| Tso::fairness | qsl-eval/src/simulation/protocol/tso.rs | examined |
| Ra::fairness (signature only) | qsl-eval/src/simulation/protocol/ra.rs | examined |
| ProtocolSystem::fairness_set, reported | qsl-eval/src/simulation/protocol.rs | examined |
| check_protocol_liveness | qsl-analyze/src/model_check/check.rs | examined |
| Finding::BoundLimited, dead_limited, decide ordering | qsl-analyze/src/model_check/liveness.rs | examined |
| Lift::edge reported | qsl-analyze/src/model_check/concretise.rs | examined |
| ProtocolReplay check reorder | qsl-replay/src/model_check/replay.rs | examined |
| tso_fairness.rs (7 tests), scheduler_fairness ra pin | qsl-analyze/tests/it | examined |
| selection::governing, access::classify, ThreadPath, PathStep | qsl-eval/src/simulation/protocol/ | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test pins MB-4's rule that a bound-limited run settles before a counterexample. A mutation that returns `Violated` ahead of the bound check passes all 758 qsl-analyze tests. The only bound test uses a true claim, so it has no lasso to report. A tso liveness run that cut a store and also found a lasso could therefore report a spurious lasso without any test failing: the cut store keeps its branch's class enabled (MF-4), but the filter never sees it. | qsl-analyze/src/model_check/liveness.rs:1465-1489, qsl-analyze/tests/it/tso_fairness.rs:582 |
| FND-002 | medium | The flush member target is encoded as `{"type":"memory-step","model","step","node","branch"}`. It reuses the `memory-step` tag of `MemoryStepId`, which carries `thread`, and it differs from FR-228/QSpec FR-436, which give a thread path. This shape now goes into every tso fairness set, obligation identity and counterexample payload before spec PR #1442 settles it. If #1442 picks another shape, stored tso counterexamples stop replaying (the replay refuses with `ProtocolFairnessSet`). The PR body already lists this as spec mismatch 1. | qsl-eval/src/simulation/protocol/fairness.rs:105-118, qsl-eval/src/simulation/protocol/memory.rs:41-62 |
| FND-003 | low | Two paths are not exercised by any test. (1) The `dead_limited` capture in the model-only pass: removing the insert passes the whole qsl-analyze suite. (2) The per-thread `each` class: every fixture has one thread per branch, so `Whole` instead of `Each` is caught only by the set-shape assertion, not by any filter or engine behaviour. | qsl-analyze/src/model_check/liveness.rs:1870-1872, qsl-eval/src/simulation/protocol/fairness.rs:247-266 |

## Verdict

Mergeable once FND-002 is decided. That means one of two things: the lead accepts the interim wire shape, or the shape is agreed with draco:QSL:codex3 for #1442. FND-001 should be fixed in this PR with a test: a tso liveness claim that is false, run with `max_store_buffer` 1 and a cut store, must settle `MemoryBoundReached` and not `Violated`. FND-003 is optional.

Soundness of the fairness semantics, as checked:

- Memory steps have no `thread()`, so they never count for a scheduler `Branch` or `Root` class, as MF-4 requires.
- The flush member's class is the full `ThreadPath` of the flushing thread, ordinal included. It is matched on the last `PathStep`, the same way the scheduler branch member is, so `each` gives one class per thread b. Under `fair weak each`, a lasso is unfair exactly when some thread's flush stays enabled through the loop and is never taken. A buffer stays non-empty until its own flush, so this is FR-436's "some enabled flush(w) is never taken".
- `governing` covers every `parallel`, nested ones included, and nested ones inherit the outermost model. Weak attempts exist only under weak `parallel`s (`access::classify`), so no buffered thread is left without a member. Compensation threads have no branch step and run no weak attempt.
- A real counterexample is never wrongly discarded in silence. With any bound-limited state, the run settles the inconclusive `MemoryBoundReached`, which names the bound and the state count. This is conservative: it also withholds a lasso that never touches a cut state. The PR lists this as spec mismatch 3. First-phase and dead-phase hits are finite `UndefinedEvaluation`/undecided prefixes, and they still return before the bound check. That is correct, because a finite witness does not depend on cut stores.

Replay reorder:

- A payload with valid steps and a tampered fairness set still refuses `ProtocolFairnessSet`.
- A payload whose steps do not replay now refuses at the first differing step, `stale_dependency`/`content-mismatch`, as FR-227-AC-1 needs once the sc fairness set lacks the memory members.
- `replay_prefix` returning `None` still goes through the fairness check first.
- No acceptance path changed. The only change is which refusal wins when both defects are present. All qsl-replay and qsl-analyze tests pass, including `a_tso_counterexample_replays_only_under_tso`.
- FR-227-AC-6 is ra, and the ra fairness set is unchanged.

ra.rs and protocol.rs:

- The ra change is a signature change only. It still returns an empty set, and ra liveness is still refused `WeakLiveness`, including with `activation on each`.
- `reported` is the identity for an all-sc subject, so lifted sc liveness steps are unchanged.
- Public API changes: `MemoryModel::fairness(&self, &[MemoryBranch]) -> Vec<ProtocolFairness>`, the new `MemoryBranch` export, and the new `ProtocolTarget::MemoryStep` and `FairnessOrigin::Memory` variants. `cargo check --workspace --all-targets` passes, so no other crate (CG, driver) breaks.
- For tso with `activation on each`, the refusal is now `InstanceLiveness` where it was `WeakLiveness`. Both are `unsupported_construct`/`not-yet-implemented`.

Panic and string-edge: the production hunks add no unwrap, expect, indexing or casts. `string-edge` is clean.

Gates run at 2c2b756cb, in their own target dir and one at a time:

- `cargo test --locked -p qsl-eval -p qsl-replay -p qsl-analyze`: all pass (qsl-analyze it 758, qsl-replay 846, qsl-eval 709 + 122, and more).
- `cargo check --workspace --all-targets`: passes.
- `cargo clippy -D warnings` on the 3 crates: passes.
- `cargo fmt --all --check`: passes.
- `xtask string-edge`: clean.

The coder's mutations, re-run:

- `Tso::fairness` returning empty: 6 of 7 tests fail.
- Memory-step `class_of` returning `None`: 4 fail.
- `BoundLimited` finding removed: 1 fails (the bound test).

Additional mutations:

- MB-4 order swapped: survives (FND-001).
- `dead_limited` insert removed: survives (FND-003).
- `Each` changed to `Whole`: 1 fails.

## Dispositions

Disposition round 1, verified at head 33ee3220528f38283546795651ebed5f4c706dfd (fix commit 33ee32205; origin/main merged in 41082b7af). Gates at the head: qsl-eval, qsl-replay and qsl-analyze tests pass, clippy `-D warnings` passes on the 3 crates, fmt and string-edge are clean.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | New test `a_false_liveness_claim_with_a_cut_store_settles_inconclusive`: the same false claim settles `Violated` under the default bound and `MemoryBoundReached` under `max_store_buffer` 1. Re-run mutation: returning `Violated` ahead of the bound check now fails it. |
| FND-002 | accepted-no-change | Lead decision: the wire shape stays interim, marked in the `ProtocolTarget::MemoryStep` doc comment as pending the FR-228/FR-436 ruling in spec PR #1442. The coordinator asked draco:QSL:codex3 for the settled shape and will merge the interim shape if there is no reply. A shape change after #1442 needs its own ticket. |
| FND-003 | fixed | New test `a_cut_store_met_only_by_the_model_only_pass_settles_inconclusive`: re-run mutation, removing the `dead_limited` insert now fails it. New qsl-eval unit test `a_flush_member_is_one_constraint_per_thread_of_its_branch`: `Each` changed to `Whole` now fails it. The per-thread check is at class and loop-filter level, not through the engine. That is accepted, because two threads alive in one branch occur only across protocol instances, which liveness refuses, and sequential forks reuse the ordinal after a join, so they share one ThreadPath. |
