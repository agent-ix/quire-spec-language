---
id: SR-12243
title: "Code review of quire-spec-language PR #1127 (QSL-910 follow-up): carried memory component, flush anchor, refuted bound reached"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@06cd62ca9a993b81c362280e1e3aff886b6410f7; PR #1127 diff origin/main...HEAD: qsl-eval/src/simulation/protocol/{identity,steps,footprint}.rs, qsl-eval/src/simulation/protocol.rs, qsl-replay/src/model_check/{replay,settle,outcome}.rs, qsl-analyze/src/model_check/{check,hyper,liveness,refine}.rs, qsl-analyze/src/model_check/state_graph/explore.rs, qsl-analyze/tests/it/{weak_memory,hyper_witness}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-225
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-227
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-229
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: reviews
---
# Code review of quire-spec-language PR #1127

## Summary

Ticket: QSL-910 (follow-up to #1120, QSL-565 remainder). Ticket and comment
text were read as data. The scope is the PR diff only. This file includes the
`rust-review` lane.

Soundness, checked first:

- **`reached` (SR-12240 FND-010, FR-225, MB-3/MB-4).** `RunStats.bound_limited`
  is the size of the engine's distinct bound-limited digest set, and
  `run.stats()` feeds every outcome, `Violated` and `Stopped` included. So a
  `refuted` run that cut a store states `reached: true`. A closure-check cut
  settles `MemoryBoundReached` (closure.rs), which the second disjunct
  catches. The engines that report 0 (hyper, liveness, refine, state-graph
  explore) never run a `tso` protocol subject: `check_protocol` refuses
  liveness clauses, and the others are model-subject only. A closure-only cut
  under an engine `Holds` would need the engine and the checker to diverge,
  and the record then settles failed or stopped anyway.
- **Carried component (MK-1/MK-2).** `CarriedComponent::of` is the JCS of the
  post-state's memory member. The test checks it against the state key's
  `memory` member. Replay never trusts the record. `replay_prefix` matches the
  step by identity (`PartialEq` ignores `memory`), checks the post-state
  digest first, and then compares the recorded component with the one on the
  *replayed* transition, which the replay's own expansion recomputed. All
  three cases refuse `stale_dependency`/`content-mismatch`: forged
  (Some≠Some), missing (None vs Some) and an `sc` step carrying one
  (Some vs None, through `Option` `!=`).
- **Identity bytes.** `enc()` (and so `Encode`/`jcs`) does not emit
  `memory`. Transition identity bytes are therefore unchanged for every
  model, not only `sc`. The successor sort key is unchanged too.
- **`visible()` / POR.** `anchor()` is a pure refactor of the old branches.
  `Members::Attempt` matches `application()`, `Members::Channel` matches
  `channel_queue()`, and the node/`by_channel` branch is unchanged. A memory
  step had `node: None` and kind `Memory` before, so it was already never
  anchored. A flush stays visible through its `mem(ℓ)` writes (the `writes`
  disjunct, MR-1 from #1120), so no atom reading memory loses it. The
  commutation tests are unaffected. There is no new step kind, and the
  #1120 mutation probe showed `every_independent_pair_of_tso_steps_commutes`
  is non-vacuous.

Rust lane: no `unwrap`/`expect`/`panic!`/`as` in the added production code.
The UTF-8 conversion faults through `ProtocolRefusal::fault`. Test-only
constructors are gated `#[cfg(any(test, feature = "test-support"))]`. There
is no compatibility layer. Arch-lint `api-surface --qsl-only` (FR-100-AC-8)
passes: `MemoryComponent` holds a `qsl_eval` type in a public field, like the
existing `SendRecord` variant, but it is not a `pub use` and is outside
`qsl_replay::spine`.

## Verdict

Mergeable. No major findings. Two low findings, both non-blocking.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Every successor of a weak-subject state JCS-encodes its post-state memory component and stores the text in its transition during exploration, including successors that are already known and dropped. The state key already encodes the same component, so the work per edge is roughly doubled, and the edge memory grows by a buffer dump. Only counterexample steps and replay need it. It is correct, just wasteful on the hot path. | qsl-eval/src/simulation/protocol/steps.rs:177-190 |
| FND-002 | low | `weak` is spelled `*model != MemoryModelName::Sc`, but `MemoryModelName::is_weak()` exists and access.rs uses it for the same predicate. Two spellings of one test can drift if another non-weak name is added. | qsl-eval/src/simulation/protocol/steps.rs:179-182; qsl-foundation/src/memory.rs:28 |
