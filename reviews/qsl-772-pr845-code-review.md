---
id: SR-9126
title: "Code review of quire-spec-language PR #845: bound supplied-value admission and conversion by phase budgets"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@1d128adad6f111ba0d72d666ed08c860f381aea9; PR #845 diff vs main (37 files): Cargo.lock, qsl-semantics/src/value/supplied.rs, qsl-eval/src/value/{mod.rs,expression/mod.rs}, qsl-replay/src/{execute.rs,execute/argument.rs,execute/composite_parity.rs,execute/value_parity.rs,composite.rs,outcome.rs,request.rs,result.rs,supplied_limits.rs,limits.rs,lib.rs,spine/call.rs,spine/clause.rs,spine/lifecycle.rs,spine/native_result.rs}, src/command{.rs,/output.rs,/wire.rs}, xtask/src/seam_probe.rs, tests"
review_set: subset
---
# Code review of quire-spec-language PR #845

## Summary

Ticket: QSL-772 (code follow-on of QSL-681). Code review with the rust-review
lane folded in, one pass over `git diff origin/main...HEAD` at 1d128adad.
Oracle: QSL FR-098, FR-255, FR-277, FR-321, FR-322, TC-830, TC-831, TC-906,
ADR-014 B-2, and QSpec FR-323 AC-10..14 / TC-278 RE-12..22 (cited by id only).
Reporting filter: wrong result, panic, failed AC, vacuous test, a budget that
does not bound work, or a broken gate; at most five findings.

Examined:
- Cargo.lock: only `quire-semantic-value` moves, 60107c5f (main, #843) to
  f4214133. f4214133 is an ancestor of QSV origin/main (verified with
  `merge-base --is-ancestor`). No Cargo.toml change (clean).
- `SuppliedLimits`: two fields, default 16777216 each, not setting rows, not
  in `stage_limits`; a `stage_limits` entry of either name refuses
  `StageLimitEntry` before any stage, and the settings operation refuses the
  names as unknown settings (clean).
- `SuppliedRun`: one per call, carries both QSV budgets, the path arena and
  the caller's Cancel; nothing resets between arguments; conversion-initiated
  membership spends H, final membership spends A (clean, except FND-001).
- Pre-call stop record `{phase, parameter, event, locus, consumed, cause}`
  with limit `{limit_kind, limit, next_charge, counter (Count | Overflow)}`
  and no charge point; replay limit stop settles `inconclusive`/`NoValue`
  with `ReplayResult::pre_call_limit()`, empty charges and `limit() == None`
  (asserted by `pre_call()` in every limit test) (clean).
- The removed `exceeded_before_call` fabricated a kernel `Incomplete` with
  `ChargePoint::FunctionCall`; C and occurrences are now conversion-phase
  pre-call records (clean).
- `CallFailure::from(SuppliedFailure)`: Input stays `Input`, Cancelled stays
  `Cancelled`, limit/storage/capacity go to `Supplied`, unqualified to
  `UnsupportedHelper`, fault to `Fault` (clean).
- New arms in every consumer by signature: replay `code`/`cause`/
  `replay_stage`, `call_failure_to_replay_refusal`, spine
  `convert_call_failure`, `check_clause_admitted`, `check_frame`,
  `RunRefusal::{scope,code,cancel_cause}`, `OutcomeDocument`,
  native_result refusal writer (both matches), CLI `output::error` Details,
  composite_parity and value_parity. No wildcard arm absorbs them; the new
  `_ =>` lines in argument.rs are the pre-existing shape/outcome fallbacks
  with `refuse` renamed (clean).
- `read_supplied_limits`: closed object, both members required, exact u64;
  the CLI wire uses it through `deserialize_with` and feeds `Call.supplied`
  (clean).
- Request identity: omitted and explicit defaults decode to equal
  `ReplayRequest`s, and either changed ceiling makes them unequal. The replay
  facade has no request digest beyond the decoded request, so equality of the
  decoded request is the identity under test (clean).
- Rebase fairness: `git range-diff 02a7d9414..2a4deb4b1 64937799f..1d128adad`
  shows all five commits identical (`=`). The base moved only by #843's
  squash (evaluate.rs `construction_fault` and two review files), which the
  PR does not touch. No conflict resolution exists to be wrong (clean).
- Panic surface in new non-test code: none (`unwrap_or(u64::MAX)` on size
  conversions, no unwrap/expect/index) (clean).
- Lenient default for unqualified helper families (`admit_unbudgeted`)
  (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The default lenient route for an unqualified helper family re-admits the whole argument from its root with `types.admits(expected, value)`, with no budget and no Cancel. Any argument that contains an Int[lo,hi], Rational or Decimal leaf is therefore admitted at A = (events before the first such leaf), whatever its size: `Sequence<Int[0,9]>[0,100000]` with 100,000 elements is admitted at A = 3. TC-906's own fixture (`Outer` with `Int[0,9]` fields) hits it, so its A-1 control admits instead of denying. FR-277 requires that neither ceiling is ever unbounded, that no phase is bypassed and that the original Cancel reaches every helper. TC-906 step 9 forbids invoking an unqualified substitute | qsl-semantics/src/value/supplied.rs:550-560, qsl-semantics/src/value/supplied.rs:572-620 |

## Verdict

**FAIL: 1 blocking finding (FND-001).** The rest holds: the typed carrier, the
cumulative A/H budgets, the phase record, zero-charge pre-call settlement, the
consumer arms, the strict wire reader and request identity are correct and
tested at the result level.

FND-001 is a failed AC, not a documented temporary limitation. The temporary
limitation FR-277 allows is narrower. Unhooked numeric subordinate work is a
helper-cost premise included in its initiating event. The logical walk around
that helper (value visits, child scheduling, type-link pairs, Cancel polls)
must stay budgeted. Fix: QSV (branch=main, landed with this PR) exposes a
supplied-admission policy that continues past `RequiresQualification` and
invokes the helper inline, as its plain `admits` already does internally.
Every logical event stays charged and Cancel is polled after the helper. QSL
uses that policy by default and keeps `unbudgeted_families()` and the opt-in
strict refusal. Decimal stays an IR-718/IR-719 qualification blocker either
way. Add a test that A-1 denies for a numeric-domain collection.

Recorded, not blocking:
- The strict opt-in returns a runtime `UnsupportedHelper`. FR-277 AC-6 calls
  an unqualified path a qualification blocker, not a runtime unsupported
  result. It is opt-in and QSV-shaped.
- CLI `request_digest` hashes raw request bytes, so omitted and explicit
  `limits.supplied` digest differently. This is pre-existing for every
  limits member.
- Plain `CheckedPackage::call`/`evaluate` build a fresh `Cancel::new()`, since
  that API takes no handle. Spine and replay pass the caller's handle.
- The Storage/Capacity arms are untested because QSV has no reservation
  injection seam.
- The command-wire projection of the pre-call record waits on the shared
  result contract, as FR-277 directs.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed 872752303 | The lenient `admit_unbudgeted` route is deleted. Numeric families are admitted through QSV `admit_supplied_bounded_numeric` under A/H and B (6c4b99a72), and the stops QSV still raises are the typed `UnsupportedHelper`. Verified at 0dc1e0187 (SR-11089) |
