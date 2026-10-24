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

## New findings (disposition pass 1)

Disposition pass 1, reviewing fix commit 70e7e960f.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | The FND-005 fix makes `TerminalRecord::from_plugin_proof` refuse every `BoundedSolver` basis (`Malformed` at `/basis/type`). FR-290-AC-5 still says each basis `read_plugin_basis` reads, the `bounded-solver` values included, "passed to `TerminalRecord::from_plugin_proof`, settles `proved` with `trusted`", and FR-289-AC-9 says a process provider's `proved` settles with "the reported basis". Code and spec now disagree. Choosing refusal over trusting an unchecked basis is the safer side, but FR-290-AC-5 needs amending to send `bounded-solver` through `from_hybrid_plugin_proof`. The test `every_plugin_basis_is_trusted` (FR-289-AC-9) now asserts the refusal. | qsl-replay/src/proof_result/plugin.rs:257-273; qsl-replay/tests/terminal_record_facade.rs:582-601; spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:542 |
| FND-010 | low | `settle_hybrid_candidates(candidate_count: usize)` sees only a count, so it returns `Unsupported(HybridProviderAbsent)` for any item with an empty set, a non-hybrid item included. The caller must already know the item carries `hybrid-dynamics`. Taking the `RequirementItem` (or its `requirements()`) would make the map impossible to misapply. | qsl-replay/src/hybrid.rs:410-419 |

## Dispositions

Round 1, at 70e7e960f9d9c27607e6630c646f87c6399b7a7e. Round 2 rows (FND-009, FND-010), at bc2e8ee1034825c0c06862ba0cf4916fb982757e, follow the round-1 rows.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `ClaimHorizon::of(&CheckedHybridClaim)` derives the horizon from the claim's own clock and horizon. `settle(graph, claim, meter)` no longer takes a horizon, and `settlement_basis(&CheckedHybridClaim)` and `from_hybrid_plugin_proof(.., &CheckedHybridClaim)` derive it the same way. A mismatched or negative horizon reads as unbounded, which no bounded proof covers. Tested by `coverage_is_decided_from_the_checked_claims_own_horizon` over real `SlowEnough`/`Positive` claims. |
| FND-002 | fixed | `settle_hybrid_candidates` maps an empty hybrid candidate set to `Unsupported(HybridProviderAbsent)`. The root test `hybrid_request_items` runs a real checked graph through the registry and this map. Production negotiation lives in quire-contract-codegen, so the CG call site is follow-up work. Accepted as the wiring hook (see FND-010). |
| FND-003 | fixed | `read_first_party_solver_basis` refuses `flowpipe-unrounded` and `unclassified` `bounded-solver` bases as `Malformed`/`InvalidValue` at `/basis/method`, tested by `a_first_party_entry_with_an_unsound_method_is_refused` under FR-250-AC-6. |
| FND-004 | fixed | `terminal_value` and `basis` are now private; `settle` and `settlement_basis` are the public entry points. |
| FND-005 | fixed | `from_plugin_proof` refuses a `BoundedSolver` basis, and coverage-checked admission goes through `from_hybrid_plugin_proof`. This opens the spec drift recorded as FND-009. |
| FND-006 | fixed | The route_registry tests are now tagged `FR-075-AC-19` and `FR-075-AC-20` only. |
| FND-007 | fixed | The test is retagged with plain `#[trace("TC-1187", "FR-447-AC-17")]`, and the matrix indexes it. It now also checks that a non-temporal item gets no label. It still asserts the `requirements()` accessor rather than serialized bytes, because this repo has no wire writer for request items. |
| FND-008 | fixed | The root test `hybrid_request_items` now has `Reaches`, a real temporal claim over a non-hybrid model, which carries no label and routes to EN-6 only. The semantics test comment now says the bare declaration has no claim. |
| FND-009 | deferred | QSL-1036 (spec ticket) amends FR-290-AC-5 and FR-289-AC-9 so a `bounded-solver` plugin basis settles through `from_hybrid_plugin_proof` with its coverage check. At bc2e8ee10 the refusal in `from_plugin_proof` stays, which is the sound behaviour. `every_plugin_basis_is_trusted` drops its FR-289-AC-9 tag and its doc comment names QSL-1036. FR-290-AC-5 stays only on `tc_774_basis_reader_accepts_the_plugin_spellings`, which checks the basis reader the refusal does not touch. |
| FND-010 | fixed | At bc2e8ee10 `settle_hybrid_candidates(requirements: &[&str], candidate_count)` returns `Unsupported(HybridProviderAbsent)` only when `requirements` holds `hybrid-dynamics` and the count is 0. The root test maps both real items (`SlowEnough` gives `Some`, `Reaches` gives `None`) and checks that `(&[], 0)` gives `None`. |
