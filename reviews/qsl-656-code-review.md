---
id: SR-2445
title: "QSL-656 code review of PR #665 (accounting budgets settable by counter name), with rust-review lane"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0d34c5742a1c2841cec9fac4209324ff4949a4b5; PR #665 diff against origin/main (git diff origin/main...HEAD); qsl-foundation/src/setting.rs; qsl-foundation/src/diagnostic/stage.rs; qsl-replay/src/limits.rs; qsl-replay/src/request.rs; qsl-replay/src/scalar.rs; qsl-replay/src/execute/value_parity.rs; qsl-replay/src/outcome.rs; qsl-replay/src/spine.rs; qsl-replay/src/spine/call.rs; qsl-replay/src/lib.rs; qsl-semantics/src/check/refusal.rs; src/command.rs; src/command/wire.rs; tests in qsl-replay, qsl-package, tests/it/spine_run.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: reviews
---
## Summary

Ticket: QSL-656. PR: quire-spec-language#665, head 0d34c5742. Code review with
the rust-review lane folded in, scoped to the PR diff.

The PR names the ten `quire.value.accounting/v1` counters as `Setting`
variants with bare names, makes `Setting::kind` return `Option<LimitKind>`,
adds `AccountingLimits` and `CallerLimits.accounting`, lets a replay
request's `stage_limits` override a counter (`effective_accounting_limits`),
adds `setting` to the `quire-outcome/1` incomplete limit, and replaces
native-run/1's `call.work_units` with a closed `call.accounting` object.

Checked and clean:

- The override rule (a `stage_limits` counter entry replaces that counter
  of `accounting_limits`, every other counter keeps its value) is computed
  in one function, `ReplayRequestWire::effective_accounting_limits`. Decode
  stores its result, so every `request.accounting_limits()` reader (frame,
  state clause, composite admission, value parity, execute) gets the
  effective limits. Both identity sites, the value-parity claim
  (`value_parity.rs:140`) and `ValueIdentity::sent` (`scalar.rs:399`), call
  the same function, so the identity and the run cannot disagree.
- `effective_accounting_limits` skips names that do not parse. Decode
  refuses those names right after, so the skip never reaches a run.
- `ReplayRequest::to_wire` now writes the effective limits as
  `accounting_limits` and keeps the entries. Decoding that again gives the
  same effective limits, so the round trip keeps identity.
- `wire::Call`/`wire::Accounting` deny unknown fields. A present `null`
  fails `u64`'s visitor. `from_object` streams the map, so serde's derive
  sees a member named twice.
- No new integer casts, `unsafe`, locks or async code. The only new panic is
  the documented one in `LimitExceeded::new` (FND-001).
- Every `LimitExceeded::new` caller in the workspace passes a static stage
  setting (`Self::SETTING`, `limit_kind.setting()` over model, checking
  and identity kinds). None can reach the new panic today.
- `CallIncomplete::setting` and `ResultLimit.setting` derive the name from
  the counter through `AccountingLimits::setting_of`, an exhaustive match on
  `quire_exact::LimitKind`.

Focused tests (TC-914, FR-255 limits, FR-100 CLI tests) were not run by this review: the run sat queued on the build lock for over 1.5 hours. The verdict rests on reading the code and the tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `LimitExceeded::new` is a public `const fn` that now panics for ten of `Setting`'s 51 variants, and `Setting::kind` became partial (`Option<LimitKind>`) because accounting budgets, which are never stage limits, were folded into the stage-limit enum. No current caller can reach the panic, but any caller that maps a setting it does not control (for example iterating `Setting::ALL`, which QSL's own AC-1 test now has to filter) panics in library code. Fix: make the stage-only invariant a type, for example a `LimitExceeded::new` over a stage-setting type or a `Setting::Stage`/`Setting::Accounting` split, so the compiler checks it and no panic is needed. Or, at least, return a `Result` instead of panicking. | qsl-foundation/src/diagnostic/stage.rs:123-126; qsl-foundation/src/setting.rs:60 |
| FND-002 | low | `CallerLimits::for_request` now also folds accounting `stage_limits` entries into `CallerLimits.accounting` over the library defaults, not over the request's `accounting_limits`. Nothing in the replay path reads that field, because the call runs under `ReplayRequest::accounting_limits()`. A request therefore carries two accounting-limit values, and one of them is wrong (default-based) and unused. Fix: document that `for_request` leaves `accounting` unused for replays, or build it from the request's effective limits. | qsl-replay/src/limits.rs:157-173 |
| FND-003 | low | A stale doc comment still says the clause meter's accounting limits are "FR-100's `work_units`". FR-100 has no `work_units` member after this PR. It is `accounting`. | qsl-replay/src/spine/clause.rs:225 |
| FND-004 | low | The PR title reads "accounting budgets settable as accounting.<counter>". The implementation, spec and the ticket's correction comment all use bare counter names (`work_units`), and `accounting.work_units` is refused as an unknown setting (FR-255-AC-9). Fix: retitle the PR, for example "accounting budgets settable by counter name". | PR #665 title |

## Verdict

Approve with low findings. The override rule is implemented once, and both
identity sites and every run path use it. The wire change is closed and
strict. The public API changes are listed in the review report. Consumer
`origin/main` greps show no compile break. All four findings are low and
can be fixed in this PR's fix round.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The fix for FND-001 changes `qsl_foundation::setting::parse_operands` to return `Vec<(SettingName, u64)>`. quire-driver `origin/main` (5cddd5f6) depends on qsl-foundation and qsl-replay at `branch = "main"`. Its `LimitOverrides(Vec<(Setting, u64)>)` builds itself with `parse_operands(..).map(Self)` (`src/limits.rs:25`), `apply_to` matches the entries as `Setting`, and its test asserts `values() == &[(Setting::S1InputBytes, 5)]` (`src/limits.rs:86`). None of that compiles against this head, so the driver's next QSL bump breaks its build. Fix: land a quire-driver adaptation together with this merge. `LimitOverrides` should hold `SettingName`, route `Stage` to `apply_to` and `Accounting` to the `Call`'s accounting limits (which also gives driver FR-003 / IR-684 its `--limit <counter>=<n>` path). | qsl-foundation/src/setting.rs:268-270; quire-driver src/limits.rs:20-26,86 |

## Dispositions

Round 1, reviewed at b76b7a9c249a496564565a80606867be62af3da9 (fix commits
8c7fce235, be3fa8072, b76b7a9c2 on 0d34c5742). The verification was by
reading, with no build.

- FND-001 was fixed with a different design. The ten accounting variants
  are gone from `Setting`, and `Setting`, `Setting::kind -> LimitKind` and
  `LimitExceeded::new` are back to their `origin/main` form, with no panic
  and no `Option`. The new `AccountingSetting` (ten variants, `ALL`, `name`)
  and `SettingName::{Stage, Accounting}` carry the accounting names.
  `SettingName::from_name` tries the stage table first, then the counters.
  A stage name holds a `.` and a counter name does not, so no name is both,
  and the new foundation test checks it.
  - `parse_operands` detects a repeated `SettingName`, so `work_units=5`
    with `work_units=6` is refused on the second operand (FR-255-AC-9).
  - `Setting::from_name("work_units")` is `None`, as on `origin/main`.
  - `StageLimits` decode sorts entries into a stage map and an accounting
    map, and still refuses a name in neither table and `replay.input_bytes`
    (FR-263 Behavior 2). `to_wire` merges the two maps without collision.
  - `CallIncomplete::setting()` returns `AccountingSetting`, and
    `ResultLimit.setting` still renders the counter's bare name (FR-255
    Behavior 9, FR-286).
  - FR-255-AC-3 still holds: the test chains `CallerLimits.accounting.bounds()`
    into the code-name set, and AC-8 checks that each counter sets only
    itself.
  - The design is sound and consistent with FR-255 and FR-100.
- FND-002: `CallerLimits::for_request` now takes the request's accounting
  limits and overlays `stage_limits.accounting_entries()`. Every replay
  entry passes `request.accounting_limits()`, so `CallerLimits.accounting`
  equals the effective limits. The overlay is idempotent. `compile_package`,
  which has no request, passes the defaults.
- FND-003: the doc comment now reads "FR-100's `accounting` object".
- FND-004: the PR title at b76b7a9c2 reads "QSL-656: accounting budgets are
  settable by counter name". It is a PR-metadata edit with no commit.
- FND-005 (new this round) is open.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8c7fce235 |
| FND-002 | fixed | 8c7fce235 |
| FND-003 | fixed | 8c7fce235 |
| FND-004 | fixed | b76b7a9c2 |

## New findings (disposition pass 2)

Round 2 reviews ab1e899e5462eafe9ed43597c1b642411b070173, the rebase onto
main 70ddb73c (after #663 and #664) plus fix commit ab1e899e5. The
QSL-656 diff against main at ab1e899e5 was compared file by file with the
QSL-656 diff at 477c0d40d. Six files differ. In three of them the rebase
reverted main's own changes. This was found by reading, with no build.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | The rebase deletes main's `IntakeInputBytes => "intake.input_bytes", "I1 semantic-IR intake", InputBytes;` row from the `settings!` table. The head still references `Setting::IntakeInputBytes` in `qsl-foundation/src/intake_limits.rs:37,42` and `qsl-replay/src/spine.rs:253`, so qsl-foundation does not compile at ab1e899e5. FR-255's table still lists `intake.input_bytes`. Fix: restore main's row after `LibrarySingleArtifactBytes`. | qsl-foundation/src/setting.rs:92-93; qsl-foundation/src/intake_limits.rs:37,42; qsl-replay/src/spine.rs:253 |
| FND-007 | high | The rebase restores `pub model_limits: ModelNormalizationLimits` on `ClauseRunRequest`, a field main removed (main reads `request.limits.model`). Every `ClauseRunRequest { .. }` literal at the head omits it: `qsl-replay/src/spine/clause/tests.rs:688,1680,...` and `examples/config-version/spine.rs:380`. Those are missing-field compile errors, and the field would be dead. Fix: take main's struct and keep only the doc-comment change ("FR-100's `accounting` object"). | qsl-replay/src/spine/clause.rs:222-226 |
| FND-008 | high | The FR-255-AC-7 test `a_reached_budget_set_by_operand_names_its_setting` calls `run(...)` with seven arguments. After #663, `spine::run` takes an eighth, `cancel: &Cancel`, and every other call in the same module passes `&Cancel::new()`. qsl-replay's test target does not compile. Fix: pass `&Cancel::new()`. | qsl-replay/src/outcome.rs:1194-1202; qsl-replay/src/spine/call.rs:339-348 |
| FND-009 | medium | The rebase reverts main's FR-255 test changes in `limits.rs`. It re-adds the `(pending QSL-487)` row machinery (`PENDING`, `Row.pending`, the partition and the pending filter), which main deleted once QSL-487 landed. It also deletes main's `IntakeInputBytes` builder case (`mapped.set_bound(Setting::IntakeInputBytes, 35)` and the `model.intake` builder), so after FND-006, FR-255-AC-3's builder test no longer covers `intake.input_bytes`. Fix: take main's test bodies and re-apply only the accounting changes. | qsl-replay/src/limits.rs:236-246,419 |

## Dispositions (round 2)

Round 2, reviewed at ab1e899e5462eafe9ed43597c1b642411b070173.

- FND-001 to FND-004 stay fixed after the rebase. The range-diff shows
  that 7149a594b and 5d77325d4 carry 0d34c5742's and 8c7fce235's changes,
  and that c1133eefa equals b76b7a9c2. Their conflict resolutions are what
  introduced FND-006, FND-007 and FND-009. ab1e899e5 passes the effective accounting limits to
  the new `CallerLimits::for_request` call in `ReplayRequest::decode`.
  That call keeps only `.spine.model.intake`, so the accounting argument
  does not change its result. Passing the effective value is the only
  correct choice (FR-255-AC-11).
- FND-005: quire-driver `origin/main` is still 5cddd5f6, after a fresh
  fetch. Its `src/limits.rs:20,25` still builds
  `LimitOverrides(Vec<(Setting, u64)>)` from `parse_operands`, and no
  adaptation has landed.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-005 | still-open | quire-driver origin/main 5cddd5f6 is unchanged: LimitOverrides(Vec<(Setting, u64)>) is still built from parse_operands, which returns Vec<(SettingName, u64)>; no driver adaptation exists |
