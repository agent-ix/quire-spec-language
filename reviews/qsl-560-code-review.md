---
id: SR-12247
title: "Code review (Rust lane) of QSL-560 E15c protocol scheduler fairness filter and lasso replay (PR #1134)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@39307eec1; git diff origin/main...HEAD: qsl-eval/src/simulation/{protocol.rs,protocol/fairness.rs,protocol/identity.rs,protocol/memory.rs,subject.rs,mod.rs,model/encode.rs}, qsl-analyze/src/model_check/{liveness.rs,check.rs,concretise.rs}, qsl-analyze/tests/it/{scheduler_fairness.rs,model_settle.rs,main.rs}, qsl-replay/src/{model_check/replay.rs,model_check/counterexample.rs,model_check/outcome.rs,model_check/settle.rs,spine.rs}, qsl-semantics/src/check/{temporal_clause.rs,refusal.rs,mod.rs,hyper_clause.rs,temporal_clause/tests.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-212
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-217
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: reviews
---
# SR-12247: Code review (Rust lane) of QSL-560 E15c protocol scheduler fairness filter and lasso replay (PR #1134)

## Summary

Ticket: QSL-560. PR quire-spec-language#1134, branch qsl-560-impl, head 39307eec1. The review covers `git diff origin/main...HEAD` only: 23 files, one feature commit and one merge of main. The rust-review lane is folded into this file.

No gates and no full builds were run, per the brief. One focused run, `cargo test -p qsl-analyze --test it scheduler_fairness`, went through the build lock. Its result is in the Verdict. `target/` was deleted afterwards.

## Verdict

**Mergeable. No major findings.** Two mediums and five lows are recorded below.

Soundness first:

- **No false `proved`, no false counterexample from the filter.** `ProtocolFairness::class_of` gives each transition at most one class per member. An operation member covers the `attempt` steps whose FR-120 application names the operation: `whole` is one class, and `each` is one class per application, as QSpec FR-430 states. A thread member covers the steps that carry a thread path. The root covers paths with no step. A branch covers paths whose last step is that `parallel` and label, so nested-parallel threads are excluded (PA-1). Channel and memory steps carry no thread and are in no class. `protocol_constraints` always makes the `whole` class, even when it is empty. An empty constraint is never enabled, so it cannot exclude a run. It makes one `each` constraint per class seen. This matches `fair_walk::fairness_constraints` for model subjects. `FairModel::build` numbers every edge label and every enabled identity, so the kernel's enabled test and taken test (FA-3/FA-4, per pair) read the same classes the replay reads. `Strength::from(kind)` carries weak and strong through unchanged. Scheduler members are always weak and `whole`.
- **Adversarial.** `scheduler_constraints` returns no members under `CheckedScheduling::Adversarial`. The AC-2 test confirms that the starving lasso is then admitted. With the default set, the same lasso is unfair (`r` and `s` are enabled throughout and take no step).
- **Model-subject behaviour is unchanged.** Each changed model path was traced:
  - `ModelSystem::canonical` is `Some(canonicalise)`.
  - `atom_state` borrows the state.
  - `undefined_finding` over a model finding is `undefined_contract` (`undefined_protocol_expression` is protocol-only).
  - `Lift::start` with a trivial group takes the `None` permuter branch, as `reduced == false` did.
  - The POR, quotient and symmetry paths move behind `LiveSubject` with their bodies unchanged.
  - Identities are numbered over `ModelTransition`'s unchanged `Ord`.
  - `found_at` now fires on any `Finding::Contract`, but the contract findings are already filtered to undefined or refused ones, so the result is the same.
  - The only observable model-side change is FND-006, a refusal-precedence change.
- **Replay.** Protocol replay recomputes the fairness set from the recompiled clause and protocol through the same `fairness_set`. It refuses a mismatch with `stale_dependency`/`content-mismatch`, closes the loop with the now-generic `close_loop`, and checks loop fairness through the same `protocol_constraints` and `check_loop` kernel the engine uses. `UnfairProtocolLasso` settles `inconclusive`/`ReplayParity`, mirroring the model `UnfairLasso`. The old `ProtocolLasso` refusal is deleted, not shimmed.
- **Canonical order.** The set is keyed by `quire_canonical::to_vec` bytes in a `BTreeMap<Vec<u8>, _>`, which is ascending JCS bytes. An authored member and a scheduler member differ by origin, so both are kept ("beside"). Encodings match FR-430: `thread-target` with `node`/`branch` both null for the root.
- **`Ord` for `ProtocolTransition`** compares `(kind, thread, node, members)`, which are exactly the fields `PartialEq` compares. `removed` and `send_record` are excluded from both, so `Ord` is consistent with `Eq`. The doc correctly says this is not FR-101's canonical order. That is harmless: the order is used only for dedup and for numbering, never for search order or counterexample choice.
- **Panic surface.** No new `unwrap`, `expect`, `panic!`, indexing or `unsafe` in production code. `u64::try_from(..).unwrap_or(u64::MAX)` is total.
- **No compatibility layer.** `TemporalCounterexample<T: SubjectTransition>` replaces the field type outright, and nothing keeps the old `FairnessConstraint` shape for protocols.
- **Arch-lint FR-100-AC-8.** No `qsl_replay::spine` public item or `qsl_replay` re-export names a new `qsl_eval` path. The spine change is a private `check_message` arm. `ModelTraceRefusal` already carried `qsl_eval` types (`ProtocolRefusal`) and is outside the spine rule.
- **Focused run.** `scheduler_fairness`: 5 passed, 0 failed at 39307eec1.

## Examined Scope

- `qsl-eval/src/simulation/protocol/fairness.rs` (whole file): targets, origin, `class_of`, encoding, `scheduler_constraints`, `fairness_set`, `protocol_constraints`, unit tests.
- `qsl-eval/src/simulation/protocol/identity.rs`: `Ord`/`PartialEq` for `ProtocolTransition`, `Members` derive.
- `qsl-eval/src/simulation/subject.rs`: `SubjectTransition`.
- `qsl-analyze/src/model_check/liveness.rs`: `check_protocol`, `decide_bindings`, `LiveSubject` and both impls, the `LassoSystem` generalisation, `FairModel::build`, `lasso`, `quotient_phase`.
- `qsl-analyze/src/model_check/check.rs`: `check_protocol` dispatch, `check_protocol_liveness`, `ProtocolRun::decide` fairness, `Run.fairness`.
- `qsl-analyze/src/model_check/concretise.rs`: `Permuter`, generic `Lift`.
- `qsl-replay/src/model_check/replay.rs`: `ProtocolFairnessSet`, `UnfairProtocolLasso`, `unfair_member`, generic `close_loop` and `loop_states_of`, the moved model fairness check.
- `qsl-replay/src/model_check/settle.rs`: `UnfairProtocolLasso` → ReplayParity.
- `qsl-semantics/src/check/temporal_clause.rs`: `resolve_protocol_target`, `checked_fairness`, `is_emitted`, `not_emitted`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A protocol deadlock counterexample now carries the scheduler constraints in `fairness`. `ProtocolRun::decide` builds `system.fairness_set(self.item.fairness())` for every item, and `Item::Deadlock` yields `&[]` authored plus the root and branch scheduler members. That contradicts the retained `TemporalCounterexample::fairness` contract ("empty for the deadlock-freedom item") and the `Run.fairness` doc in check.rs. FR-430 adds scheduler constraints to infinite-trace clauses, not to the derived deadlock item. Protocol replay checks the set only when `self.clause.is_some()`, so a deadlock payload's fairness set is never verified either way. Fix: use an empty set for `Item::Deadlock`, or state and test the other rule. | qsl-analyze/src/model_check/check.rs:643-650, qsl-replay/src/model_check/counterexample.rs:149-151, qsl-replay/src/model_check/replay.rs:639-650 |
| FND-002 | medium | The `WeakLiveness` refusal has no test. This is the soundness guard that keeps a TP-4 clause over `tso`/`ra` out of an engine that reads no memory fairness (FR-228), where an unflushed store buffer would give a false counterexample. A regression in `SubjectMemory::of`, or a reordering of `check_protocol_liveness`, would pass every test in this PR. | qsl-analyze/src/model_check/check.rs:606-609 |
| FND-003 | low | Replay maps `ProtocolFairnessError::UnknownOperation` to an internal fault (`a-recompiled-fairness-names-an-applied-operation`). The check path maps the same error to the user-facing `LivenessRefusal::UnknownOperation`. A crafted payload against a package whose clause names an operation the protocol's model does not apply faults instead of refusing. | qsl-replay/src/model_check/replay.rs:639-643 |
| FND-004 | low | The `ProtocolFairnessError` → `ModelCheckRefusal` mapping closure is duplicated verbatim in `check.rs::ProtocolRun::decide` and `liveness.rs::check_protocol`. A `From<ProtocolFairnessError> for ModelCheckRefusal` would keep the two in step. | qsl-analyze/src/model_check/check.rs:645-651, qsl-analyze/src/model_check/liveness.rs:284-292 |
| FND-005 | low | S3 resolves `CheckedProtocolTarget::AttemptNode`, but `qsl_eval`'s `ProtocolTarget` has no attempt-node variant and no class for it (FR-430 "every step of that node", `each` one per (thread path, application)). It cannot be reached today because S4 refuses the clause, but the eval side of FR-212 "Classes" for attempt nodes is unbuilt, and the PR body does not list it. | qsl-eval/src/simulation/protocol/fairness.rs:34-52, qsl-semantics/src/check/temporal_clause.rs:399-417 |
| FND-006 | low | Model replay's fairness-set check moved out of `resolve_item` into the `replay_model_trace` closure, after `resolve_item`'s `AuthoredUndefined` and `GuardedActivation` refusals. A model payload with a stale fairness set and an `UndefinedEvaluation` kind now refuses `unsupported_construct` instead of `stale_dependency`/`content-mismatch`. The change is small, but it is a model-subject behaviour change. | qsl-replay/src/model_check/replay.rs:524-535, qsl-replay/src/model_check/replay.rs:800-810 |
| FND-007 | low | Four `#[allow(clippy::type_complexity, reason = ...)]` sit on `LiveSubject::retain`, `retained`, `LassoSystem::successors` and `loop_states_of`. A `type Exploration<S> = GraphExploration<LState<S::State>, PT<Tr<S>>, Finding<Tr<S>>>` alias, beside the existing `Retained<S>`, would remove three of them. | qsl-analyze/src/model_check/liveness.rs:272-298, qsl-replay/src/model_check/replay.rs:1294-1297 |

## Coverage

Focused run `cargo test -p qsl-analyze --test it scheduler_fairness` through `locked-build.sh`: 5 passed, 0 failed. Unit tests in `fairness.rs` (2) were read, not run. `target/` deleted after.
