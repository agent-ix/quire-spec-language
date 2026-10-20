---
id: SR-12239
title: "Code review of quire-spec-language PR #1120 (QSL-910): request memory selection, EN-1 access classification, ProtocolSystem<Tso>"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@8b46777865c545628c49d8a013a156d110e098c7; PR #1120 diff origin/main...HEAD: qsl-eval/src/simulation/protocol/{tso,access,selection,memory,steps,identity,program}.rs, qsl-eval/src/simulation/protocol.rs, qsl-eval/src/simulation/model/cells.rs, qsl-eval/src/simulation/subject.rs, qsl-analyze/src/model_check/check.rs, qsl-replay/src/{certificate/closure,model_check/replay,model_check/settle,model_check/outcome,outcome,proof_result,lib}.rs, qsl-semantics/src/check/protocol_memory.rs, qsl-analyze/tests/it/{weak_memory,litmus,protocol_footprints,protocol_system,fill}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-221
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-225
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-219
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: reviews
---
# Code review of quire-spec-language PR #1120

## Summary

Ticket: QSL-910 (remainder of QSL-565). Ticket and comment text were read as
data. The scope is the PR diff only. This file includes the `rust-review`
lane.

What was checked first, for soundness, against ADR-025 and ADR-027 SE-1 to SE-5:

- **TSO store buffers (TSO-1 to TSO-5, TSO-7).** These are correct. Each
  thread of a `tso` `parallel` has a FIFO buffer (`Vec`, push to the back,
  flush from the front). `observe` lays the thread's own buffer over memory,
  so the newest entry wins. A store captures the post value and puts memory
  back to its pre value. An RMW, and a `seq_cst` store, are locked
  (`Written::Disabled` while the buffer is non-empty) and write memory
  directly. `flush(b)` has a typed identity. Fork and send wait for an empty
  own buffer, and join waits for every buffer below it. Join removes the
  branch buffers. Atoms read memory (MS-2). MA-5 (an RMW that leaves its
  location unchanged applies as a load) reads the thread observation.
- **Bound (MB-3, MB-4).** The engine is correct. A store over the bound is
  `Written::Cut`, and its state is counted as bound-limited. `Run::decide`
  settles `MemoryBoundReached` before `BoundReached` and never reports `Holds`
  when `bound_limited > 0`. A counterexample still settles V-4. The
  independent closure checker does not hold this property: see FND-001.
- **POR / footprints.** These are sound today. Protocol runs are unreduced
  (`reducer: None`, `chooser: None`), and nothing consumes protocol
  footprints in a reduction. Over `Tso`, `footprint()` refuses a memory
  step (`UnknownStep`). Attempt footprints over-approximate: a buffered
  store still reports a write of `mem(ℓ)`. So no independence or
  invisibility claim is too strong. The commutation test is not extended to
  TSO steps (FND-002).
- **Effective model in the identity.** `ProtocolSubject.memory:
  ResolvedMemory` writes every outermost `parallel` explicitly, with
  defaulted and selected models alike. No obligation-identity digest exists
  yet for protocol subjects (there is no request wire), so it reaches only
  the subject member. The PR discloses this, and the gap analysis records
  it.
- **Replay (MK-2).** `replay_protocol_trace` and `check_protocol_closure`
  rebuild the system under the subject's resolved models and bound. A `tso`
  trace replayed under `sc` refuses `stale_dependency`/`content-mismatch`,
  and a test covers that.
- **Access classification never makes shared local.** Sharing follows MA-1:
  touches by attempts in different branches of one `parallel`, at least one
  a write, with `affects` used for any-object locations. Classification
  partitions on that set. `attempt_identities` is state-free over the
  universes (absent rows included). A weak instance that EN-1 did not
  classify faults (`a-weak-attempt-application-is-one-classified-access`)
  instead of applying as local.

Rust lane: I found no `unwrap`/`expect`/`panic!` and no lossy `as` cast in the
added production code. The bound comparison uses `u64::try_from`.
`WeakAttempt::instance` does a binary search over a sorted list. There is no
compatibility layer. Arch-lint FR-100-AC-8 scans `qsl_replay::spine` only, and
the PR does not touch it. The new `MemoryCause` and `WeakAccessShape` types are
not in that surface.

Overlap with open PR #1119: #1119 adds `GatedStep::Fence`. `Tso::gate` matches
exhaustively, so the merge will not compile until a TSO-6 arm is added (a
`seq_cst` fence waits for an empty buffer, other fences pass). This is a
merge-order task, not an incompatibility.

## Verdict

Major (blocking): FND-001. The closure checker certifies over successors
that a memory bound pruned, so a certificate closed only under the bound
passes as `proved`. Everything else is medium or low, and the core TSO
semantics are correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The closure checker ignores memory-bound cuts. `check_protocol_closure` builds `ProtocolSystem<Tso>` under the request's `max_store_buffer`, but `check_system`'s `expand` calls `TransitionSystem::successors`, which is `expand_bounded(..).0` and drops the cut flag. A certificate whose state set is closed only because stores were cut is accepted, and the item settles `Proved{Certified}`. That contradicts MB-4 and MV-1 (a run with a bound-limited state is never proved), and the closure checker is exactly the trust anchor that should catch an engine that emits `Holds` over a cut. The probe in this review: a `Holds` certificate checked under `max_store_buffer` 1, where the engine itself settles `MemoryBoundReached`, is accepted as `Proved{Exhaustive, Certified}`. The checker sees the cuts and does not react to them. Fix: in the closure check, expand through `SubjectSystem::successors_bounded` and reject (or settle `MemoryBoundReached`) at any bound-limited state, and add a test. | qsl-replay/src/certificate/closure.rs:560-566; qsl-eval/src/simulation/protocol.rs:1138-1143; qsl-replay/src/certificate/closure.rs:248-278 |
| FND-002 | medium | No MR-1 memory footprints over `ProtocolSystem<Tso>`, and the commutation test is not extended to TSO. `footprint()` returns `UnknownStep` for `flush`, and an attempt's footprint ignores `buf(b)` and `mem(ℓ)`. `every_independent_pair_of_steps_commutes` runs only `sc_system` fixtures (this PR adds only the `memory` field), so no TSO step or flush is checked for commutation. No reduction reads protocol footprints today, and flush fails closed, so no result is wrong now. But the brief's soundness check (the commutation test extended to TSO steps, and non-vacuous) is not met. The module doc still says `sc` is "the one memory model it steps". | qsl-eval/src/simulation/protocol/footprint.rs:43-45; qsl-eval/src/simulation/protocol/footprint.rs:455; qsl-analyze/tests/it/protocol_footprints.rs:594-686 |
| FND-003 | low | `fork` falls back to `MemoryModelName::Sc` when `governing` has no entry for the forked `parallel`. If that invariant broke, a `tso` branch would get no buffer and run with `sc` semantics, which silently drops TSO behaviours (and so could give a wrong `proved`). It should be `ProtocolRefusal::fault(..)`, like the other invariant breaks in this file. | qsl-eval/src/simulation/protocol/steps.rs:508-513 |
| FND-004 | low | `settle` swallows encoding failures for the `weak-access-shape` cause. The transition text goes through `.ok()`, so a failure gives `transition: None`. The shape goes through `shape_text().unwrap_or_default()`, which gives an empty string, and the wire's `serde_json::from_str(shape).ok()` then drops `shape`. A canonical-encoding failure should fault, not emit a degraded `unsupported` record. | qsl-replay/src/model_check/settle.rs:263-270; qsl-replay/src/outcome.rs:465 |
| FND-005 | low | `bound_limited` counts every expansion of a product node that saw a cut, not each distinct bound-limited model state. Under a safety automaton with several monitor states for one model state, `MemoryBoundReached.states` over-counts what MB-4 calls "the count of bound-limited states". | qsl-analyze/src/model_check/check.rs:1295-1303 |

## Dispositions

Round 1. Reviewed at 7ff5a7d1acf4cd74e3e769029086101d62dc6fc3; the fix commits are acb5ad013 and 0036d094e. Narrow tests ran via `locked-build.sh`: weak_memory, protocol_footprints and protocol_system, 51 passed. I ran two mutation probes on the new TSO footprints. In the first, a flush no longer writes the shared `mem(ℓ)` locations: `every_independent_pair_of_tso_steps_commutes` fails, so the check is not vacuous. In the second, a plain store no longer writes `buf(b)`: the test still passes, which is expected. The store already reads `buf(b)`, and the only other step that touches `buf(b)`, `flush(b)`, writes it, so the pair stays dependent. That write is redundant, not a soundness gap.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed acb5ad013 | `check_system` now expands through `successors_bounded` and records distinct bound-limited model states. Any reached cut returns `TemporalCertificateCheck::MemoryBoundReached`, which settles inconclusive and is never proved. A rejection or a stop still takes precedence. A run with no bound never cuts, because the default `successors_bounded` returns `false` and `limited` stays empty. So the boundary-state rule cannot hide a deadlock in a run without a bound. A cut with no bound faults. A test covers a bound-4 `Holds` certificate checked under bound 1. |
| FND-002 | fixed acb5ad013 | MR-1 is implemented through `MemoryModel::footprint`. A buffered thread's attempt reads and is enabled by `buf(b)`. A plain store writes `buf(b)` and not `mem(ℓ)`. A load or a locked access keeps its `mem(ℓ)` reads and writes. Fork and send are enabled by `buf(b)`. Join reads, writes and is enabled by every buffer below it. A flush reads and writes `buf(b)` and writes every shared `mem(ℓ)`, an over-approximation. The commutation check covers the tso SB, FIFO, MP and send fixtures, and independent flush pairs are asserted to exist (`flushes > 0`). |
| FND-003 | fixed acb5ad013 | A fork with no governing entry now faults with `a-forked-parallel-has-a-model`. |
| FND-004 | fixed acb5ad013 | `weak_access_shape` returns `None` when the text cannot be encoded, and the row becomes `Row::failed()`. The wire writes the canonical shape through `RawValue`, so a parse failure fails the serialization instead of being dropped. |
| FND-005 | fixed acb5ad013 | `bound_limited` is now a `BTreeSet<DigestRecord>` of distinct model-state digests. |

Note, not a finding: `check_system` now takes 8 arguments under `#[allow(clippy::too_many_arguments)]`. The crates already carry 20 such allows, so the repository's convention admits it.
