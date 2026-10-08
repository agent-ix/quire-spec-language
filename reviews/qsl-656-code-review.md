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
