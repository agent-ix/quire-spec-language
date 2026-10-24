---
id: SR-16365
title: 'Code review of PR #1405: hybrid claim requirement records and FR-450 solver-result settlement, code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@51046b01f536adffd33973f25ee4e7fece04ed1b; git diff origin/main...51046b01f (PR #1405, branch task/qsl-900-codex4, 24 files): qsl-replay/src/hybrid.rs, qsl-replay/src/hybrid_replay.rs, qsl-replay/src/proof_result{,.rs}/plugin*.rs, qsl-replay/src/outcome.rs, qsl-route/src/{lib,request}.rs, qsl-semantics/src/check/{mod.rs,lowering/hybrid.rs}, qsl-semantics/src/family/requirements.rs, tests; ticket QSL-900'
review_set: subset
date: '2026-10-24'
---

## Summary

Ticket: QSL-900. PR: quire-spec-language#1405, head 51046b01f, merge base equal to origin/main b8d713820. Method: code-review with the rust-review lane folded in. The diff adds the hybrid-dynamics requirement record per checked hybrid claim (FR-447, FR-250-AC-1), the descriptor `dynamics` and published solver bounds with candidate filtering (FR-075-AC-19/20), the FR-250 method/outcome/coverage map with `InsufficientHorizon`, `InductiveSolver` and `NoInitialState`, `SolverResult::settle` (FR-450 static then exact replay), and the FR-290 plugin readers for `bounded-solver{exhaustive}`, `inductive-solver`, `insufficient-horizon` and `solver-inconclusive{trace}`.

Checked and sound: `settle` returns `Refuted` only on `ExactReplay::Reproduced`; static and exact refusals return `Err`, meter exhaustion returns `Incomplete(LimitReached{EvaluationMeter{WorkUnits}})`, and every other path stays `Inconclusive(SolverInconclusive{trace, replay})`; exact replay reads the claim (and its own horizon) from `CheckedHybridClaim`. The `hybrid-dynamics` label is read from S3's recorded `VariableClass::Hybrid` and nothing reclassifies. The soundness triple table matches FR-250 exactly. All tags are `#[trace]` attributes; no `/// Trace:` doc line is added. Public API changes (`SolverOutcome` variants carrying `Option<Box<HybridTrace>>`, `InconclusiveCause::SolverInconclusive{trace, replay}`, new `ProofBasis`/`UnavailabilityCause`/`InconclusiveCause` variants, `terminal_value`/`basis` taking `&ClaimHorizon`) break nothing found downstream: quire-driver and quire-contract-codegen name none of the changed variants and match none of these enums exhaustively, and the workspace builds and tests.

Gates run (own target dir, one cargo job at a time): `cargo test -p qsl-route -p qsl-replay -p qsl-semantics --no-fail-fast` all green except `route_registry::cargo_deny_bans_the_three_registry_crates::cargo_deny_refuses_a_graph_that_depends_on_a_denied_crate` (known env noise: local cargo-deny 0.20.2 rejects `--config`); `cargo test --test it` 993 passed, 0 failed, 6 ignored.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| SolverResult::settle | qsl-replay/src/hybrid.rs:312-376 | examined |
| SolverResult::terminal_value / basis | qsl-replay/src/hybrid.rs:241-400 | examined |
| SolverMethod::is_sound_for, SolverCoverage::covers | qsl-replay/src/hybrid.rs:57-211 | examined |
| TerminalRecord::from_hybrid_plugin_proof | qsl-replay/src/proof_result/plugin.rs:292-323 | examined |
| read_plugin_basis / read_exhaustive / read_plugin_inconclusive_cause | qsl-replay/src/proof_result/plugin_read.rs | examined |
| read_plugin_inconclusive_cause_over, read_trace | qsl-replay/src/proof_result/plugin_read/trace.rs | examined |
| hybrid requirement record | qsl-semantics/src/check/mod.rs:3316-3373 | examined |
| Requirements::with_hybrid_dynamics | qsl-semantics/src/family/requirements.rs:201-212 | examined |
| RequestItem/RequirementItem::requirements, candidates_of | qsl-route/src/request.rs | examined |
| BackendDescriptor::with_dynamics, serves_dynamics, descriptor_from_manifest_with_dynamics | qsl-route/src/lib.rs | examined |
| FR-250 | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md | context_only |
| FR-450, FR-290, FR-447, FR-075 | spec/functional/ | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `settle`, `terminal_value` and `basis` take the claim's horizon as a caller-supplied `&ClaimHorizon` beside the `&CheckedHybridClaim` that already carries `clock` and `horizon`; nothing ties them, so a wrong horizon settles `Proved` on a bounded proof that does not cover the claim (FR-250 decides coverage "from the CheckedHybridClaim's clock and horizon"). The PR's own AC-4 test passes `Seconds(12)` for `SlowEnough` (10 s). | qsl-replay/src/hybrid.rs:246,312-319; qsl-replay/src/hybrid_replay/tests.rs:770-772 |
| FND-002 | medium | FR-250-AC-1's "no hybrid provider registered settles `unsupported`, `unsupported-requested-capability`" has no production path: `UnavailabilityCause::HybridProviderAbsent` is constructed only inside a test, which hand-builds the `TerminalValue` and asserts its serialization, so the AC-1 tag is not backed by the settlement it names. | qsl-replay/src/proof_result.rs:390; qsl-replay/src/hybrid.rs:522-534 |
| FND-003 | medium | FR-250-AC-6 is tagged on five unit tests, but its clause "a first-party `bounded-solver` entry with `method` `flowpipe-unrounded` is refused `invalid_result`/`invalid-value` at `/basis/method`" has no implementation anywhere (the only method reader is the plugin reader, which admits it, as FR-290 requires), so the computed matrix counts AC-6 covered. | qsl-replay/src/hybrid.rs:434-593; qsl-replay/src/proof_result/plugin_read.rs:309-321 |
| FND-004 | low | `SolverResult::terminal_value` is public and keeps a trace-carrying outcome as `SolverInconclusive{trace: Some, replay: None}` without admission or replay, an easy bypass of FR-250 "check a trace ... before keeping it" and FR-450 "replay every HybridTrace it keeps"; only `settle` is conforming. | qsl-replay/src/hybrid.rs:246,292-295 |
| FND-005 | low | `TerminalRecord::from_plugin_proof` still accepts a `BoundedSolver` basis with no coverage check; FR-250 requires checking the coverage of every `bounded-solver` proof, a plugin's included, and only the new `from_hybrid_plugin_proof` does so. | qsl-replay/src/proof_result/plugin.rs:257-286 |
| FND-006 | low | `hybrid_manifest_admission_checks_dynamics_after_fairness_and_evidence` is tagged FR-250-AC-3, but it asserts descriptor `dynamics`/`bounds` admission (FR-075-AC-20); AC-3 is about request `max_solver_time` rounding and request-budget refusals, deferred to QSL-1035, so the matrix shows AC-3 tagged with no test of it. | qsl-route/tests/it/route_registry.rs:115-117 |
| FND-007 | low | The request-writer test uses the path form `#[ix_trace_rs::trace("TC-1187", "FR-447-AC-17", "FR-250-AC-1")]` in a module that imports `trace`; the computed matrix does not index it, so FR-447-AC-17 reads untagged. The test also checks only the `requirements()` accessor and item inequality, not the wire member or request identity AC-17 states. | qsl-route/src/request.rs:750-762 |
| FND-008 | low | `each_hybrid_claim_has_a_hybrid_dynamics_requirement_record` ends with `records(&use_case()).is_empty()`, documented as "a temporal claim over no hybrid model has none", but `use_case()` declares no claim at all, so the assertion is vacuous for the stated case. | qsl-semantics/src/check/hybrid/tests.rs:1078,1122 |

## Verdict

CHANGES REQUESTED. Settlement soundness is correct where it matters most: `Refuted` is reachable only through exact replay reproducing the trace, and every other path is `Inconclusive`, `Incomplete` or a refusal. The hybrid label is derived from S3's classification without reclassifying. Trace attributes are used throughout and no deprecated doc lines are added. No downstream breakage was found. Before merge, FND-001 should be fixed by deriving `ClaimHorizon` from `CheckedHybridClaim`, and FND-002 should be either wired or explicitly deferred with the AC-1 tag narrowed. FND-003 and FND-006 overstate matrix coverage for ACs the PR body says are deferred.
