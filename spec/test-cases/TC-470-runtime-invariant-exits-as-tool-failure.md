---
id: TC-470
title: "runtime_invariant exits 30 and outranks other diagnostics"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
---
# TC-470: runtime_invariant exits 30 and outranks other diagnostics

## Description

Verify FR-096-AC-12: the FR-301 exit status of `runtime_invariant`.

Scope: FR-096-AC-12.

## Test Procedure

1. For every `Code::all()`, assert `exit_code()` is 30 exactly for
   `RuntimeInvariant` and otherwise in 20..=22.
2. Corrupt a validated evaluation context so `evaluate` refuses with
   `runtime_invariant`; assert the diagnostic's `exit_code()` is 30.

3. Combine the code pairs {30,20}, {30,21}, {30,22}, {20,21}, {21,22}, {20,22} through
   the report's combining function (both orders); assert 30, 30, 30, 20, 21, 20.

Tag the tests `#[trace("TC-470", "FR-096-AC-12")]`.

## Expected Results

1. `Code::RuntimeInvariant` maps to exit status 30, and every other `Code` in
   `Code::all()` maps to 20, 21 or 22 (`only_runtime_invariant_exits_as_tool_failure`,
   `tests/it/native_boundaries.rs`).
2. With a validated context's private snapshot corrupted after a completed
   evaluation, `evaluate` returns `EvaluationOutcome::Refused` with a diagnostic of
   phase `Evaluate` and code `runtime_invariant` whose `exit_code()` is 30 and which
   is not incomplete. No corrupted value turns into a Boolean result
   (`private_corruption_cannot_turn_failed_checked_arithmetic_into_a_boolean`,
   `tests/support/runtime_evaluation_invariants.rs`).
3. `Category::most_severe` combines {30,20}, {30,21} and {30,22} to 30, {20,21} to 20,
   {21,22} to 21 and {20,22} to 20 in both orders, and an empty set to `None`
   (`a_report_combines_exit_codes_by_fr_301_severity`, `src/command/output.rs`).
