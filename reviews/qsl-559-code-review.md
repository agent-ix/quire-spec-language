---
id: SR-12223
title: "Code review (Rust lane) of QSL-559 E15 PR 1 ProtocolSystem<Sc>: sequence, parallel, attempt, finish, finite replay (PR #1102)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@5e4ce0ded; qsl-eval/src/simulation/protocol{.rs,/identity.rs,/memory.rs,/program.rs,/steps.rs}, qsl-eval/src/simulation/{subject.rs,model.rs,model/application.rs,model/frames.rs,model/walk.rs}, qsl-eval/src/value/expression/{mod.rs,evaluate.rs}, qsl-eval/src/model_subject.rs, qsl-semantics/src/check/{protocol_clause/types.rs,protocol_clause/mod.rs,protocol_slots.rs,protocol_memory.rs,ir.rs}, qsl-analyze/src/model_check/check.rs, qsl-replay/src/{model_check/replay.rs,model_check/settle.rs,model_check/counterexample.rs,certificate/closure.rs}, qsl-route/src/deadlock.rs, qsl-analyze/tests/it/{protocol_system.rs,fill.rs}"
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
# SR-12223: Code review (Rust lane) of QSL-559 E15 PR 1 ProtocolSystem<Sc>: sequence, parallel, attempt, finish, finite replay (PR #1102)

## Summary

Ticket: QSL-559. PR quire-spec-language#1102, branch qsl-559-impl, head 5e4ce0ded. The review covers `git diff origin/main...HEAD` only: 42 files, 5 commits, two of them merges of main. The rust-review lane is folded into this file.

No gates were run, per the brief. One narrow probe was run with `cargo test -p qsl-analyze --test it` through the build lock. A temporary test built `P` over a flat `sequence` of n always-enabled attempts and timed S3 and `ProtocolSystem::new` (debug build):

| n | S3 check | `ProtocolSystem::new` | VmHWM |
| --- | --- | --- | --- |
| 500 | 0.66 s | 0.17 s | 49 MB |
| 1000 | 2.6 s | 0.95 s | 71 MB |
| 2000 | 9.2 s | 3.3 s | 145 MB |
| 4000 | refused at S1 (`StageLimit(TokenCount)`) | - | - |

The probe was reverted and `target/` deleted.

## Verdict

**Mergeable. No major findings.** Four mediums and seven lows are recorded below.

What is sound:

- **Transition system.** `ProtocolState` is the model state, the live instances (threads, parallel instances, binders) and the memory component (PS-8). `expand` enumerates every running thread's step, and `successors` is FR-101's. The engine (`order::expand`, `check.rs` `transition_key` sort) orders successors by canonical identity bytes, so search order and counterexample choice are deterministic.
- **Interleaving.** Each running thread contributes its own enabled step at every state, and settling (`Program::settle`) folds structural moves into the step. A sequence child is entered only after its predecessor's step, and a branch thread exists only after `fork`. Join, finish and the attempt precondition are the only cross-thread orderings, so the behaviours are the linear extensions (CI-1 to CI-3). The FR-210 tests confirm this for 3!, the sequence-in-branch case, and the split shipments.
- **Attempt.** Each attempt goes through `ModelSystem::apply_operation` → `frames::apply`. That is the same function `expand_state` now calls with `select = |_| true`, so there is no second contract evaluator (TS-3). The selected clauses come from `Walk::selected`. The binder constraint is evaluated through `evaluate_protocol_slot` on the one clause machine, and an undecided constraint becomes a `ProtocolExpressionUndetermined` finding that settles V-6 or UE-6.
- **Join and finish.** `join` is enabled by the policy over the joined labels' dispositions (`all`, `any`, `quorum(n)`), and `predicate` is refused up front. Under `cancel` the step cancels the running branch threads, and `normalize` removes them. `finish` needs every other thread of the instance to have left `running` (ST-9) and removes the instance (AE-3).
- **Key.** `control`, `observations`, `roles`, the empty `queues` and `bounds` maps, and `memory` are built through `map_enc`, which sorts by JCS bytes. PS-4 and PS-5 removal runs in `normalize` after every step. The parallel-instance ordinal is the smallest one not live, which is PS-2's reuse rule. That is the shape PR 2's `repeat` needs.
- **Replay.** `replay_protocol_trace` recompiles, checks `package_id`, re-admits the subject and rebuilds `ProtocolSystem<Sc>`. It re-executes every step by identity plus digest, refuses a disabled step `invalid-value`, and refuses a protocol lasso `not-yet-implemented`. For a deadlock it recomputes the class (no live instance → not deadlocked) and the blocked threads, so a mismatch settles `ReplayParity`. It reads no path, environment or clock.
- **Panic surface.** No `unwrap`, `expect`, `panic!`, `todo!` or unchecked indexing was added in production code, and no `unsafe`. Settling is iterative and bounded by twice the control count. Reachability and reach sets are built iteratively, so deep nesting cannot overflow the stack. Fan-out is bounded by the engine's state limit, budget and cancel poll.
- **Extensibility.** `Rest`, `ControlStep`, `StepKind` and `WaitCause` are closed enums matched exhaustively. `Instance` and the key already reserve `queues` and `bounds`. `instances: BTreeMap<u64, Instance>` admits `on each`. Channels, timeouts and repeat (PR 2) and roles and `on each` (PR 3) extend these without restructuring. The exceptions are FND-002, FND-003 and FND-008.
- **Rename.** The `ReplayPackage` alias (formerly `Recompiled`) is fine. It removes the name clash the TC-244 stage-type scan saw with `execute::Recompiled`.
- **Arch-lint and string-edge.** The PR body states that both ran in the gate on the merged head. Nothing in the diff adds a string-matched code or a new cross-layer import beyond `qsl-replay` → `qsl-eval::simulation::protocol`, which follows the existing `simulation::model` import pattern.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `outstanding cancel` cancels only the parallel instance's direct branch threads. Suppose a cancelled branch thread had forked an inner `parallel`. Its inner branch threads stay `running` and keep taking attempts that write the model after their ancestor was cancelled. The inner parallel instance, whose owner thread `normalize` removed, is never settled, so it stays in `control` until `finish`. Scenario: `parallel P { branch a attempt X; branch b parallel Q { branch c attempt Y; branch d attempt Z } join all [c, d] } join any [a, b] outstanding cancel`. After `fork(P)`, `fork(Q)` by `b` and `attempt(X)`, `join(P)` cancels `b`, yet `attempt(Y)` and `attempt(Z)` stay enabled. QSpec FR-426's `join` row says "every branch thread still running" and does not address descendants, so the spec and the code need one ruling. | qsl-eval/src/simulation/protocol/steps.rs:264-272, qsl-eval/src/simulation/protocol/steps.rs:454-486 |
| FND-002 | medium | `ProtocolTransition`, `BlockedThread` and `WaitCause` are defined in `qsl-eval`. FR-215 "Outputs" states that `quire-semantic-value` owns `ProtocolTransition`, because the compiler produces it and CG reads it in counterexamples, and QSL-559 "Touches" names all three as SV types. CG cannot read a protocol counterexample without depending on `qsl-eval`. Moving the types later changes every generic signature this PR adds in `qsl-analyze` and `qsl-replay`. | qsl-eval/src/simulation/protocol/identity.rs:15-358 |
| FND-003 | medium | Binder scoping is per node, not per thread. `frame` sorts every binder of a node by (own thread, ancestor, other), takes only the first, and drops it unless it is an ancestor or descendant. A sibling-instance binder ranked first therefore hides a valid descendant one, and between two descendant binders (ordinal 0 still running, ordinal 1 just joined) it takes the lower `ThreadPath`, not the instance the reader joined. `normalize` keeps a binder while any running thread can reach any reader of its node, even a thread that cannot see that binder, which over-retains `observations` against QSpec FR-425's key. This cannot be reached in PR 1, because each node is bound by at most one thread per instance. It becomes reachable as soon as PR 2's `repeat` reuses ordinals (FR-205-AC-3). | qsl-eval/src/simulation/protocol.rs:640-670, qsl-eval/src/simulation/protocol/steps.rs:487-501 |
| FND-004 | medium | `replay_protocol_trace` maps a failed requires-bound pre-check to `fault("a-checked-protocol-subject-is-bounded")`, which is `runtime_invariant`/`established-invariant-broken`. A replay request is untrusted input, and its unit is recompiled from source. A request whose protocol has a ranged binder of unbounded type (for example `finish Done as (d: Int)`) gets an internal-invariant refusal instead of the typed `RequiresBound`, or a `ModelTraceRefusal`, that `check_protocol` and `check_protocol_closure` give for the same unit. | qsl-replay/src/model_check/replay.rs:455-456 |
| FND-005 | low | `Program::reachability` builds an `after` set and a `reach` set per control by cloning the parent's set and extending it with later siblings' subtrees. That is O(n²) in time and memory for a flat sequence, it is unmetered, and it does not poll cancel. Measured: n=2000 attempts takes 3.3 s, and at every n it costs about a third of S3's own time. S1's token limit refuses 4000, so the input bound upstream holds it. A reader-oriented check (is a reader after the rest point in tree order within scope) would be linear. | qsl-eval/src/simulation/protocol/program.rs:386-453 |
| FND-006 | low | FR-215 "The system" says `successors` enumerates in ascending JCS bytes of the identity. `ProtocolSystem::expand` returns thread order (derived `Ord` on `ThreadPath`), and only the engine sorts. The derived order compares the ordinal as `u64`, while JCS compares it as text, so `"10" < "9"`. The two orders therefore diverge once ordinals reach 10 (PR 2). The FR-215-AC-1 test asserts raw `successors` order, which holds today only because `left < right` in both orders. The order of contract and protocol-expression findings, and so which undefined evaluation UE-6 reports first, follows the same thread order. | qsl-eval/src/simulation/protocol/steps.rs:57-101 |
| FND-007 | low | `ProtocolRefusal::Unsupported(Unsupported { construct: &'static str })` carries prose payloads such as "a join that may leave branches outstanding with no `outstanding` rule", "an `over` input that is no object" and "a read of an attempt binder with no result", beside keyword payloads. A caller or test can only match the variant. Two of these are not unbuilt constructs but S3-shaped refusals. | qsl-eval/src/simulation/protocol/program.rs:217-244, qsl-eval/src/simulation/protocol.rs:439-443 |
| FND-008 | low | `frame` gives the `over` value to every slot whose binder has no node: the `over` input, but also the activation binder and protocol captures. Captures and `on each` are refused today. PR 3 must replace this before either is admitted, or a capture slot reads `over`. | qsl-eval/src/simulation/protocol.rs:640-646 |
| FND-009 | low | `ProtocolSystem::threads` and `binders` silently skip an entry whose node fails to name (`if let Ok` / `filter_map`). A broken program invariant then shows up as a missing thread in tests and blocked lists, not as a fault. | qsl-eval/src/simulation/protocol.rs:491-517 |
| FND-010 | low | `BOUND_POSITIONS` (`1 << 20`) is now declared in three places: `qsl-analyze` check.rs, `qsl-replay` closure.rs and `qsl-replay` replay.rs. Check, certificate and replay must classify the same roots under the same position limit, and nothing ties the copies together. | qsl-replay/src/certificate/closure.rs:257, qsl-replay/src/model_check/replay.rs:498 |
| FND-011 | low | `TemporalCounterexample::measured_bytes` counts `prefix` and `fairness` but not the new `blocked` list, so a deadlock payload's measured size under-counts its blocked threads and their paths. | qsl-replay/src/model_check/counterexample.rs:218-230 |

## New findings (disposition pass 1)

Disposition pass 1 at head cf4888b16. It holds fix commit e11ea8f4c and a merge of main that resolved a conflict in `qsl-replay/src/certificate/check.rs`. No gates were run. The fix commit and the merge resolution were read against the code. The resolution keeps one `BOUND_POSITIONS`, in `qsl-eval/src/simulation/not_simulated.rs`, and main's new `certificate/simulation.rs` imports it.

How the two rewrites were checked:

- **Reachability.** The new `reads_from` takes the preorder interval from the rest point to the end of its scope, which is the nearest enclosing `branch`, or the whole tree plus `finish` for the root. That interval holds the node's subtree, the later siblings of every enclosing `sequence` within the scope, and every nested branch body those contain. This is exactly the set the old `subtree ∪ after` computed. A `Join` rest starts one past the parallel's subtree, so the forking thread does not reach its own branches, as before. The test is one `BTreeSet::range` query per reader slot, so the quadratic precompute is gone.
- **Cancellation.** The join cancels every running thread whose path extends a cancelled branch, and settles every parallel instance whose parent is such a thread. `normalize` then removes those threads and instances (PS-3, PS-4). `a_cancelled_branch_takes_the_threads_it_forked_with_it` checks that no attempt of the inner `parallel` stays enabled. The ruling is in the PR body and the `join` doc comment.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | low | `not_simulated_code` maps each `NotSimulated` arm to a `Code` in a second table beside `NotSimulated::catalog_code`, and `ModelTraceRefusal::cause` reads the cause from `catalog_code`. The two tables must be kept in step by hand: a new `NotSimulated` arm, or a changed catalog code, can give a code and a cause that disagree. | qsl-replay/src/model_check/replay.rs:302-320 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e11ea8f4c: under `outstanding cancel`, `join` cancels every running thread whose path a cancelled branch prefixes, and settles the parallel instances those threads own, so `normalize` removes them. Tested by `a_cancelled_branch_takes_the_threads_it_forked_with_it`. The QSpec FR-426 reading is recorded in the PR body and the doc comment. |
| FND-002 | deferred | The types belong in quire-semantic-value, which is a cross-repo SV change outside this PR. Nothing reads the types outside `qsl-eval`, `qsl-analyze` and `qsl-replay` yet, so moving them later is mechanical. |
| FND-003 | fixed | e11ea8f4c: `frame` selects only binders the reader `sees` (its own, an ancestor's, or a descendant's), and faults `a-reader-sees-one-binding-of-a-node` on a second qualifying binding instead of choosing one. `normalize` keeps a binder only while a running thread that sees it `reads_from` a reader. PR 2's `repeat` must define the binding a reader sees before it lifts the guard. |
| FND-004 | fixed | e11ea8f4c: the replay pre-check failure is `ModelTraceRefusal::NotSimulated(Box<NotSimulated>)`, and `RequiresBound` maps to `invalid_runtime_input`/`invalid-value`. Tested by `a_replay_over_an_unbounded_protocol_subject_refuses_requires_bound`. |
| FND-005 | fixed | e11ea8f4c: per-rest reach sets are replaced by preorder spans with a scope end, plus one range query over each slot's reader positions. That is O(n) to build and O(log n) per query, and equivalent to the old reach (see the note above). |
| FND-006 | fixed | e11ea8f4c: `expand` visits running threads in ascending JCS bytes of their paths, and sorts successors by the JCS bytes of their identities. |
| FND-007 | fixed | e11ea8f4c: `Unsupported` is an enum of typed variants, with `construct()` for the diagnostic text. |
| FND-008 | fixed | e11ea8f4c: `Program::new` refuses `Unsupported::NodelessBinder` when any slot other than slot 0 reads a nodeless binder, and `frame` gives `over` to slot 0 only. |
| FND-009 | fixed | e11ea8f4c: `threads()` and `binders()` return `Result` and propagate a broken invariant. |
| FND-010 | fixed | e11ea8f4c and the merge cf4888b16: a single `pub const BOUND_POSITIONS` in `qsl-eval::simulation`. Every caller imports it, including main's `certificate/simulation.rs`. |
| FND-011 | fixed | e11ea8f4c: `measured_bytes` adds each blocked thread's inline size, its path steps, its rest node and the paths its cause names. |
