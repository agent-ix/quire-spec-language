---
id: SR-17048
title: "Code and Rust review of QSL-1047 FR-177-AC-8..12 clause-complete tests"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #1485 (branch qsl-1047-fr177, head 9ab89aeed); qsl-analyze/tests/it/hyper_witness.rs, qsl-analyze/tests/it/hyper_world.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-177
    type: references
---

## Summary

Ticket: QSL-1047. Review pass (round 0) at frozen head 9ab89aeed against the
origin/main merge base. Method: code-review with the rust-review lane folded
in. The diff is test-only: five new TC-602 tests in hyper_witness.rs, one per
FR-177-AC-8..12, and a `1 mod value(..)` arm in the hand-modelled match reader
in hyper_world.rs. No production file changes. There is no `#[allow]`,
`#[ignore]`, `unsafe` or `todo!`. The string-edge gate (FR-064) does not scan
`tests/`.

Gates, run by the reviewer on an exported copy of the head with its own target
dir: `cargo test -p qsl-analyze --test it hyper_witness` gives 38 passed. The
compile shows no warnings in the touched files. `rustfmt --check` is clean on
both files. `cargo clippy -p qsl-analyze --test it` could not reach the
touched files: it stops on unrelated baseline lints in qsl-forms
(`large_enum_variant`) and qsl-semantics. The touched files are not
clippy-checked by this review.

Clause by clause, against spec/functional/FR-177 lines 211-215:

- AC-8: asserts `Violated` with `WitnessExhausted{loop_entry: 0, position: 1}`
  under the late undefined `μ_U`. It asserts `Undecided(MatchUndetermined)`
  under AC-6's metered `μ_E` on the leaky world. It also asserts that the
  same world without the refusal empties `X` at position 1, which shows the
  "although" premise.
- AC-9: asserts AC-1's lasso as one `step(0)` with loop entry 0 and positions
  `(0,0),(0,0)`. For the `fair weak reset` lasso it asserts a stem of
  `step(0)`, a connecting model step to `l = 1`, a terminal stutter (nothing
  enabled), `position: 1` and loop entry 2.
- AC-10: asserts `Undefined` at position 1, `division-by-zero`, `a` taking
  `step(0)`, and `b` starting at `(1, 0)` with no step. It asserts two
  `variables`/`state_keys` entries, and with `c` added, the order `a, c, b`
  in `variables`, the traces and three `state_keys`.
- AC-11: asserts that the first instance's `Undefined` is returned at
  position 1. It asserts `v`'s empty `X` at position 0, `max_states` 7 →
  `Violated` and 6 → `Stopped`, and `w`'s position-0 `Undefined` beating `v`'s
  empty `X`.
- AC-12: asserts that `X_0` is empty (read from the explored graph) and the
  `NoPartnerPath` refusal (`Err`, no outcome). It asserts that the refusal
  ends the item ahead of the other instance's empty `X`.

Harness `mod` arm (hyper_world.rs:970-990): it reads `1 mod x` as 0 for
x = 1, `Undefined(DivisionByZero)` for x = 0, and the `else 0` branch (holds)
where the step has no input. The dividend is 1, so truncated `checked_rem`
equals Euclidean `mod` for every divisor, including a negative one. The arm
is correct for QSL's Euclidean `mod`. Its token pattern does not overlap the
`/` arm, and an unmodelled shape still panics, so it cannot hide a failure.

Mutations re-run by the reviewer, each on the exported copy and then
reverted:

- MA: the `NoPartnerPath` (Unstated) rank changed from 0 to 3, after an empty
  `X`. The AC-12 test fails.
- MB: instance offer `<` changed to `<=` (a later instance wins ties). The
  AC-11 test fails.
- MC: offers compared by rank before position. The AC-8 test fails.

An instrumented run of AC-8's `LateUndefined` seam shows `μ_U` evaluated 12
times per run, with the undefined reading returned on calls 6, 8, 10 and 12,
which are joint steps entering position 2. The premise holds at the head
(see FND-001). The coder's five reported mutations are consistent with these
results.

## Verdict

**APPROVED.** Each of the five tests asserts every clause of its criterion.
The coder's mutation checks are credible, and three independent reviewer
mutations each turn the matching new test red. The harness `mod` arm is
correct and cannot hide a failure. There are no tautologies and no production
changes. The four low findings can be fixed in this PR or deferred. None of
them blocks merge.

The AC-11 premise that both instances end at the undefined `μ_E` at
position 1 is not asserted for `w`, but `two_instances` builds `w` as an exact
copy of `v`, and mutation MB shows the tie-break is exercised. This is not
raised as a finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | AC-8's guard `late.calls.get() >= 5`, with the message "the undefined step was evaluated", does not show that the seam returned `Undefined`: call 5 is a `step(0)` (one=false). Instrumentation shows the undefined reading fires on calls 6/8/10/12 at this head, so the clause is exercised. But a change in evaluation order could leave the "does not replace" premise unmet while the test still passes. Count the `Undefined` returns in `LateUndefined` and assert at least one | qsl-analyze/tests/it/hyper_witness.rs:1444-1447, 509-536 |
| FND-002 | low | AC-12's second clause needs the other instance's empty `X` to be at the same position as the refusal. The test does not assert that `w`'s empty `X` is at position 1. If the fixture drifted so that `w` emptied later, the refusal would win by position, not by rank, and the test would still pass. Mutation MA shows the rank is what decides at this head. Assert the premise, as the AC-8 test does for its own (`WitnessExhausted{position: 1}` for the leaky world with the same conjunct) | qsl-analyze/tests/it/hyper_witness.rs:1704-1718 |
| FND-003 | low | Most of the new test bodies copy existing untagged TC-602 tests nearly line for line: `a_later_undefined_evaluation_does_not_replace_an_earlier_empty_witness_set`, `the_lasso_reaches_its_fair_cycle_by_a_connecting_path`, `an_undefined_universal_match_with_a_witness_ends_the_item_with_the_partners_path`, `a_later_instance_wins_only_with_a_strictly_earlier_finding`, `a_later_instance_with_an_undefined_evaluation_at_position_zero_beats_an_empty_witness_set`, `an_undefined_universal_match_with_no_witness_is_refused_not_guessed` and `an_unstated_undefined_evaluation_ranks_before_an_empty_witness_set_at_its_position`. Each fixture now lives in two places that can drift. Folding the old tests into the tagged ones (or removing them) keeps one copy | qsl-analyze/tests/it/hyper_witness.rs:1415-1719 |
| FND-004 | low | In the AC-10 and AC-11 tests, the `division-by-zero` cause comes from the test's own hand-modelled match reader (the new `mod` arm). No production `StepMatch` exists. The tests check how the engine ranks, places and passes on that reading, not how `1 mod 0` is evaluated. The AC-6 test's doc comment says it is "the engine half"; the AC-10 and AC-11 doc comments do not | qsl-analyze/tests/it/hyper_witness.rs:1530-1540, 1601-1612; qsl-analyze/tests/it/hyper_world.rs:970-990 |

## Dispositions

Round 1, checked at follow-up PR #1493 head 945493aba. #1485 merged before
the fix round, so the fixes are in that PR's commit b5393c87e (test and doc
comments only), plus 945493aba, which adds the review artifacts under
`reviews/`. Those two copies are byte-identical to the round-0 SR files.
Checks on an exported copy of the head with its own target dir:
`cargo test -p qsl-analyze --test it hyper_witness` gives 38 passed. With
`LateUndefined` changed so it never returns `Undefined`
(`if false && calls >= 5 && one`), the AC-8 test fails and only that test
fails. The new counter assert is real.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `LateUndefined` counts the `Undefined` readings it returns, and the AC-8 test asserts `late.undefined.get() >= 1` (hyper_witness.rs:1449-1452). A never-`Undefined` seam fails it |
| FND-002 | fixed | Before the `PerInstance` run, the AC-12 test runs the same two-instance leaky world with the default seams and asserts `WitnessExhausted{position: 1}` and `trace_position` 1 (hyper_witness.rs:1718-1730). The returned counterexample is the first instance's. `w` is an exact copy of `v` (`two_instances`), so this fixes `w`'s empty `X` at position 1 |
| FND-003 | deferred | Not changed in #1493. The PR says the existing untagged tests are not refactored there. Its claim that only AC-9's clause string is duplicated understates the overlap: the seven old tests named in the finding still repeat the new tagged tests' bodies. This is low, maintenance-only and out of scope for a test-hardening follow-up; no ticket is filed |
| FND-004 | fixed | The AC-10 and AC-11 doc comments now say that no production `StepMatch` exists and that the division-by-zero cause comes from the hand-modelled match reader in `hyper_world`. The tests check the engine's handling and ranking of that reading (hyper_witness.rs:1543-1546, 1619-1622) |
