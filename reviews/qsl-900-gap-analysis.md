---
id: SR-16366
title: 'Gap analysis of PR #1405: hybrid claim requirement records and FR-450 solver-result settlement'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@51046b01f536adffd33973f25ee4e7fece04ed1b; git diff origin/main...51046b01f (PR #1405, branch task/qsl-900-codex4, 24 files); criteria FR-250-AC-1..7, FR-447-AC-17, FR-450-AC-2, FR-290-AC-18, FR-075-AC-19/20; ticket QSL-900'
review_set: subset
date: '2026-10-24'
---

## Summary

Ticket: QSL-900. Planless gap analysis of PR #1405 at 51046b01f. The test matrix came from `quire matrix --scope <worktree> --format json` (quire 0.36.2, engine 0.50.2; quoin 0.28.4) and was read for every criterion the diff tags or names. Reverse gaps and stubs were checked over the 24 changed files. A scoped intent↔test↔code check was done by the reviewer on the tagged FR-250 criteria. A full fan-out semantic review was not run.

Matrix for the touched criteria: FR-250-AC-1, AC-2, AC-3, AC-4, AC-6 and AC-7 are tagged. FR-250-AC-5 is untagged (the PR body defers it to QSL-1035). FR-447-AC-17 is untagged. FR-450-AC-2, FR-290-AC-18, FR-075-AC-19 and FR-075-AC-20 are tagged. No reverse gap was found: every new production unit is owned by FR-250, FR-450, FR-290, FR-447 or FR-075. `descriptor_from_manifest_with_dynamics` (FR-075) has no production caller yet, which the PR body records as groundwork.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-447-AC-17 is untagged in the computed matrix: its only binder uses the path form `#[ix_trace_rs::trace(..)]`, which the index does not pick up in this file. | qsl-route/src/request.rs:750 |
| FND-002 | medium | FR-250-AC-5 (`NoInitialState` over `Braking` with an excluded `initial`, and through the plugin reader) is untagged; the PR body defers it to QSL-1035. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md |
| FND-003 | medium | Coverage inflation: FR-250-AC-3 is tagged only by a descriptor-admission test that asserts none of AC-3's request-budget behaviour. | qsl-route/tests/it/route_registry.rs:115-117 |
| FND-004 | medium | Hollow evidence: FR-250-AC-1's `unsupported-requested-capability` clause is backed by a test that hand-builds `TerminalValue::Unsupported(HybridProviderAbsent)`, and no production path produces it. FR-250-AC-6's first-party `/basis/method` refusal has no implementation behind its tag. | qsl-replay/src/hybrid.rs:522-534; qsl-replay/src/hybrid.rs:434-593 |

## Verdict

FAIL under the gap-analysis rule, because two touched criteria are untagged (FR-447-AC-17, FR-250-AC-5). FR-250-AC-5 is a tracked deferral (QSL-1035). FR-447-AC-17 is a one-line tag-form fix. FND-003 and FND-004 overstate what the matrix reports as covered. With FR-447-AC-17 retagged and the AC-1/AC-3/AC-6 tags narrowed to what the tests assert, the result would be CONDITIONAL, pending the deferrals.

## Coverage

- Criteria examined: FR-250-AC-1 to AC-7, FR-447-AC-17, FR-450-AC-2, FR-290-AC-18, FR-075-AC-19, FR-075-AC-20.
- Tagged: 10 of 12. Untagged: FR-250-AC-5, FR-447-AC-17.
- Reverse gaps: none. Stubs: none in production code. Hollow evidence: FND-004.
- Semantic review: reviewer-scoped to the touched FR-250 criteria; the full fan-out semantic review was not run.
- Plan completion: not assessed

## New findings (disposition pass 1)

Disposition pass 1 at 70e7e960f. The matrix was recomputed with `quire matrix --scope <worktree> --format json`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | FR-290-AC-5 is tagged, and its text says each `bounded-solver` value `read_plugin_basis` reads, "passed to `TerminalRecord::from_plugin_proof`, settles `proved` with `trusted`". At 70e7e960f `from_plugin_proof` refuses every `BoundedSolver` basis, and the tagged test never exercises that clause, so the tag covers a clause the code now contradicts (code SR-16365 FND-009). | qsl-replay/src/proof_result/plugin_tests.rs; qsl-replay/src/proof_result/plugin.rs:257-273 |

## Dispositions

Round 1, at 70e7e960f9d9c27607e6630c646f87c6399b7a7e.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-447-AC-17 is now tagged in the computed matrix by `request_writer_keeps_hybrid_dynamics_in_the_item_identity`, which uses plain `#[trace("TC-1187", "FR-447-AC-17")]`. |
| FND-002 | deferred | QSL-1035 owns FR-250-AC-5. It stays untagged at 70e7e960f, and no tag claims it. |
| FND-003 | deferred | QSL-1035 owns FR-250-AC-3. The inflated tag is gone, the matrix now reports AC-3 honestly as untagged, and no tag claims it. |
| FND-004 | fixed | AC-1's unsupported clause now runs a real checked graph through the registry and `settle_hybrid_candidates`, in `a_checked_hybrid_claim_routes_to_the_hybrid_provider_only`. AC-6's first-party `/basis/method` refusal is implemented in `read_first_party_solver_basis` and tested in `a_first_party_entry_with_an_unsound_method_is_refused`. |
