---
id: SR-645
title: "QSL-272 failure-domain review of the pinned sampler, provenance and requires-bound"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@84a691bf8beb9df40aa945c48e1e05d7f5fb070b; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md; spec/test-cases/TC-453-exploration-orders-successors-canonically-and-keys-states-by-jcs-bytes.md; spec/test-cases/TC-454-the-pinned-sampler-reproduces-its-vectors-and-sampled-traces-replay.md; spec/test-cases/TC-455-stopped-explorations-stay-incomplete-and-unbounded-requests-require-a-bound.md; qsl-foundation/src/selection.rs; qsl-semantics/src/family/requirements.rs; qsl-eval/src/simulation/sample.rs; quire-specification@0d53cf2d simulation-sampler.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: reviews
---

## Summary

Ticket: QSL-272. This review probes identity confusion in the sampler's
provenance, the step counter across steps with no draw, and unstated failure
paths.

What holds:

- NaN payloads and signed zeros are distinct keys and states. The recomputed
  NaN digests differ: a3d5ecff… and 62c344cc….
- The digest is never used to coalesce.
- Cancellation and limits never report `Exhaustive`.
- `requires-bound` is decided before any `TransitionSystem` call.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Nothing checks that the `DefinitionRef` the caller supplies names the generator that runs. The engine implements `quire.simulation.sampler/v1` revision `1-draft.1` and records the caller's `DefinitionRef` as provenance. `DefinitionRef::new` accepts any non-empty identity and version (selection.rs:59-67), and FR-101 states no refusal. Take a lock whose `definitions` pins `quire.simulation.sampler/v1` at a later revision, or a different digest. The trace records that revision while the draws come from `1-draft.1`. Another implementation that honours the revision produces different traces under "the same ecosystem lock", so FR-181-AC-2 fails and the provenance is false. Fix: `sample` refuses, with a typed cause and before any draw, a `DefinitionRef` whose identity or revision is not `quire.simulation.sampler/v1` / `1-draft.1` (and whose digest is not the pinned raw-byte digest, if QSL pins it). Add a TC-454 step for it. | spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:55-56,104-106,134; qsl-foundation/src/selection.rs:59-67 |
| FND-002 | medium | No vector pins the step counter across steps with no draw. simulation-sampler.md defines `step` as "the transition's 0-based index within that trace", so steps with `n = 1` still advance it. FR-101 says only "step `s`" (:94) and does not define it. Every vector draws at every step with n ≥ 3, and TC-454 step 3 has `n = 1` throughout. So an engine that advances `step` only when it computes a digest passes all of TC-454 while diverging from QSpec on any mixed trace. Fix: define `s` in FR-101 as the 0-based transition index within the trace, counting steps with `n = 1`. Add a mixed vector: seed `424242`, trace `0`, `n = 1` at steps 0 and 1, then `n = 5` at step 2, selects index `4`. Recomputed: it is step 2 of the TC-210 sequence. The wrong counter uses step 0 and selects `0`. | spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:93-100; spec/test-cases/TC-454-the-pinned-sampler-reproduces-its-vectors-and-sampled-traces-replay.md:18-22,33-38 |
| FND-003 | low | Three failure paths are not stated. (1) `classify_extent` can fail with a node-count stage limit or an internal fault (requirements.rs:215-240, `ClassifyFailure`). FR-101 does not say what the simulation entry returns then; it must be neither `RequiresBound` nor an exploration. (2) Sampling a system with no initial states (today `EmptyInitial`, sample.rs) and exploring one are in no AC. (3) AC-4 says "`m > 1` initial states", while Behavior says "distinct", and TC-453 step 4 and TC-454 step 6 include no duplicate initial state. So `t mod m` over the raw count, before coalescing, passes both. Fix: add the three to FR-101 and add a duplicate initial state to TC-454 step 6. | spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:100-103,112-120,134; qsl-semantics/src/family/requirements.rs:215-240; spec/test-cases/TC-454-the-pinned-sampler-reproduces-its-vectors-and-sampled-traces-replay.md:26,43-44 |
