---
id: SR-2446
title: "QSL-656 gap analysis of PR #665 (FR-255-AC-7 to AC-11, FR-100-AC-3/6/12, TC-914)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0d34c5742a1c2841cec9fac4209324ff4949a4b5; PR #665 diff against origin/main; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/functional/FR-098-execute-a-replay-request.md; spec/functional/FR-357-replay-scalar-parity-claims.md; spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md; spec/functional/FR-263-replay-at-any-depth-under-the-request-limits.md; spec/test-cases/TC-914-accounting-budgets-are-settable-by-counter-name.md; QSpec FR-461 (read-only, qspec main fbd6ea0f)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
---
## Summary

Ticket: QSL-656. PR: quire-spec-language#665. This is a planless gap
analysis. `quoin matrix --repo . --json` at 0d34c5742 shows every changed or
new criterion as `tagged`: FR-255-AC-7 to AC-11, FR-100-AC-3, AC-6 and AC-12.
Each binding was then read for oracle strength.

QSpec check: FR-461 Behavior 6 ("That counter name is its setting at every
entry point") and Behavior 3 (one name at every entry point) back the bare
counter names and the `stage_limits` entry. Behavior 2's dotted rule covers
stage limits only, so the bare names do not conflict with it.

Per criterion:

- **FR-255-AC-7**: `outcome.rs::a_reached_budget_set_by_operand_names_its_setting`.
  It takes the limits from `CallerLimits::from_operands(["work_units=0"])`,
  runs `seven` from `tests/fixtures/spine-compile.native` and compares the
  `result` to the AC's literal JSON, `setting` included. The `1000000` case
  must complete with 7. Strong oracle.
- **FR-255-AC-8**: `limits.rs::each_accounting_setting_sets_its_counter_alone`.
  The defaults come from the spec's own accounting table, read by
  `include_str!`. It sets each counter alone and checks the other nine, and
  checks `u64::MAX`. Strong: it is independent of the code's list.
- **FR-255-AC-9**: `a_malformed_accounting_operand_is_a_usage_refusal_naming_it`.
  It covers all six operands, each with an exact operand and cause. Strong.
- **FR-255-AC-10**: `limits.rs::a_stage_limits_counter_replaces_only_that_accounting_counter`
  checks the struct with exact equality.
  `composite.rs::tc_914_a_stage_limits_counter_entry_sets_the_budget` decodes
  `accounting_limits.work_units = 10000` with the entry at 8 and asserts that
  the decoded limit is 8 and that `value_occurrences` is kept. For FR-098-AC-9's
  counterexample it asserts `Incomplete` `WorkUnits` at bound 8 and `NoValue`.
  The raised entry (10000, over `accounting_limits.work_units = 0`) must
  reproduce with the same charges as an unlimited run. Strong: both
  directions of the override are exercised.
- **FR-255-AC-11**: `tc_914_a_split_accounting_limit_gives_the_same_claim_settlement_and_charges`.
  This is the whole-vs-split value-parity pair, with fitting limits and with
  `work_units` 0. It asserts equal `claim()` and that `claim().limits` equals
  the effective limits. That assertion would fail if `value_parity.rs`
  bound the raw `accounting_limits`. Results are compared by `Debug`, which
  includes `charges` and the agreement's claim. With `work_units` 0, the
  split request's raw value (12345) would Agree, so the `Incomplete`
  assertion proves that the run uses the effective limit. Strong for
  `replay_value_parity`. FND-001 covers the gap.
- **FR-100-AC-3**: `tc_450_step_4_malformed_work_units_refuses`. It covers
  every operand the AC lists, plus the refusal table's member-named-twice
  case. Each must give `invalid-request` at stage `request`, exit 20.
- **FR-100-AC-6**: `tc_451_step_7_zero_work_units_is_incomplete`. It drives
  `accounting {"work_units": 0}` end to end through the CLI, which proves the
  `call_accounting` wiring.
- **FR-100-AC-12**: `command.rs::call_accounting_reaches_the_call_as_its_limits`.
  It checks exact `ScalarLimits` for 1 to 10, `{work_units: 7}`, `{}` and an
  omitted member. Strong. `run_complete` passes `call_accounting(...)`
  straight into `spine::Call`, and AC-6 exercises that end to end.

Gap judged: the rule is "stage_limits overrides accounting_limits per
counter; identities use the effective limits". The implementation is
correct at every site. The test gap is FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-255 Behavior 10 requires every identity that binds the request's limits to use the effective accounting limits. The PR changes two identity sites to `effective_accounting_limits()`, but only one is tested: `replay_value_parity`'s claim (TC-914 AC-11). The other is `ValueIdentity::sent`, the public helper a consumer uses to build the identity of the claim it sent, which FR-357-AC-13 requires to equal `report.claim()`. No test calls it on a request whose counters are split into `stage_limits`. `tc_904_a_value_report_carries_the_sent_claim_on_every_outcome` uses wires with no accounting entries, and TC-914 never calls `sent`. Reverting `scalar.rs:399` to `wire.accounting_limits` would leave every test green, while `sent(split) != split.claim()` for every consumer. Fix: in the AC-11 test, also assert `split.claim() == &ValueIdentity::sent(&split_wire, &integer(4))` and `sent(&whole_wire, ..) == sent(&split_wire, ..)` in both runs. | qsl-replay/src/scalar.rs:394-401; qsl-replay/src/execute/tests.rs:1692-1720 |
| FND-002 | low | FR-255 Behavior 6 now says `<name>` is "a setting name from either table", but its refusal sentence still reads "When `<name>` is not in the table". Behavior 8 restates the rule for accounting names, so this cannot be misread in practice, but the two sentences of one behavior disagree. Fix: change it to "is in neither table". | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:169-173 |

## Verdict

Changes requested (one medium, test only). The behavior is implemented
correctly, and every new or changed criterion has a tagged test with a
strong oracle. FND-001 adds an assertion that pins `ValueIdentity::sent` to
the effective limits. FND-002 is a one-word spec fix.
