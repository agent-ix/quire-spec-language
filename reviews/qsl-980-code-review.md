---
id: SR-12368
title: "Code review of quire-spec-language PR #1366: application-claim replay by occurrence key (QSL-980)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@4fd9be8a6; git diff origin/main(e5554bc9f)...4fd9be8a6: qsl-replay/src/execute.rs, qsl-replay/src/witness.rs, qsl-replay/src/proof_result/plugin_replay.rs, qsl-replay/src/spine/lifecycle.rs, qsl-replay/src/admitted.rs, qsl-replay/src/outcome.rs, qsl-replay/src/execute/value_parity.rs, qsl-replay/src/call_site.rs, qsl-semantics/src/check/claims.rs and the test files; read for context: FR-098, FR-290, FR-404, FR-062, TC-1173, quire-semantic-value Location, agent-ix/quire-contract-codegen main 820bcbec"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
---
# Code review of quire-spec-language PR #1366 (QSL-980)

## Summary

Ticket: QSL-980. Rust review folded in (`/rust-review` lane). Claim
selection is placed correctly: `function_claim` runs after `recompile`
(stale `package_id`) and `lookup` (`UnknownFunction`) and before `select`
and the argument join. It compares the whole key (node, role, ordinal),
gives `ClaimOccurrence` the `missing_declaration`/`missing-name` code, and
returns `Err` before any meter exists, so nothing is charged. `r` at its
`declaration` still refuses `NotAPredicate` through `select`. The
unread-parameter fill follows the FR-098 rule (0, or the range end nearest
0), and the Witness path uses `decode_optional` without changing `decode`.
`PluginReplayContext::request` builds application items for
`callable_by_name` owners. The planner's case `r(x) = x + 1 > 3` passes: the
`+` and `>` items settle `replay-parity` at `x = 0`, and the
function-contract item settles `refuted`.

Public-API changelog: `ReplayRefusal::ClaimOccurrence` is accurate. The only
exhaustive matchers are in this workspace (`admitted.rs`, `outcome.rs`), and
both are updated. CG main (820bcbec) has no exhaustive match on
`ReplayRefusal`. Its function requests carry `call_site`'s
`site.site.declaration`, so the placeholder-key refusal does not break CG.
Focused tests at the head pass: `qsl-replay --lib` (plugin_tests,
call_replay_context, application_claim: 20 passed),
`--test call_replay_context` (8) and `--test call_replay_context_root_only`
(1).

The one real defect is the recursion case the coder flagged. A temporary
local probe test (reverted, not pushed) measured it at the head.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The observation matches by static `Location` only, so a stop at the claim site inside a recursive call of the same function counts as the outer evaluation. FR-098 "Application claim: the observation" says such an evaluation "is not observed", so the stop should settle `NoValue`. Probe at the head: `function f using v(x: Int[0, 9]): Integer pure decreases(x) { 10 mod x + (if x > 0 then f(x - 1) else 0) }`, at the `10 mod x` record with `x = 1`. The outer application `10 mod 1` is defined, and the inner `f(0)` stops undefined at the same `Location`. Replay settles `Reproduced(Boolean(false))`, which is wrong: the claim holds at `x = 1`. The replay oracle therefore accepts a backend counterexample that does not refute the claim at its own assignment. A Witness request would settle `reproduced-with-evaluated-witness` the same way. The FR-098 argument that unread-parameter fills are sound also assumes nested calls are not observed, so the fill can feed this case too. `quire_semantic_value::Location` carries no frame, so this cannot be fixed by comparing locations. Fix: have S6a report the stop's call depth (or the frame it belongs to) and observe only depth 0. As a smaller and conservative alternative, settle a site stop `NoValue` whenever the owner is in a recursive group (FR-092/FR-146): that loses a reproduction but never invents one. Add the probe as a TC-444 test. | qsl-replay/src/execute.rs:849-858,882-884 |
| FND-002 | low | The "completes a value outside the record's result bound" arm (FR-098, a SHALL, stated as defensive) is not implemented. A completed call always settles `Success`/`true`, and no comment or fault records that the arm was omitted on purpose. This fails in the safe direction (a missed reproduction), but the code says nothing about it. Fix: add a comment citing FR-098's "no checked package reaches it", or get a spec ruling that removes the arm. | qsl-replay/src/execute.rs:869-873 |
| FND-003 | low | Hidden test-only global state in a production module. `compose()` writes every composed package's declaration keys into a `#[cfg(test)]` `thread_local!` map (`DECLARATIONS`), and `execute/tests.rs::declaration_key` reads that map and falls back to `test_key()` when there is no entry. A test whose package was composed on another thread, or not through `compose`, silently gets the placeholder and then fails with a puzzling `ClaimOccurrence`. Fix: pass the declaration key from the `ComposedUnit` at the `request(...)` call sites, or give `request` the `ComposedUnit`, and delete the registry. | qsl-replay/src/spine/lifecycle.rs:812-858; qsl-replay/src/execute/tests.rs:129-139 |
| FND-004 | low | The public-API changelog leaves out a behaviour change that consumers can see: `PluginReplayContext::request` now builds requests for application items that it used to refuse `NotReplayable`, so an item that settled `inconclusive`/`replay-refused` can now settle `refuted` or `replay-parity`. The driver (IR-672) and CG (IR-968) consume this. Fix: add one changelog line. | qsl-replay/src/proof_result/plugin_replay.rs:193-228 |

## Verdict

**Not mergeable** until FND-001 is fixed. Claim selection, the refusal
order, the codes, the no-charge behaviour, the unread-parameter fill and the
application-item builder are correct. The changelog's `ClaimOccurrence` line
is accurate, and it breaks nothing in CG. FND-001 is a wrong result that a
probe reproduced, in a case the spec names explicitly.

Coder's recorded gaps, judged:
- Recursive-call indistinguishability: this is a correctness hole. It
  settles `reproduced` wrongly (FND-001). It does not make the
  property-level verdict unsound, because the inner assignment really
  violates the claim, but it confirms the wrong counterexample.
- "Completes outside the result bound" arm unimplemented: acceptable, since
  it fails safe, but the code should say so (FND-002).
- "Completes 10" unobservable and the unnamed `extent_roots()` accessor:
  both are spec gaps. See SR-12369.

Merge-order note: #1269 (IR-583 S0) touches 13 of this PR's 17 files
(`execute.rs`, `admitted.rs`, `outcome.rs`, `plugin_replay.rs`,
`lifecycle.rs`, `claims.rs`, and others). Whichever PR lands second must be
rebased and re-run on its focused tests.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The FND-001 fix is conservative: a claim whose owner is in a recursion group never reproduces, including a real undefined stop in the outer call (for example `f` at `x = 0`). FR-098 "the observation" still says the executor SHALL observe each evaluation in the call's own body, so code and spec now disagree for recursive owners. This fails safe (a missed reproduction). Mutual recursion is covered by the same `recursion()` check: a local probe with `g`/`h` at `x = 2` settles `NoValue` with the fix and `Reproduced(false)` without it. That case is untested. Fix: either amend FR-098 to state the recursive-owner rule (draco), or file the S6a frame-depth follow-up and add a mutual-recursion case to TC-444. | qsl-replay/src/execute.rs:849-861,1367-1369 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 10999567d: `failed_at_site` is false when the owning function's node has `recursion()`. `tc_444_a_stop_inside_a_recursive_call_is_not_the_outer_observation` asserts `NoValue` at `x = 1`. I checked that the test fails at the head with the guard reverted (left `Reproduced(Some(Boolean(false)))`, right `NoValue`). |
| FND-002 | fixed | 10999567d: a comment cites FR-098 and says the omission fails safe |
| FND-003 | fixed | 10999567d: `declaration_key` panics with a clear message when the package was composed on this thread but lacks the function. It still returns the placeholder only for a package id this thread never composed, which refuses before claim selection. |
| FND-004 | fixed | 10999567d (PR body edit): the changelog gains the `PluginReplayContext::request` line, naming the IR-672 and IR-968 consumers |
