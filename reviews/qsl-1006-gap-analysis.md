---
id: SR-12359
title: 'Gap analysis of PR #1285: FR-404 same-run CallReplayContext'
type: SpecReview
scope: 'agent-ix/quire-spec-language@eab4e9af3a6e2df760e5bba53d72b0bf96be2425; git diff origin/main...HEAD (PR #1285, branch qsl-1006-impl): qsl-replay/src/call_replay_context.rs, qsl-replay/src/proof_result/plugin_replay.rs, qsl-replay/tests/call_replay_context.rs, qsl-replay/tests/call_replay_context_root_only.rs; FR-404-AC-1..AC-9, TC-1173; ticket QSL-1006'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-404
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-1173
  type: reviews
---

## Summary

Ticket: QSL-1006. PR: quire-spec-language#1285, head eab4e9af3. Method: quoin gap-analysis, planless (Plan completion: not assessed). `quoin matrix --json` (quoin 0.28.3, quire 0.36.2) lists FR-404-AC-1 to AC-9 all `tagged` with `no run evidence`; binder counts are AC-2 2, AC-5 3, AC-6 2, and 1 for each of the rest. Measured against the tests' assertions, AC-1, AC-4, AC-5 and AC-8 are fully backed, and the six AC-9 compile_fail cases are real. AC-2 and AC-3 are tagged but not verified. AC-6 and AC-7 are partly verified, and so is AC-9's consumer half. Each gap traces to one cause: every member of a run is a `value-validity` application record, which `PluginReplayContext::request` refuses `NotReplayable` until QSL-980. Linear: QSL-1006 is related only to IR-672 (blocks) and STD-321 (blocked by). QSL-980 (Specced) mentions neither TC-1173, FR-404 nor QSL-1006. IR-672 is already blocked by QSL-980 as well.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-404-AC-1 | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md | examined |
| FR-404-AC-2 | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md | examined |
| FR-404-AC-3 | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md | examined |
| FR-404-AC-4 | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md | examined |
| FR-404-AC-5 | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md | examined |
| FR-404-AC-6 | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md | examined |
| FR-404-AC-7 | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md | examined |
| FR-404-AC-8 | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md | examined |
| FR-404-AC-9 | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md | examined |
| TC-1173 | spec/test-cases/TC-1173-the-replay-facade-carries-the-call-s-plugin-replay-context-from-the-same-run.md | examined |
| qsl-route items_from_requirements | qsl-route/src/request.rs | context_only |
| qsl-route Cargo.toml | qsl-route/Cargo.toml | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The tests for FR-404-AC-2 and AC-3, AC-7's DuplicateBoundKey arm and AC-9's request-building are blocked on QSL-980, and nothing in Linear records that. QSL-1006 has no edge to QSL-980, and QSL-980 never names TC-1173. Partial coverage is acceptable for this PR only with the non-asserting tags removed (SR-12358 FND-001..003) and a tracked follow-up: QSL-980's scope, or a ticket QSL-980 blocks, that adds TC-1173 steps 2, 3, 7 (dup) and 9 (request) to the test files. Otherwise the matrix keeps showing AC-2 and AC-3 as covered once QSL-1006 closes. | qsl-replay/tests/call_replay_context.rs:9-12 |
| FND-002 | low | TC-1173 asks for proof bounds from `qsl_route::items_from_requirements`, but qsl-replay has no qsl-route dev-dependency, so the tests use a hand-made bound at node [7;32]. qsl-route depends only on qsl-foundation and qsl-semantics, so a qsl-replay dev-dependency on it is cycle-free. family_outcome_layering.rs does not constrain qsl-replay's dev table. The other route is a root `tests/it` file, like TC-1089's call_requirements_items.rs. Either way, the QSL-980 follow-up needs it to write AC-2's request with the item's real bounds. | qsl-replay/Cargo.toml:47 |

## Verdict

No untraced production code: everything new is FR-404's `run_with_replay_context`, `RunWithReplayContext`, `CallReplayContext`, the two `PluginReplayRefusal` variants and the root `BackendId` re-export. No stubs, no `todo!`, no placeholder returns. The coverage gap is real but bounded. AC-2 and AC-3 cannot pass until QSL-980 replays application claims, and the PR body states this. The in-crate tests are a reasonable stand-in for request building, package binding and the limits copy. A partial delivery is acceptable once the tags tell the truth and Linear carries the follow-up (FND-001). Once QSL-980 lands, the tests need a rewrite, not just stronger asserts (SR-12358 FND-007).
