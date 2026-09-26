---
id: SR-643
title: "QSL-272 integrity review of FR-101, its TCs and the ADR-011/TC-390 amendment"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@84a691bf8beb9df40aa945c48e1e05d7f5fb070b; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md; spec/test-cases/TC-453-exploration-orders-successors-canonically-and-keys-states-by-jcs-bytes.md; spec/test-cases/TC-454-the-pinned-sampler-reproduces-its-vectors-and-sampled-traces-replay.md; spec/test-cases/TC-455-stopped-explorations-stay-incomplete-and-unbounded-requests-require-a-bound.md; spec/test-cases/TC-390-family-outcome-and-refusal-layering.md; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md (unchanged); spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md (unchanged); spec/functional/FR-097-classify-claim-extent-and-write-bounded-requests.md (unchanged); qsl-eval/Cargo.toml; tests/it/family_outcome_layering.rs; qsl-eval/src/simulation/trace.rs; quire-canonical@0143a2b src/lib.rs; quire-specification@0d53cf2d FR-181, FR-201, TC-210, simulation-sampler.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-390
    type: reviews
---

## Summary

Ticket: QSL-272. This review checks consistency with the QSpec sources and
with QSL's own ADRs.

What holds:

- **FR-181 and simulation-sampler.md.** The canonical successor order, the
  state-key form, the preimage `{draw, seed, step, trace}`, the
  rejection bound `n * floor(2^256 / n)`, `n = 1` / `n = 0`, and `d`
  restarting per step all match `1-draft.1`.
- **FR-201.** It gives `quire.simulation.state-key/v1` as SHA-256 over the
  JCS bytes, with the label outside the preimage. FR-101's "no domain label
  in the preimage" matches, and it rules out `quire_canonical::sha256_with_domain`.
- **ADR-013 §2.** It requires one RFC 8785 encoder. `quire_canonical::sha256`
  exists at the pinned rev (lib.rs:174), so "SHA-256 comes through
  `quire-canonical`; `sha2` stays a dev dependency" can be implemented.
- **FR-097.** `Outcome::category()` stays its map.
- **ADR-014 §1 and §4.** `RequiresBound` is not an `Outcome`, and `Limits`
  never answers it.
- **TC-390 amendment.** It is correct. The old step-4 text listed `qsl-attrs`,
  `serde` and `serde_json`, which were stale: `qsl-eval/Cargo.toml` names
  none of them in `[dependencies]`, and `family_outcome_layering.rs:420-431`
  asserts the five-crate set. The new set is that set plus
  `quire-canonical` and `serde`. The Status note says clearly that the
  assertion changes in the implementing commit. `serde` is needed because
  `quire_canonical::sha256` takes `T: Serialize`. ADR-011 X-8 agrees with
  TC-390.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-101 does not settle whether a trace records full state-key bytes or state-key digests. Behavior says "The digest identifies a state in traces and frontiers", following FR-181 ("the digest identifies states only in traces and frontier records"). AC-5 and TC-454 step 7 instead speak of a trace's "initial key" and "recorded key", and today `Trace.initial` and `Step.key` are full `StateKey` bytes (trace.rs:12-15,42-44). The Status paragraph lists the frontier's move to digests, but not the trace's. Two implementations can diverge: one keeps full bytes in `Trace`, and one records digests and replays by recomputed digest. They produce non-interchangeable traces, and the second's `KeyMismatch.actual`/`expected` change type. TC-453 step 5 also says "3's key then 4's" for a frontier that AC-6 and AC-7 make digests. Fix: state what `Trace` records, and word AC-5, TC-454 step 7 and TC-453 step 5 to match. | spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:90-91,135,160-165; spec/test-cases/TC-454-the-pinned-sampler-reproduces-its-vectors-and-sampled-traces-replay.md:27-29,45-47; spec/test-cases/TC-453-exploration-orders-successors-canonically-and-keys-states-by-jcs-bytes.md:44-45; qsl-eval/src/simulation/trace.rs:12-15,42-44 |
| FND-002 | low | ADR-014 still names the interfaces FR-101 replaces. TR-7 (:189) and the §7 cancellation row (:352) say `explore::Outcome::Cancelled{frontier}`, over a key-bytes `Frontier`. TR-1 (:183) identifies a sampled trace by `SampleProvenance{seed, sampler_version}` plus its index. FR-101 makes these `Cancelled{stats, frontier, cause}` with a digest frontier, and `SampleProvenance{seed, trace, sampler, stopped}`. ADR-011 X-8 is amended in this PR, but ADR-014 is not, so the design authority FR-101 cites (`ADR-014 … TR-1, TR-6, TR-7`) contradicts it. Fix: amend TR-1, TR-7 and the §7 row, or note in them that FR-101 supersedes. | spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:183,189,352; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:62-64,104-106,147-148 |
| FND-003 | low | TC-390 step 4, in the sentence this PR edits, still says `qsl-eval`'s "`[dev-dependencies]` may also name `qsl-forms`, and no other workspace crate". `qsl-eval/Cargo.toml` names `qsl-cst` as a dev dependency, and the backing test's doc admits it (family_outcome_layering.rs:264-265). This was already true before the PR, but the edit keeps the stale clause. Fix: "may also name `qsl-forms` and `qsl-cst`". | spec/test-cases/TC-390-family-outcome-and-refusal-layering.md:79-80; qsl-eval/Cargo.toml:37-39; tests/it/family_outcome_layering.rs:264-265 |
