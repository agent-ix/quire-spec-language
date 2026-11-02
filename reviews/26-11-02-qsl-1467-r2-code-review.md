---
id: SR-2026110215391
title: "Code review round 2 — QSL-1467: settle a construction budget stop as FR-126's Stopped"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2118 fix round 0b1d9029b..08b58f4f9; qsl-analyze/src/model_check/check.rs; qsl-replay/src/model_check/settle.rs; qsl-eval/src/simulation/model.rs; qsl-eval/src/simulation/protocol/access.rs; qsl-analyze/tests/it/weak_memory.rs"
review_set: subset
---

## Summary

Ticket: QSL-1467. PR: quire-spec-language#2118, branch `qsl-1467`, head 08b58f4f9. Round 2, scoped to the fix-round commit 08b58f4f9 (`git diff 0b1d9029b..HEAD`; 0b1d9029b is the rebased round-1 head 354364ecf, identical by range-diff). Method: code-review with the Rust lane.

The fix round adds `construction_refusal`, shared by `check_protocol` and `check_protocol_liveness`, and `expansion_stopped(stop, meter, max_candidates)`, shared with the explorer's `Run::expansion_stop`. A `ProtocolRefusal::InitialStop` from `ProtocolSystem::new` now settles `Stopped{ResourceExhausted, limit}`. In `settle_protocol_check`, `load_buffering_warnings` now returns `Result<_, Option<ExpansionStop>>`. A non-internal stop in the FR-226 pre-check keeps an already-`Stopped` record with no warnings.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| construction_refusal | qsl-analyze/src/model_check/check.rs:909-938 | examined |
| expansion_stopped | qsl-analyze/src/model_check/check.rs:1028-1050 | examined |
| Run::expansion_stop | qsl-analyze/src/model_check/check.rs:2851-2858 | examined |
| check_protocol meter/max_candidates source | qsl-analyze/src/model_check/check.rs:694-695, 813 | examined |
| settle_protocol_check warnings match | qsl-replay/src/model_check/settle.rs:290-311 | examined |
| load_buffering_warnings | qsl-replay/src/model_check/settle.rs:348-379 | examined |
| ProtocolSystem::new Enabled + Potential classify | qsl-eval/src/simulation/protocol.rs:1030-1047 | context_only |
| load_buffering_shapes / classify_accesses (Enabled only) | qsl-eval/src/simulation/protocol.rs:830-882 | context_only |
| walk stop(), COLLECTION_WRITE, FamilyRefused stops (limit None) | qsl-eval/src/simulation/model/walk.rs:21; frames.rs:1408,1654; constraint.rs:86 | context_only |
| FR-126 expansion-stop SHALL | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:306-313 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `construction_refusal` now settles any non-internal `InitialStop`, not only a budget stop, as `Stopped{ResourceExhausted, None}`. That includes a stop with no limit and a non-resource code, such as `unsupported_construct`/`collection-write-not-computed` from `apply_operation` during Enabled classification, or a `FamilyRefused` code from an initial guard. The new FR-220 bullet covers only a stop "on either budget". Before this fix such a stop reached analyze as `Failed(BROKEN)`, and the explorer already maps a mid-run stop with no limit the same way, so this is no regression and no verdict reads as verified. FR-126 does not say what a non-budget expansion stop settles. | qsl-analyze/src/model_check/check.rs:1046-1047; qsl-analyze/src/model_check/check.rs:929-935 |

## Verdict

Mergeable. FND-001 is a non-blocking spec question that predates this PR.

- `expansion_stopped` gives the right limit values. A meter stop yields `value = meter_bound(meter, counter)`, and a `MaxCandidates` stop yields the configured `max_candidates`, as FR-126 lines 306-313 require. Both callers pass the subject's own budgets. `check_protocol` uses `simulation.meter` and `simulation.expansion.max_candidates`. The liveness path uses `subject.model.simulation.meter` and `.expansion.max_candidates`.
- An internal-failure stop still refuses. `expansion_stopped` returns `None`, and `construction_refusal` returns `Err(ModelCheckRefusal::Protocol(InitialStop))`, which analyze still reports as broken. The explorer still reports `fault("model-expansion-invariant")`. `WeakAccessShape` and `RecoveryView` keep the same `Memory`/`RecoveryView` outcomes with `RunStats::default()`.
- No verdict can read a budget stop as verified or as a counterexample. The outcome is `Stopped` with zero stats, which FR-127 settles as `incomplete`/`limit-reached`.
- settle.rs drops the warnings only when two things hold together: the pre-check returned a non-internal stop, and the outcome is already `Stopped`. Every other `Err` still fails the record. A complete outcome cannot hide a warning. `ProtocolSystem::new` runs the same Enabled `classify` over the same `model.initial().first()` with a fresh `model.budget()` before the run. If the pre-check stops, construction stopped first, and the outcome is construction's `Stopped`. FR-226 decides a shape only over shared locations. A stopped classification decides none, so there is no shape to warn of.
- Rust lane: no `#[allow]`, no new `unwrap`/`panic!` outside tests, no compatibility layer, no pins. `Result<_, Option<ExpansionStop>>` is documented under `# Errors`. Exposing `EXHAUSTED` as `pub(crate)` resolves round-1 FND-002.
