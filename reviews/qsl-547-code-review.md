---
id: SR-13595
title: 'Code review of PR #1113: refinement liveness under the abstract fairness (code-review with rust-review lane)'
type: SpecReview
scope: 'agent-ix/quire-spec-language@38b65f748; git diff origin/main...38b65f748 (PR #1113, branch qsl-547-refine-liveness): qsl-analyze/src/model_check/{refine,fair_walk,liveness}.rs, qsl-replay/src/model_check/{refinement_replay,refinement_settle,replay,settle,outcome}.rs, qsl-replay/src/refinement/{liveness,outcome,failure,step}.rs, qsl-replay/src/{proof_result,lib}.rs, qsl-replay/src/certificate/simulation.rs and the tests; ticket QSL-547'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-143
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-144
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-145
  type: reviews
---

## Summary

Ticket: QSL-547. PR: quire-spec-language#1113, head 38b65f748. Method: code-review with the rust-review lane. The review covers only the delivered scope. FR-143-AC-5, FR-144-AC-1/AC-5 and the FR-145-AC-4 identity and package refusals are remaining work and are not reviewed.

Evidence on head 38b65f748 (own clone, own CARGO_TARGET_DIR): `cargo fmt --all -- --check` exit 0. `cargo clippy --workspace --all-targets -- -D warnings` exit 0. `cargo test -p qsl-eval -p qsl-replay -p qsl-analyze -p qsl-semantics` exit 0. Root `cargo test --test it` exit 0 (975 passed, 6 ignored). `make string-edge`, `make arch-lint-api-surface-qsl`, `make arch-lint-canonical-encoder` and `make arch-lint-qualified-core` each exit 0.

Brute-force oracle (reviewer probe, not part of the PR). Random `Pair` refinements: fields `a`, `b` in Int[0,1]; `flipA`, `flipB` and `setA(x)` on both models, plus a concrete-only `peek` mapped to `stutter`; random preconditions, random initial state and random `F_C` (weak/strong, each/whole). Each case compares the engine with exhaustive fair-cycle enumeration over vertex subsets. With weak `F_A` (2100 cases, seeds 11 to 13) there were 0 mismatches: the safety verdict, Holds against Violated, the first violated `F_A` constraint and the lasso were all right. Every engine lasso was a valid, F_C-fair cycle that violates its class, its kind (Divergence or AbstractUnfair) was right, it replayed `Reproduced`, and the kind-swapped payload was `Inconclusive`. Of 12 988 crafted lassos, 6417 F_C-fair ones replayed `Reproduced` and 6571 unfair ones were refused `UnfairLasso`. With strong `F_A` there were 17 mismatches in 300 cases (FND-001).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| check_refinement liveness gating | qsl-analyze/src/model_check/refine.rs | examined |
| Run::liveness / second_phase / abstract_enabled | qsl-analyze/src/model_check/refine.rs | examined |
| Run::fair_graph / Run::lasso | qsl-analyze/src/model_check/refine.rs | examined |
| fairness_constraints (extracted) | qsl-analyze/src/model_check/fair_walk.rs | examined |
| AbstractClass / abstract_classes | qsl-replay/src/refinement/liveness.rs | examined |
| replay_refinement_trace lasso arm / replay_lasso / same_product_state | qsl-replay/src/model_check/refinement_replay.rs | examined |
| settle_refinement / settle_liveness / record_of | qsl-replay/src/model_check/refinement_settle.rs | examined |
| LivenessHalf / RefinementOutcome | qsl-replay/src/refinement/outcome.rs | examined |
| RefinementFailure::Divergence / AbstractUnfair / position | qsl-replay/src/refinement/failure.rs | examined |
| UnavailabilityCause::HiddenFieldLiveness | qsl-replay/src/proof_result.rs | examined |
| ModelTraceRefusal::LoopDrifts (RefinementLasso deleted) | qsl-replay/src/model_check/replay.rs | examined |
| FailedStep wire arm | qsl-replay/src/certificate/simulation.rs | examined |
| FR-143 | spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md | context_only |
| FR-137-AC-4 | spec/functional/FR-137-check-a-refinement-s-fairness-rows.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A strong `F_A` constraint is decided as weak. `AbstractClass` keeps no strength, and the second phase decomposes only the states where the class is enabled. So a fair concrete loop that only sometimes reaches a state enabling the class, and never takes it, is never seen, and the liveness half settles `proved` (Uncertified) on a refinement it should refute. S3 admits `ensure fair strong` (FR-137-AC-4), and nothing here refuses it. Probe: Pair with flipB pre `self.a = 0` on both models, `assume fair weak each Impl::Pair::flipA`, `ensure fair strong whole Spec::Pair::flipB`. The loop flipA, flipA visits a=0 forever and never takes flipB, yet the result is Checked(HoldsReduced{[]}). The random oracle with strong rows gives 17/300 mismatches: Holds where the refinement is violated, or a later weak constraint named instead of the first strong one. Replay's `violated` check has the same weak-only reading. This needs a spec ruling: implement strong (enabled at some loop state) or settle it `unsupported`. | qsl-analyze/src/model_check/refine.rs:709-716; qsl-replay/src/refinement/liveness.rs:17-55; qsl-replay/src/model_check/refinement_replay.rs:373-377 |
| FND-002 | low | The `RefinementOutcome.liveness` doc says the half is `None` with a non-empty `F_A` "when the safety half is other than `Holds`". But `check_refinement` returns `Some(Unsupported(HiddenFieldLiveness))` for a hidden-field refinement whatever the safety half is, as FR-143's second bullet requires. The doc drops the no-hidden-field condition. | qsl-replay/src/refinement/outcome.rs:113-114; qsl-analyze/src/model_check/refine.rs:213-227 |
| FND-003 | low | An internal fault in the second phase returns `Err(ModelCheckRefusal::Fault)` from `check_refinement`. That discards the already-proved safety half and its certificate, so the item cannot settle `failed` with its safety value beside it. A safety-phase fault becomes `SafetyOutcome::Fault`, which settles `failed` (FR-144 "a fault `failed`"). This matches FR-126's own liveness refusal, so it is a judgement call. | qsl-analyze/src/model_check/refine.rs:617, 224 |

## Verdict

Not mergeable: FND-001 lets a refinement settle `proved` when a strong `F_A` constraint is violated. If the owner rules strong `F_A` out of scope for this ticket, the half must settle `unsupported`, not `proved`.

The rest is correct and verified by the oracle and by mutation:
- F_A classes: `whole` is one operation, `each` is one identity in canonical transition-key order.
- The subgraph keeps the states that enable the class and drops the edges that take it.
- Vertex enabledness for F_C is read from all the node's edges.
- The F_C filter goes through the qsl-eval kernel, strong F_C refinement included.
- The terminal stutter self-loop is read as RS-10 requires.
- The canonical lasso is built from the tree-path stem and CX-5's greedy walk, and is classed Divergence only when every loop edge is `Taken::Stutter`.
- The replay lasso arm refuses a drifting product loop (`LoopDrifts`) and an F_C-unfair loop (`UnfairLasso`).
- `settle_refinement`: a holding half settles `proved` Uncertified, a violated half settles `refuted` only through a `Reproduced` lasso replay, and `Unsupported` keeps the proved safety half in `RefinementRecord.safety`.

Judgement calls accepted:
- `HoldsReduced` with no reductions for a holding half. The spec representation drift is recorded in SR-13596.
- `HiddenFieldLiveness` shares the wire cause and code of `UnsupportedReduction`, which matches FR-143's `unsupported-requested-capability`.
- Hidden-field gating is unconditional, as FR-143 bullet 2 says.
- `Undecided(UndecidedSuccessor)` for an undecided abstract contract.
- `position() -> Option<u64>`: it has no callers.

`ModelTraceRefusal::RefinementLasso` is deleted, with no remaining code reference. No shim, compatibility layer or dead code was found. The extraction of `fairness_constraints` is behaviour-preserving, and the FR-126 suite is green. Mutation results are in SR-13596.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | FR-144 on main (#1117) says a `check_refinement` refusal (a second-phase internal fault, FR-143) "settles the item `failed` with that fault and no half value". Head 5884e0302 has no code that does this: `settle_refinement` takes only a `RefinementOutcome`, and `RefinementRecord.safety` is a required `TerminalValue`, so a record with no half value cannot be built. There is also no FR-143-AC-7 fault-injection test, although TC-548 step 7 calls for one. | qsl-replay/src/model_check/refinement_settle.rs:49-56, 62; qsl-analyze/src/model_check/refine.rs:632 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | A strong `F_A` constraint now gives `Unsupported(StrongAbstractFairness)` whatever the safety half is, and a hidden field is still reported first. Replay never reproduces a lasso that records a strong constraint. Oracle at 5884e0302: 266/266 strong cases are Unsupported, safety-failing ones included, with 0 mismatches. Weak `F_A`: 2100 cases, 0 mismatches. M29 and M31 are killed. |
| FND-002 | fixed | The `RefinementOutcome.liveness` doc now names the hidden-field and strong cases, which hold whatever the safety half is. |
| FND-003 | accepted-no-change | FR-143 on main (#1117) adopts the refusal: a second-phase fault returns a refusal and the safety half is not returned. The doc on `check_refinement` says so. The missing settlement path is the new FND-004. |
| FND-004 | fixed | `settle_refinement_refusal(&InternalFault, &ModelCheckLimits)` settles the refusal `failed`, with `safety: None`, `liveness: None` and `fault: Some(..)`. `RefinementRecord.safety` is now an `Option`. This matches FR-144 on main: "settles the item `failed` with that fault and no half value". M33 (refusal settles refuted) is killed. |

