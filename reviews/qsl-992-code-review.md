---
id: SR-16635
title: "Code review of PR #1434: window aggregates stop at the first undefined position"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2f6f9e7be215f2ab4485391ed3c7d5229122ba09; qsl-eval/src/temporal/window.rs, qsl-eval/src/temporal/observed_window.rs, qsl-eval/src/temporal/mod.rs, qsl-eval/src/simulation/choice.rs, qsl-eval/tests/it/window_aggregates.rs, qsl-analyze/src/statistical/tests.rs, qsl-analyze/tests/it/exact_probabilistic.rs, qsl-analyze/tests/it/statistical_witness_replay.rs; ticket QSL-992"
review_set: subset
date: '2026-10-24'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-194
    type: reviews
---
# SR-16635: Code review of PR #1434, window aggregates stop at the first undefined position

## Summary

Ticket: QSL-992. Reviewed `git diff origin/main...HEAD` at 2f6f9e7be (merge base 0adfb1cee, which is current `origin/main`). The review includes the rust-review lane.

The production change replaces the `WindowSource` methods' `Result<T, Incomplete>` with `Result<WindowRead<T>, LetterStop>`. `AggregateAtom::evaluate` now returns `Undefined` at the first undefined filter, value or predicate, reading oldest first. The PR also adds `ObservedWindowSource`, which no production caller uses yet.

Checks run in the reviewer's own worktree and target directory:

- `cargo test` for `window_aggregates` (17 tests) and every touched qsl-analyze and choice test: all pass.
- `cargo clippy -p qsl-eval -p qsl-analyze --all-targets -D warnings`: clean.
- `cargo fmt --check`: clean.
- `cargo run -p xtask -- string-edge`: clean.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-194 "Evaluation" (oldest first, first undefined position, charging) | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md | examined |
| ADR-024 WA-4, WA-5 | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md | context_only |
| `WindowSource` / `WindowRead` / `AggregateAtom::evaluate` | qsl-eval/src/temporal/window.rs | examined |
| `ObservedWindowSource` | qsl-eval/src/temporal/observed_window.rs | examined |
| `ObservedLetters::atom` (pattern parity) | qsl-eval/src/temporal/observed.rs | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `ObservedWindowSource`, new public production code, is tested only on its defined and kernel-undefined paths. Three mutants survive the whole qsl-eval suite: a non-Boolean filter or predicate read as `Defined(false)` instead of the `window-condition-boolean` fault; an `Integer` value turned into a fault instead of being widened to `Rational`; and a refused evaluation that no longer records `stopped()`. No test covers `stopped()`, a `Failed` call, an `Incomplete` outcome, a missing value or predicate expression, `position-has-an-environment`, or `FamilyResult::Undefined`. | qsl-eval/src/temporal/observed_window.rs:78-177 |
| FND-002 | low | At the position where the admitted count reaches `max_window_values`, the limit check runs before the value is read. An undefined value at the (limit+1)-th admitted position therefore stops `Incomplete`, not `Undefined`. FR-194 orders an undefined position against a *later* overflow, but not against an overflow at the same position, and no test pins either order. A mutant that moves the limit check after `take` survives. | qsl-eval/src/temporal/window.rs:450-467 |
| FND-003 | low | `ObservedWindowSource::read` matches `FamilyOutcome` without the `#[deny(clippy::wildcard_enum_match_arm)]` guard that `ObservedLetters::atom` carries for the same match. A future variant could then be absorbed silently once someone adds a wildcard. The type is exported but no production caller uses it (S3 does not admit aggregate terms yet). | qsl-eval/src/temporal/observed_window.rs:78-118 |

## Verdict

CONDITIONAL. The core semantics are sound:

- **Visit order.** `trace.window` yields an ascending `lo..hi` range, so positions are visited oldest first.
- **Visit charge.** The `CollectionVisit` unit is charged before `admits`, so a stop at position k charges k−lo+1 visit units, as FR-194 "Evaluation" requires.
- **Undefined filter.** An undefined filter returns before the limit check and before the value read.
- **Undefined value or predicate.** These return from `take` before anything is folded in.
- **`min_count`.** It is checked only after the loop, so an undefined position wins over `InsufficientData`. A later defined violation is never reached.
- **`count`.** It reads no value expression.

The public API change to `WindowSource` breaks no caller:

- The only implementors are the test `Rows` and the new `ObservedWindowSource`.
- `qsl-eval` is `publish = false`.
- No other repository or worktree under /home/peter/dev uses `WindowSource` or `AggregateAtom`.

Production code has no new panic (the `unreachable!` sits under `cfg(seam_probe_eval_downstream)`, mirroring observed.rs), and every fault is a typed `InternalFault`.

Mutation re-run (seven mutants):

- **Killed (3):** M1, the observed kernel undefined turned into a fault; M5, an undefined filter skipped; M7, an undefined predicate counted as false.
- **Survived (4):** M2, M3 and M4 on `ObservedWindowSource` (FND-001), and M6, the limit check moved after the value read (FND-002).

CI: the `cla / cla-check` job reports fail while `cla / gate` passes; guard and language-boundary pass.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The fix for FND-002 (f17ed350a) reads the value at the position past the limit before checking `max_window_values`. An overflow therefore costs one extra value evaluation, and a `quantile` window briefly holds `max_window_values + 1` values before the stop. If that extra read exhausts the work budget, the run stops `Incomplete(LimitReached{work_units})` rather than FR-194's `LimitReached` naming `max_window_values`. No test pins either effect. Reading only the undefined-ness first, or checking the limit after `take` only for `Undefined`, would avoid both. | qsl-eval/src/temporal/window.rs:461-472 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | f17ed350a adds `the_observed_source_reads_numbers_and_faults_on_anything_else`, and mutants M2, M3 and M4 are now killed. Remaining gap: the `FamilyResult::Undefined` arm is still untested. It is a three-line mapping identical to `ObservedLetters::atom`, so it is acceptable without its own ticket; cover it when `ObservedWindowSource` is wired into the observed path. |
| FND-002 | fixed | `evaluate` now reads the value before the limit check, so an undefined value at the limit position is `Undefined`. This matches FR-194: an undefined position holds no value, and the evaluator stops at it. `an_undefined_position_at_the_limit_is_undefined_and_not_an_overflow` kills M6. QSL-1045 covers the spec wording. |
| FND-003 | fixed | `#[deny(clippy::wildcard_enum_match_arm)]` now guards `ObservedWindowSource::read`. |
