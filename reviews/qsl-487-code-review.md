---
id: SR-2440
title: "Code review of quire-spec-language PR #652: B6 domain-package intake on the shared reader (QSL-487)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0552dd1f2fc86ae544feb58daf9f2d5670ef3422; PR #652 diff origin/main...HEAD (main bfeb258c): qsl-foundation/src/intake_limits.rs, qsl-foundation/src/setting.rs, qsl-semantics/src/model/intake.rs, qsl-semantics/src/model/intake/unit.rs, qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/refusal.rs, qsl-semantics/Cargo.toml, Cargo.lock, qsl-replay/src/{spine.rs,spine/clause.rs,spine/lifecycle.rs,request.rs,execute.rs,limits.rs,compile.rs,call_site.rs}, src/protocol_artifact/{work.rs,domain.rs,models.rs}, src/command.rs, spec/functional/FR-255, spec/test-cases/TC-722, and the call-site and test updates"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-260
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: reviews
---
# Code review of quire-spec-language PR #652

## Summary

Ticket: QSL-487 (B6). PR: quire-spec-language#652, head 0552dd1f2, six commits on
main bfeb258c. Companion: quire-specification#198, which has its own reviewer. The
Rust lane (rust-review) is folded into this file.

What the ticket asked for is present:

- `too_deep` and `IntakeLimit::NestingDepth` are deleted, with their scan tests.
  No `NestingDepth`, `MAX_DEPTH` or `too_deep` remains in code. The one
  `json::MAX_DEPTH` mention left is a stale doc comment (FND-006).
- `IntakeLimits` (64 MiB default, builder, `SettingLimits`) is a new
  `Setting::IntakeInputBytes`, reached through `SpineLimits`, the settings
  operation and a request's `stage_limits`. `IntakeLimit::setting()` returns that
  `Setting` and has a doc comment. `ModelRefusalCause::IntakeLimitExceeded` now
  carries `bound: u64` and `actual: u64` and maps to FR-255's `LimitExceeded`.
- `protocol_artifact::Limits` gains a public `intake` field. Its doc says it is
  used as given, and the clamp macro does not touch it. `verify_document` takes it.
- FR-056-AC-16's test now goes through `read_records` (intake.rs:5068-5070).
- The FCD rev is 68ace480 in qsl-semantics/Cargo.toml:43-44 and in both
  Cargo.lock entries. 68ace480 is FCD #264's merge commit and is on FCD main.
  There is no committed `[patch]` and no `.cargo/config`. quire-exact,
  quire-walk and quire-canonical are `branch = "main"`. quire-semantic-value is
  pinned by rev. That pin predates this PR (FND-009).
- The deep test exercises depth. It is 100,000 nested arrays, parsed and then
  dropped inside a 512 KiB thread. `quire_canonical::read`, the inexact-number
  walk, the JCS digest, both view builds and `PackageDocument`'s drop all run at
  that depth. Before this PR the same input refused `NestingDepth`, so the
  `expect` would fail. It does not reach `admit`, `read_records` or semantic-IR's
  `decide`. See SR-2441 FND-001.

Public API changes. Every one is a breaking signature or literal change, and the
PR body names them: `PackageDocument::parse`, `admit`, `admit_selections`,
`package_input`, `admit_unit(_with_cancel)`, `spine::select`, the observation
entry points, `SpineLimits` (new field), `ClauseRunRequest` (new field),
`protocol_artifact::Limits` (new field), and `ModelRefusalCause`/
`ReplayRequestRefusal::IntakeLimitExceeded` (`bound` widened, `actual` added).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | In the spine path, a document over `intake.input_bytes` never surfaces the intake limit. `package_input` parses under the caller's limit to key each document. On any refusal, including `IntakeLimitExceeded`, it keys the bytes by their raw SHA-256. The unit's model selection names the JCS digest, so `admit` finds nothing under it and refuses `missing_import`/`MissingSelection` at intake.rs:1006. It never reaches the parse that would name the limit and `intake.input_bytes`. Before this PR the bound was a fixed 64 MiB, so this case was effectively unreachable. Now `SpineLimits.intake`, a request's `stage_limits` and FR-260-AC-4's `--limit` can lower the bound, and a lowered bound gives a wrong cause that names no setting. Fix: key documents under a read that is not bounded by the caller's limit, for example `u64::MAX` or the payload ceiling, so that `admit`'s own parse under the caller's limit returns the limit refusal. Add a spine-level test (`select` with `intake` at `B` over a `B + 1` document) asserting `IntakeLimitExceeded`. | qsl-semantics/src/model/intake/unit.rs:82-94; qsl-semantics/src/model/intake.rs:1006-1016 |
| FND-002 | medium | `ClauseRunRequest` carries the intake bound twice: `limits.intake` is used for compile-time `select`, and the new `intake_limits` is used for admission's re-read. They are public fields that can disagree. With `intake_limits` below `limits.intake` and a document between the two, the unit compiles, but `model_views` maps the re-admission refusal to `fault("model-reconsistent-admission")`. A caller's own limit then surfaces as an internal fault, not a limit refusal. Fix: drop `intake_limits` and use `request.limits.intake`. The two in-repo constructors (execute/frame.rs:245, execute/state_clause.rs:232) already set it from `limits.spine.intake`. | qsl-replay/src/spine/clause.rs:219-223; qsl-semantics/src/model/observation.rs:720-721 |
| FND-003 | medium | The replay request's over-limit test no longer goes through `ReplayRequest::decode`. It calls `package_document_refusal` on a `PackageDocument::parse` result directly. Nothing tests the new wiring `StageLimits` → `CallerLimits::for_request(..).spine.intake` → `digest_of` in `decode`. If that wiring reverted to `IntakeLimits::default()`, every test would still pass. Fix: put `intake.input_bytes=4` in the wire request's `stage_limits` and assert, through the existing `decode` closure, that `decode` refuses `IntakeLimitExceeded` with bound 4 and actual 17. | qsl-replay/src/request.rs:1262-1288; qsl-replay/src/request.rs:619-621; qsl-replay/src/request.rs:673 |
| FND-004 | low | No test runs `verify_document` with a non-default `Limits::intake`, so nothing checks that the caller's value reaches admission. A reached intake limit there is folded into `Error::Invalid(Invalid::Model)` by `map_err(|_| ..)`, the same as a malformed model, and the limit is not reported. Today the 8 MiB `payload_bytes` ceiling hides this at the default intake bound. Fix: add one test that lowers `Limits::intake` below a valid document's size and asserts the refusal. Decide whether a reached intake limit should be reported as exhaustion rather than an invalid model. | src/protocol_artifact/domain.rs:247-262; src/protocol_artifact/work.rs:39-41 |
| FND-005 | low | The doc comment "The limits admission re-normalizes the domain packages under." now sits above the new `intake_limits` field, and `model_limits`, which it describes, has no doc. Fix: give `intake_limits` its own line ("The byte limit admission re-reads the domain packages under") and put the existing line back on `model_limits`. | qsl-replay/src/spine/clause.rs:783-785 |
| FND-006 | low | Stale depth text. intake.rs:6139 expects "a document within the reader's 200-deep bound". model_intake.rs:908-926 and :967 explain the test by `agent-ix-semantic-ir`'s "200-deep bound" and `json::MAX_DEPTH`. The new test's doc at intake.rs:6289 says "the old 128 cap", but the deleted bound was `json::MAX_DEPTH` (200). Fix: reword these to say there is no depth bound, or delete the history. | qsl-semantics/src/model/intake.rs:6139; qsl-semantics/src/model/intake.rs:6289; qsl-semantics/tests/it/model_intake.rs:908-926; qsl-semantics/tests/it/model_intake.rs:967 |
| FND-007 | low | rust-review: two new `#[allow(clippy::too_many_arguments)]` (`admit_current_snapshot`, `admit_clause_observations`) are added only to thread `IntakeLimits` next to `ModelNormalizationLimits` as a separate parameter. The same pair is now passed side by side through `model_views`, `admit_observations`, `population_universe_for` and `admit_frame_invocation`. Fix: bundle the two into one admission-limits value, for example a small struct or the existing `SpineLimits` slice, so these signatures stay under the lint and the pair cannot be split. | qsl-semantics/src/model/observation.rs:1172; qsl-replay/src/spine/clause.rs:538-546 |
| FND-008 | low | `PackageDocument` still derives `Clone` and `Debug`. Depth is now unbounded, and `serde_json::Value`'s derived `Clone` and `Debug` recurse once per level, so cloning or `{:?}`-formatting an admitted 100,000-deep document overflows the stack. No production path does either today. The type's custom `Drop` exists only to avoid the same recursion. Fix: drop the `Clone` derive, or implement it iteratively, and keep `Debug` shallow (for example the digest only). | qsl-semantics/src/model/intake.rs:294 |
| FND-009 | medium | Not introduced by this PR (it came in with #641, B4). `quire-semantic-value` is pinned `rev = "e0ada807..."` in the workspace `Cargo.toml`. The qsv family is one-copy and `branch = "main"` everywhere, and its siblings quire-exact, quire-walk and quire-canonical are on `branch = "main"`. A consumer that takes qsv from `branch = "main"` resolves a second copy of its types. Fix: `branch = "main"` with no rev, in whichever PR the lead assigns. | Cargo.toml:15 |

## Verdict

No high findings in the code. The deletion, the new setting, the `Limits::intake`
plumbing, the FCD bump and the FR-056-AC-16 move are correct and match the ticket.
The FCD pin is a rev pin used only for repeatable builds, with no `[patch]`.
`cargo clippy --locked --workspace --all-targets -- -D warnings` at 0552dd1f2:
exit 0, no warnings. Focused tests at 0552dd1f2: qsl-semantics `model::intake`
90 passed, qsl-semantics it `model_intake` 11 passed, qsl-replay `request::` +
`limits::` 13 passed, root `protocol_artifact` 7 passed.

Merge after FND-001 to FND-003 are fixed in this PR. FND-001 is the only
wrong-result path: a lowered intake limit is reported as a missing package.
FND-002 lets a public caller turn its own limit into an internal fault. FND-003
leaves the replay request's new wiring untested. The lows can ride the same fix
round. FND-009 predates this PR and needs a lead decision on where it lands.

Downstream breakage on merge (branch=main consumers; listed in the Linear
comment): quire-driver `src/upstream.rs:139` and `tests/drive.rs:387,422`;
quire-integration `tests/qsl_model_owner_admission.rs:57,90-92`. When
quire-protocol, which pins QSL 9395be42, is next bumped:
`tests/support/v2_handoff/mod.rs:228` builds an `artifact::Limits` literal with
no `..`.

## New findings (disposition pass 1)

Reviewed at be49d0e576454f5620fc611e037316bf255aedd0.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | The intake bound now rides `ModelNormalizationLimits`. `ClauseRunRequest` still carries that type twice: `limits.model`, used by compile-time `select`, and the older `model_limits`, used by admission's re-read. The bound is still one field per type, but a public caller can again set `model_limits.intake` below `limits.model.intake`. A document between the two then compiles, and `model_views` turns the re-admission refusal into `fault("model-reconsistent-admission")`. The in-repo constructors (execute/frame.rs:245, execute/state_clause.rs:232) pass `limits.spine.model` to both, so nothing in the repo diverges. The duplicate `model_limits` field predates this PR. Fix, if the ruling "carried once, inside SpineLimits" should hold for `ClauseRunRequest` too: drop `model_limits` and use `request.limits.model`. Otherwise record that both fields must agree. | qsl-replay/src/spine/clause.rs:219-224; qsl-semantics/src/model/observation.rs:720-726 |

## Dispositions

Round 1, reviewed at be49d0e576454f5620fc611e037316bf255aedd0 (fix commits 9f771b93..be49d0e57).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 4b1e37f63 (test settled in e77697e29). `package_input` keys each document under `UNBOUNDED` (unit.rs:77-79, 89-101), so `admit`'s own parse under the caller's `intake.input_bytes` returns the limit refusal. `select_names_the_intake_limit_for_an_oversize_document` (lifecycle/tests.rs) asserts a `StageFailure::Limit` naming `Setting::IntakeInputBytes`, bound `size - 1` and actual `size`. |
| FND-002 | fixed | 4b1e37f63. The `intake_limits` field and every `intake` parameter are gone. The bound is one field, `ModelNormalizationLimits::intake`, inside `SpineLimits.model`. A residual through the older `model_limits` duplicate is recorded as FND-010. |
| FND-003 | fixed | 4b1e37f63. `a_package_document_refusal_keeps_its_cause` now puts `intake.input_bytes=4` in the wire request's `stage_limits` and asserts that `ReplayRequest::decode` refuses `IntakeLimitExceeded { bound: 4, actual: 17 }`. |
| FND-004 | fixed | 4b1e37f63. A reached intake limit maps to `Error::Incomplete(Exhaustion { dimension: PayloadBytes, limit: bound, requested: len })`, and `verify_document_reads_under_the_callers_intake_limit` tests bound = size (ok) and size - 1 (exhaustion). |
| FND-005 | fixed | 4b1e37f63. `intake_limits` is gone. The `CompiledRun.model_limits` doc reads "The limits admission re-reads (`intake.input_bytes`) and re-normalizes the domain packages under." |
| FND-006 | fixed | 4b1e37f63. intake.rs:6148 reads "a deeply nested document admits under its JCS digest". The 100,000-deep test's doc drops "the old 128 cap". The model_intake.rs H3 doc and expect message now name only serde_json's recursion limit. No 200-deep, `MAX_DEPTH` or `NestingDepth` text remains about intake. |
| FND-007 | fixed | 4b1e37f63. Both new `#[allow(clippy::too_many_arguments)]` are removed. The intake bound rides `ModelNormalizationLimits`, so `admit_current_snapshot` and `admit_clause_observations` keep their original arity. |
| FND-008 | fixed | 4b1e37f63. `#[derive(Debug, Clone)]` is removed from `PackageDocument`. A hand-written `Debug` prints only `jcs_digest` (intake.rs:307-313). |
| FND-009 | fixed | 4b1e37f63. `quire-semantic-value` is `branch = "main"` (Cargo.toml:15). Cargo.lock resolves the same commit e0ada807 through `?branch=main`. |
