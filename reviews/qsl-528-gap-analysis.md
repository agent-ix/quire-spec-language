---
id: SR-12190
title: "Gap analysis of quire-spec-language PR #1029: QSL-528 FR-126 TP-2 and TP-3 rows on the EN-1 product"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@64ba23327be808d3dc22067405cd79e417cebf8a; FR-126-AC-2 (Counter rows), FR-126-AC-4 (Restless fixture only), FR-126-AC-6 (Restless rows), FR-126-AC-9 (TP-3 and TP-2 rows); FR-125-AC-6 (context); TC-521, TC-537; qsl-analyze/tests/it/model_check_safety.rs, qsl-analyze/tests/it/model_world.rs, qsl-analyze/tests/it/model_check.rs, qsl-eval/tests/it/{automaton_kernels,property_automata}.rs, and the production files they test"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1029

## Summary

Ticket: QSL-528. PR quire-spec-language#1029. Planless, diff-scoped gap
analysis. Plan completion: not assessed. I traced criteria to tagged tests
by reading. No test was run (reviewer-only brief). The repository tags
tests by TC id only (`#[trace("TC-521")]`), so each binding below is judged
against the TC procedure step and the AC row the test's doc names, and on
whether the test can fail on that behaviour.

Trace bindings in the diff:

- `bounded_each_clauses_are_decided_on_the_product` -> TC-521 (FR-126-AC-2,
  the two `Counter` `on each` rows; TC-521 step 2): correct. Both outcomes
  and the `0 -inc-> 1` prefix are asserted, and the prefix is read back by
  re-execution. AC-2's two §6 rows are not claimed (declared: the FR-120
  frame gives 1001² candidates).
- `a_long_interval_materializes_automaton_states_as_the_product_reaches_them`
  -> TC-521 (FR-126-AC-6, the two `Restless` rows; TC-521 step 6): correct.
  It asserts `Stopped{ResourceExhausted, {MaxAutomatonStates, 50}}` with
  `automaton_states == 50`, then `Violated` with at least 101 consecutive
  unhealthy positions on the re-executed prefix. Both can fail.
- `a_formula_violation_before_an_undefined_position_and_an_unread_position`
  -> TC-537 (FR-126-AC-9, the TP-3 row and the TP-2 row; TC-537 step 3):
  correct. The TP-3 row asserts `kind: Formula` and the prefix `0 -inc-> 1`.
  The TP-2 row writes `6 mod (2 - c.value) = 0 and c.value = 1` for the
  AC's `6 / (2 - c.value) = 6`, because S3 types `mod` and not `/`. The
  truth pattern is the same (false at 0, true at 1, undefined at 2), and the
  test fails if position 2 is evaluated.
- `a_terminal_state_closes_a_bounded_monitor_with_false_letters` -> TC-521:
  wrong. See FND-002.
- `a_terminal_state_stutters_under_infinite_trace` -> TC-521: wrong. See
  FND-002.
- `an_empty_or_unbounded_subject_explores_nothing` -> TC-521 (renamed; the
  TP-3 refusal half was removed because TP-3 is now decided): correct for
  what remains (the AC-6 `RequiresBound` row and `NoInitialState`).

Untagged and fine: `the_monitor_rejects_at_the_first_bad_prefix`,
`a_liveness_clause_and_a_guarded_activation_are_refused`, and the
`qsl-eval` monitor tests.

The `Restless` fixture (`test/health`, written in `model_world.rs`) is
faithful to FR-126-AC-4. It has `healthy: Boolean` and `failures: Int[0,
1]`. `fail` modifies both, has no precondition, and posts `not healthy and
failures = 1`. `recover` modifies `healthy` only, with `pre not healthy`
and `post healthy`. The initial state is `healthy` true and `failures` 0.
AC-4's `terminal any` is not written. That has no effect: with no
precondition on `fail`, no Restless state is terminal, and terminal
declarations only change the deadlock-freedom item. The clause variable is
`c` and the object key is `s`, which matches the AC's `s.healthy` under
binding `c = s`.

Declared and not findings: the ADR-018 §6 rows of AC-2 (spec/fixture
decision for the planner), the TP-4 rows of AC-1, AC-3 to AC-5, AC-7 and
AC-9 (QSL-530), and AC-4's base `Health` row (a TP-4 formula).

## Verdict

Coverage of the claimed rows is sound for what the tests assert. Two tags
over-claim TC-521 (FND-002), which is this lane's main failure mode, but
the TC was already tagged by other tests on `main`, so no AC changes
status. The blocking gap is the code defect SR-12189 FND-001: no test
exercises a TP-2 or TP-3 position where the monitor's next step does not
read an atom that is undefined (FND-001). Only majors block, so this
artifact adds none of its own beyond tracking that test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test covers ADR-018 UE-1's "value of every atom of the claim" for a TP-2 or TP-3 clause at a position where the monitor's next step does not constrain the undefined atom. That is the case SR-12189 FND-001 gets wrong. Every UE test in the diff puts the undefined atom on a guard the monitor still reads, so the lazy `reads` filter passes them all. Add `holds(c.value >= 0) release holds(6 mod (2 - c.value) >= 0)` over `Counter`, infinite-trace, which must be `Violated`, `UndefinedEvaluation` at position 2. Add a TP-2 `on origin` row with the undefined atom inside the horizon after the claim is decided. | qsl-analyze/tests/it/model_check_safety.rs:322-378; qsl-analyze/src/model_check/check.rs:668-674 |
| FND-002 | medium | Two tests carry `#[trace("TC-521")]`, but TC-521's procedure has no step for the behaviour they test. `a_terminal_state_closes_a_bounded_monitor_with_false_letters` tests FR-126's TP-2 closed-boundary closure at a terminal state (`eventually[0,5] holds(c.value > 3)` `on origin`). TC-521 steps 1 to 7 contain no closure row. `a_terminal_state_stutters_under_infinite_trace` tests a TP-3 claim over the terminal stutter. TC-521 step 3's stutter row (AC-3) is the TP-4 `always eventually holds(c.value = 0)` claim with the stutter marker set, which this test does not run. Both tests are sound, but the tag claims TC-521 evidence they do not provide. Fix: remove the two tags, or add the closure and TP-3 stutter rows to a TC (TC-521 or a new one) and tag them there. | qsl-analyze/tests/it/model_check_safety.rs:157; qsl-analyze/tests/it/model_check_safety.rs:189; spec/test-cases/TC-521-*.md |
| FND-003 | low | Nothing pins the "never proved" property of a TP-2 or TP-3 `Holds` carrying an empty closure certificate. Structurally it holds: `instances()` refuses a non-TP-1 translation, `check_closure` returns `ClosureRefusal::Item`, and `settle` returns `failed()`. But the PR's claim rests on code in another crate with no test in this diff. Fix: one settle test that a TP-3 `Holds` from `check_model` settles `Failed`, not `Proved`, to keep until QSL-878 replaces it. | qsl-analyze/src/model_check/check.rs:47-50; qsl-replay/src/model_check/settle.rs:174-205; qsl-replay/src/model_check/item.rs:80-125 |

bindings:

| test | TC / AC row | trace |
| --- | --- | --- |
| bounded_each_clauses_are_decided_on_the_product | TC-521 / FR-126-AC-2 | correct |
| a_long_interval_materializes_automaton_states_as_the_product_reaches_them | TC-521 / FR-126-AC-6 | correct |
| a_formula_violation_before_an_undefined_position_and_an_unread_position | TC-537 / FR-126-AC-9 | correct |
| an_empty_or_unbounded_subject_explores_nothing | TC-521 / FR-126-AC-6 | correct |
| a_terminal_state_closes_a_bounded_monitor_with_false_letters | TC-521 | wrong |
| a_terminal_state_stutters_under_infinite_trace | TC-521 | wrong |
