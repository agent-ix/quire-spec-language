---
id: SR-202611021416
title: "Code review — QSL-1467: a budget stop while classifying is a resource stop"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2118 head 354364ecf; qsl-eval/src/simulation/protocol/access.rs; qsl-eval/src/simulation/protocol.rs; qsl-analyze/tests/it/weak_memory.rs"
review_set: subset
---

## Summary

Ticket: QSL-1467. PR: quire-spec-language#2118, branch `qsl-1467`, head 354364ecf, `git diff origin/main...HEAD` against origin/main 4a7cef38e. Method: code-review with the Rust lane (rust-review and rust-style).

The PR adds `classification_stop` in `access::classify`. A non-internal `ExpansionStop` from `attempt_identities` (Potential mode), `apply_operation` (Enabled mode) or `cell_of` becomes `ProtocolRefusal::InitialStop(stop)`. An internal-failure stop stays `ProtocolRefusal::fault("attempt-identities-list")`. The `InitialStop` doc and message now say "building over an initial state" instead of "settling an initial state".

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| classification_stop | qsl-eval/src/simulation/protocol/access.rs:703-713 | examined |
| classify map_err sites (Potential, Enabled, cell_of) | qsl-eval/src/simulation/protocol/access.rs:815-925 | examined |
| ProtocolRefusal::InitialStop doc, code(), cause() | qsl-eval/src/simulation/protocol.rs:233-280 | examined |
| ProtocolSystem::new classify calls (Enabled first, then Potential) | qsl-eval/src/simulation/protocol.rs:1028-1047 | examined |
| initial_states settle InitialStop | qsl-eval/src/simulation/protocol.rs:1140-1150 | context_only |
| check_protocol / check_protocol_liveness refusal routing | qsl-analyze/src/model_check/check.rs:778-827, 868-929 | examined |
| Explorer::expansion_stop (V-7 Stopped naming the limit) | qsl-analyze/src/model_check/check.rs:2820-2834 | context_only |
| model_refusal (FR-281 engine refusals) | qsl-analyze/src/analyze/items.rs:807-861 | examined |
| ModelTraceRefusal::Protocol code()/cause() | qsl-replay/src/model_check/replay.rs:483, 543, 898 | examined |
| ClosureRefusal::Protocol | qsl-replay/src/certificate/closure.rs:225, 418-466 | examined |
| AbstractProtocolSystem::new (abstract side, sc only) | qsl-replay/src/refinement/subject.rs:135 | examined |
| unit test a_budget_stop_is_a_stop_and_a_broken_one_a_fault | qsl-eval/src/simulation/protocol/access.rs:1404-1449 | examined |
| integration test a_candidate_bound_reached_while_classifying_is_a_resource_stop | qsl-analyze/tests/it/weak_memory.rs:2631-2657 | examined |
| FR-120 "Post-states" max_candidates stop | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:631-643 | context_only |
| FR-126 expansion-stop SHALL | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:306-313 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The construction-time budget stop still reaches the analyze report as a broken invariant. `check_protocol` and `check_protocol_liveness` send every non-shape `ProtocolRefusal` to `Err(ModelCheckRefusal::Protocol(..))`, and `model_refusal` maps every `ModelCheckRefusal::Protocol(_)` to `Refusal::Failed(BROKEN)`. A model check whose EN-1 classification reaches `max_candidates` therefore still reads `runtime_invariant`/`established-invariant-broken`, which is the ticket's defect. FR-126 says an expansion that reaches `max_candidates` or the meter returns `Stopped{ResourceExhausted, limit: Some({MaxCandidates, value})}`. The explorer already builds that outcome in `expansion_stop`, but the construction path never reaches it. The settle-path `InitialStop` has the same gap. | qsl-analyze/src/model_check/check.rs:827; qsl-analyze/src/model_check/check.rs:929; qsl-analyze/src/analyze/items.rs:858 |
| FND-002 | low | The unit test builds `CatalogCode::new("resource_exhausted", "insufficient-next-charge")` inline. `model.rs` already declares the same code as `EXHAUSTED`, so the test restates a catalog constant. Making it `pub(crate)` like `BROKEN` would let the test reuse it. | qsl-eval/src/simulation/protocol/access.rs:1419-1422 |

## Verdict

Not mergeable as is, because of FND-001. The eval-level change is correct as far as it goes:

- `classification_stop` splits on `is_internal_failure`. This matches every other stop consumer in the workspace, for example `expansion_stop` at check.rs:2832 and replay.rs:1247.
- `InitialStop` carries the stop, so `code()`/`cause()` give `resource_exhausted`/`insufficient-next-charge` and the limit survives.
- All three `map_err` sites changed: Potential, Enabled and `cell_of`. `cell_of` can only return a walk stop.
- Replay passes the code and cause through (`ModelTraceRefusal::Protocol` code()/cause()), so a replay now reports `resource_exhausted`.
- A certificate closure refuses with `ClosureRefusal::Protocol`. That is a refusal, never a confirmed certificate.
- The abstract refinement side resolves `sc` with no weak `parallel`, so `classify` lists nothing there and the new path cannot be reached.
- No consumer reads the new refusal as `Holds`/verified or as a counterexample. The misclassification is only the "broken invariant" label at the analyze report (FND-001).

Rust lane: there are no `#[allow]`/`#[expect]`, no caps, no compatibility layer, no new `unwrap`/`panic!` outside tests, and no pins or digests. `ExpansionStop` is `Copy`, so passing it by value is right. The `# Errors` doc on `classify` is accurate. The `InitialStop` doc still says that a guard that breaks while settling is an `InitialStop`, which matches `initial_states`.

Trace tags: `#[trace("TC-665", "FR-120", "FR-220")]`. TC-665 verifies FR-220 only, and no AC of FR-220 or FR-120 states the behaviour these tests assert. Gap-analysis (SR-202611021417) records this.
