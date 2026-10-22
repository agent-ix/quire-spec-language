---
id: SR-12358
title: 'Code review of PR #1285: same-run CallReplayContext on the replay facade (code-review with rust-review lane)'
type: SpecReview
scope: 'agent-ix/quire-spec-language@eab4e9af3a6e2df760e5bba53d72b0bf96be2425; git diff origin/main...HEAD (PR #1285, branch qsl-1006-impl): qsl-replay/src/call_replay_context.rs, qsl-replay/src/call_requirements.rs, qsl-replay/src/lib.rs, qsl-replay/src/proof_result/plugin_replay.rs, qsl-replay/src/spine/call.rs, qsl-replay/tests/call_replay_context.rs, qsl-replay/tests/call_replay_context_root_only.rs; against FR-404, TC-1173, FR-290, FR-098, FR-071; ticket QSL-1006'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-404
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-1173
  type: reviews
---

## Summary

Ticket: QSL-1006. PR: quire-spec-language#1285, head eab4e9af3. Method: code-review with the rust-review lane. Evidence from a detached review worktree with its own target dir: `cargo test -p qsl-replay --test call_replay_context --test call_replay_context_root_only` exit 0 (7 + 1 passed); in-crate `call_replay_context::tests` 2 passed; `cargo test -p qsl-replay --doc CallReplayContext` 6 compile-fail doctests passed; `cargo clippy -p qsl-replay --all-targets --all-features -- -D warnings` clean. Probes, all reverted afterwards. (1) With `compile_fail` removed, each doctest fails for the intended reason: E0451 private fields, E0599 no `new`, E0599 no `default`, E0277 no `Deserialize`, E0614 no deref, E0599 no `as_ref`. (2) An eprintln in `a_member_request_equals_the_oracles_whatever_the_outcome` shows all three runs return `Err(NotReplayable)` from both the context and the oracle, so the `Ok` arm never runs. (3) arch-lint `api-surface --cg` over a tree that holds only the new root-only test reports T12-A FAIL, at line 4 (a doc comment) and lines 11-12. TC-1089's `call_requirements_root_only.rs` fails the same way.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-404 One compile | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md | examined |
| FR-404 Limits mapping | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md | examined |
| FR-404 Item binding | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md | examined |
| FR-404 Refusal order | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md | examined |
| FR-404 No consumer construction | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md | examined |
| FR-404 No unbound access | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md | examined |
| FR-404 Facade boundary | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md | examined |
| FR-404-AC-1 .. FR-404-AC-9 | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md | examined |
| TC-1173 | spec/test-cases/TC-1173-the-replay-facade-carries-the-call-s-plugin-replay-context-from-the-same-run.md | examined |
| FR-098 stale package_id code | spec/functional/FR-098-execute-a-replay-request.md | context_only |
| FR-290 UnknownOccurrence code | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md | context_only |
| run_with_replay_context / CallReplayContext | qsl-replay/src/call_replay_context.rs | examined |
| run_reading read closure | qsl-replay/src/spine/call.rs | examined |
| run_with_requirements | qsl-replay/src/call_requirements.rs | examined |
| PluginReplayRefusal ForeignRun/NotACallItem | qsl-replay/src/proof_result/plugin_replay.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `a_member_request_equals_the_oracles_whatever_the_outcome` is tagged FR-404-AC-2 but asserts none of AC-2's claims. All three runs hit `Err(NotReplayable)` on both sides (probed), so the `Ok` arm is dead. Nothing checks x = 9, selected_function `q`, replay completing `10`, or settling `inconclusive`/`replay-parity`, yet `quoin matrix` lists AC-2 as `tagged`. | qsl-replay/tests/call_replay_context.rs:199-248 |
| FND-002 | medium | The same test is tagged FR-404-AC-3 but asserts only the outcome kinds `Undefined` and `Incomplete`. Nothing checks r's request (`selected_function` r, `process-provider` obligation identity), the `refuted` settlement, or that the work_units = 0 run's request equals AC-2's. The matrix lists AC-3 as `tagged`. | qsl-replay/tests/call_replay_context.rs:199-248 |
| FND-003 | medium | In-crate `a_member_request_carries_the_run_and_replays` is tagged FR-404-AC-2, but it overwrites the context's members with `p`'s function-contract item, which no production run yields. It replays `p`, not `q`, and asserts `Refuted`, where AC-2 expects `inconclusive`/`replay-parity`. The AC-5 and AC-6 parts it checks (package, source and limits copied from the run) are real; the AC-2 tag is wrong. | qsl-replay/src/call_replay_context.rs:317-387 |
| FND-004 | medium | No test fixes the order ForeignRun before NotACallItem. Both ForeignRun cases pass a member index (q_index, or R2's equal q index), so swapping the two checks in `request` keeps every test green. One case with a foreign package_id and a non-member index (e.g. 99) expecting ForeignRun would close it. AC-7's DuplicateBoundKey arm for q's member is also untested, because the member is NotReplayable until QSL-980. | qsl-replay/tests/call_replay_context.rs:333-365 |
| FND-005 | medium | The FR-404-AC-9 root-only test builds no request and never names `ProofBound`, `CanonicalAssignment` or `ReplayRequestWire` (it passes `&[]` and `Vec::new()`), so it does not compile-check that `request`'s signature types are reachable from the root. It also skips `qsl_route::items_from_requirements`. The types are re-exported today (lib.rs 272, 280, 306), but the test would not catch a removal. | qsl-replay/tests/call_replay_context_root_only.rs:22-74 |
| FND-006 | low | FR-404-AC-9 says T12-A reports no call site for the root-only test, but no test or CI step runs T12-A over it. Run by hand, T12-A FAILs on it: line 4's doc comment trips the textual `qsl_replay::` scan, and every root path counts as a non-allowed caller. The precedent file call_requirements_root_only.rs fails identically, so the AC clause cannot be met as written. It needs a spec fix that scopes the clause to the forbidden module `qsl_replay::spine`. | qsl-replay/tests/call_replay_context_root_only.rs:4 |
| FND-007 | low | The oracle comparison will not strengthen cleanly when QSL-980 lands. The `Ok` arm compares only `package_id`, `obligation_identity` and `replay_target`, not member by member: `ReplayRequestWire` has no PartialEq or Debug, and `source_digests`, `dependencies`, `byte_provision`, `stage_limits` and `selected_function` go unchecked. The test also uses a dummy bound at node [7;32] and empty assignments, where TC-1173 asks for items_from_requirements' bounds and x = 9. | qsl-replay/tests/call_replay_context.rs:126-136,228-246 |
| FND-008 | low | The FR-404-AC-6 integration test matches `RunRefusal::Compile(_)` and never shows the refusal is the S1 token limit. It does not run `run_with_requirements` with `SpineLimits::default()`, which the AC asks for. stage_limits and accounting_limits are asserted only in the in-crate test, over a forged member. | qsl-replay/tests/call_replay_context.rs:318-331 |

## Verdict

The production code is correct and has no defects; every finding is in the tests and traces. `run_with_replay_context` compiles once: the members, `PluginReplayContext::new(checked, emitted, models, limits)` and the outcome all come from the one `run_reading` pass. Only `limits.spine` bounds parse, select and check. `package` uses `PackageLimits::default()`, and the call charges `call.accounting`. `run_reading`'s new `read(&checked, &emitted, &models)` is called at the same point as before. `run_with_requirements` reads `checked.package().graph()`, which is the same graph as the old `package.graph()` (`package = checked.package()`), so its behaviour is unchanged. `CallReplayContext` has private fields and no constructor, Default, Deserialize, Deref, AsRef, Borrow, From/Into or accessor for the wrapped context. Its `Clone`/`Debug` derives cannot forge one, and all six compile_fail doctests fail for the intended reason (probed). `request` checks ForeignRun, then NotACallItem, then delegates to FR-290 at the member's key. The codes match FR-098 (`stale_dependency`/`content-mismatch`) and FR-290 (`missing_declaration`/`missing-name`). Both new variants settle `failed`. `CallRequirement::new` and `CallRequirements::new` are `pub(crate)` and also back production `associate`, so they are not test-only seams. Rust lane: no new panic or unsafe surface in shipped code, no integer conversions, clippy clean. No other crate matches `PluginReplayRefusal` exhaustively. The PR's public-API changelog is accurate; `BackendId` really is new at the root. One small inaccuracy: the body says the root-only test "builds bounds by hand", but it builds no bounds. PR #1269 (S0 repoint) will conflict on imports; whichever PR lands second rebases. Verdict: mergeable after one small fix round. Remove the AC-2/AC-3 tags that assert nothing and the wrong AC-2 tag on the in-crate test (FND-001..003), add the ordering case (FND-004), and record the QSL-980-gated tests as a tracked follow-up (see SR-12359).
