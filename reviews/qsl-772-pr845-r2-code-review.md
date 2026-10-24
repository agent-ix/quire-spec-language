---
id: SR-11089
title: "Code review (round 2) of quire-spec-language PR #845: supplied-value phase budgets and magnitude bound B"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0dc1e01879383b0727bef72a6ec9d3837ec82ae1; PR #845 diff vs main (44 files): Cargo.lock, qsl-semantics/src/value/supplied.rs, qsl-semantics/src/check/{check.rs,lowering.rs}, qsl-semantics/src/family/requirements.rs, qsl-eval/src/value/{mod.rs,expression/mod.rs}, qsl-replay/src/{execute.rs,execute/argument.rs,execute/composite_parity.rs,execute/value_parity.rs,execute/composite_domain.rs,execute/state_clause.rs,composite.rs,outcome.rs,request.rs,result.rs,result/wire/value.rs,supplied_limits.rs,limits.rs,lib.rs,spine/call.rs,spine/clause.rs,spine/lifecycle.rs,spine/native_result.rs}, src/command{.rs,/output.rs,/wire.rs}, tests"
review_set: subset
---
# Code review (round 2) of quire-spec-language PR #845

## Summary

Ticket: QSL-772 (code follow-on of QSL-681). Code review with the rust-review
lane folded in, one pass over `git diff origin/main...HEAD` at 0dc1e0187.
Oracle: QSL FR-098, FR-255, FR-277 (AC-6, AC-7), FR-321, FR-322, FR-358,
TC-830, TC-831, TC-906, and QSpec FR-323 AC-10..16 / TC-278 (ids only), QSV
FR-109 at QSV 542bc097. Reporting filter: wrong result, panic, failed AC,
vacuous or weakened test, broken gate; at most five findings.

Examined:
- Earlier review SR-9126 FND-001 (lenient unbudgeted fallback): fixed. No
  `admit_unbudgeted` or post-`UnsupportedHelper` plain `admits` remains; the
  two remaining `TypeEnvironment::admits` calls (spine reference binder,
  result-wire codec) are pre-existing non-supplied paths (clean).
- Cargo.lock: QSV 542bc097, quire-exact 9a3411d3, no Cargo.toml change (clean).
- `SuppliedLimits` A/H/B, defaults 16777216/16777216/4096, not setting rows,
  refused in `stage_limits` and by the settings reader (clean).
- One `SuppliedRun` per call/replay; admission via QSV
  `admit_supplied_bounded_numeric` with B passed explicitly from the run;
  every QSV error arm mapped by name: NumericSize/DecimalSize/NumericTypeSize
  to a B `limit` with operand, DecimalCapacity to Capacity, DecimalAllocation
  and Allocation to Storage (never a type refusal), UnsupportedHelper to the
  distinct typed stop (clean).
- Pre-call record and settlement: limit stops settle inconclusive/NoValue via
  `with_pre_call`, empty charges, `limit()` None; Input/Cancel/Storage/
  Capacity are typed refusals (clean).
- Consumers by signature of the new CallFailure/RunRefusal/ReplayRefusal
  arms: replay code/cause/stage, value_parity, composite_parity,
  state_clause, spine clause (both matches), native_result (both matches),
  OutcomeDocument, CLI `output::error`. No wildcard swallows them (clean).
- `read_supplied_limits`: closed, three required members, exact u64;
  null/negative/float/string/above-u64 refuse; CLI wire `supplied: null`
  refuses (clean).
- Original Cancel: replay, spine `execute`/`run` pass the caller's handle into
  the run (clean).
- Union arms: walks traverse member positions; lowering/union slot list are
  typed refusals; unreachable until QSL-503 authors unions (clean, one low
  recorded).
- Panic surface in new non-test code: none (clean).
- Root numeric domain check moved out of conversion (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Conversion now skips the Int/Rational domain and B check for an argument's root node (`if self.position != 0`), relying on the later S6a admission. `replay_composite_parity`'s F-3/F-4 `admit` runs conversion only and never S6a admission, so a parameter operand declared `Int[0, 9]` (or Rational) with value 12, or with a value over B bits, is now Admitted and compared instead of settling `RefusedInput` (or the B limit). main refused it in conversion. FR-358 requires admission through FR-098 conversion and S6a admission. Composite parity already takes scalar roots (TC-907 has an enum-root claim), and no test covers a scalar numeric root | qsl-replay/src/execute/argument.rs:567, qsl-replay/src/execute/argument.rs:588, qsl-replay/src/execute/composite_parity.rs:333-384 |

## Verdict

**FAIL: 1 blocking finding (FND-001).** SR-9126 FND-001 is fixed. The run,
the A/H/B budgets, the phase record, zero-charge settlement, the typed arm
consumers, the strict wire reader and the storage/type separation are
correct.

Fix for FND-001: in composite parity, run the converted operand through
`SuppliedRun::admit(SuppliedPhase::Admission, ..)` (one run for both
operands) and map its stops the way the conversion stops are mapped, or keep
the bounded root check inside conversion. Add a TC-907 vector with an
`Int[0, 9]` parameter operand at 12 that settles `RefusedInput`.

Recorded, not blocking:
- medium: conversion's Decimal leaf still calls the unbounded released
  `DecimalType::contains` (no B, no Cancel). A Decimal in a record or tuple
  field never meets B, because QSV does not descend into composite values.
  This is pre-existing, and FR-277 scopes B to Integer and Rational.
- low: a conversion-phase B stop on a nested Int/Rational names
  `witness-materialization`, its charged event, not `value-visit`.
- low: the union arms in `classify_domains`/`derive` number positions
  across members. Unreachable before QSL-503.
- low: plain `CheckedPackage::call`/`evaluate` build a fresh Cancel. The API
  takes no handle; spine and replay pass the caller's handle.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed a4a193942 | Composite parity `admit` now runs each converted operand through `run.admit(SuppliedPhase::Admission, ..)` on one run for both operands, so A is cumulative per call as FR-277 requires. The shared `stopped()` names all three `SuppliedFailure` arms. `tc_907_a_numeric_root_operand_is_admitted_by_its_domain_and_by_b` was red before the fix (qsl772-e1, EXIT 101) and is green after it. It asserts RefusedInput at index 0 for (12, 3) and at index 1 for (3, 12), that (3, 4) is compared, and that B=3 settles Incomplete(Admission) with NumericMagnitudeBits, limit 3. qsl-replay lib: 528 passed, 1 ignored; spine_run: 22 passed (qsl772-e2). Recorded low: a conversion-phase stop on operand 1 names parameter 0, because `convert_arguments` runs per operand |
