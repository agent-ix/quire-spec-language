---
id: SR-12807
title: "Code review of quire-spec-language PR #966: window aggregates over observed traces (FR-194 evaluation half)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3d62446e19b1e53a3a92e9433a452b8dbf076f26; PR #966: qsl-eval/src/temporal/window.rs, qsl-eval/src/temporal/letters.rs, qsl-eval/src/temporal/mod.rs, qsl-eval/tests/it/window_aggregates.rs"
review_set: subset
---
# Code review of quire-spec-language PR #966

## Summary

Ticket: QSL-572. Code review with the rust-review lane folded in. Focused
checks run on the head commit: `cargo test -p qsl-eval` (528 passed),
`cargo clippy -p qsl-eval --all-targets -D warnings` (clean) and
`cargo fmt --check` (clean).

The atom-level arithmetic is correct. Event-position windows read
`max(0, i-b)..=i-a` and are empty when `a > i`. Timestamped windows read only
positions `<= i` whose stamps lie in `[t_i - b, t_i - a]`, by binary search
over sorted stamps. Count, sum, min, max and fraction are exact. The
quantile is `ceil(q*n)`-th smallest. `min_count` gives
`Undefined(InsufficientData)`. The window limit stops on the
`max_window_values + 1`-th admitted value. Term refusals cover a reversed
window, a negative duration, `q` outside (0, 1) and `min_count` 0.

The defect is at the run seam. FR-194 says an oversized window makes the
**run** result `Incomplete(LimitReached{limit, value, setting})`. That stop
only exists as `AggregateStop::LimitReached` from `AggregateAtom::evaluate`.
`TemporalLetters::atom` can only return `Incomplete`, and `TemporalOutcome`
has no limit-reached outcome. So a monitor that wires the atom into the
temporal evaluator cannot report the stop FR-194 requires.

Author's open questions, judged:

- Work charged per position visited, where FR-194 says "per value visited":
  the code's reading is the sound one, because the filter is evaluated at
  every position in the window, so a position that is filtered out still
  costs work. FR-194 should say "per window position visited". The AC-3 test
  cannot tell the two readings apart (FND-004).
- Undefined value, filter or predicate is unspecified: this is a real gap,
  and a spec gap rather than only a code gap (FND-002).
- Adding the `InsufficientData` cause locally: correct and spec-mandated.
  FR-194 "Outputs" and its Behavior name
  `Undefined(InsufficientData{count, min_count})`. Adding it to
  `UndefinedCause` is the right place, and nothing else in the workspace
  matches on that enum exhaustively.

## Verdict

Changes requested: one high finding (the run-level `LimitReached` result
cannot be produced), one medium finding that needs a spec ruling, and lows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-194 requires that a window over `max_window_values` makes the run result `Incomplete(LimitReached{limit, value, setting})` (ADR-018 V-7). That stop exists only as `AggregateStop::LimitReached` on `AggregateAtom::evaluate`. The `TemporalLetters::atom` seam returns `Result<AtomReading, Incomplete>`, and `TemporalOutcome` has only `Incomplete(Incomplete)`, which is a `quire_exact` meter record with no limit, value or setting name. So a caller that wires an aggregate into the temporal evaluator must either panic (the test's `Derived` does `.expect`) or misreport the stop as a meter shortage. The same applies to `AggregateStop::Refused`. Failure scenario: a live monitor whose 300 s window fills with more than 2^20 responses has no way to return the required `LimitReached`. Fix: give the letters seam and `TemporalOutcome` a limit-reached stop, or a general stop type, and test it end to end with `evaluate`. | qsl-eval/src/temporal/window.rs:257-266; qsl-eval/src/temporal/letters.rs:45-58; qsl-eval/src/temporal/outcome.rs:48; qsl-eval/tests/it/window_aggregates.rs:471-474 |
| FND-002 | medium | `WindowSource::{admits, value, predicate}` return `Result<_, Incomplete>` only, so the value expression, the filter and the `fraction` predicate have no undefined outcome, for example a division by zero in `e.a / e.b`. A real source built on the clause evaluator must panic, drop the position, or invent an `Incomplete`. FR-194 does not say what an undefined value, filter or predicate does to the window: skip it, make the atom undefined, or something else. That needs a spec ruling (QSpec FR-414 / FR-194) before the seam is fixed. | qsl-eval/src/temporal/window.rs:289-322 |
| FND-003 | low | Weak limit-boundary oracle. A check that allows `max + 1` values (`count > max_window_values`) still passes both assertions: 20 values against a limit of 10 still stops, and exactly 20 against a limit of 20 is allowed. Add a case with `limit + 1` values. | qsl-eval/tests/it/window_aggregates.rs:297-321 |
| FND-004 | low | One work unit is charged per position visited, before the filter. FR-194 says "per value visited". Every row in the AC-3 test is admitted, so the test cannot tell the two readings apart. Fix FR-194's wording to "per window position visited", and add a filtered case that pins it. | qsl-eval/src/temporal/window.rs:445; qsl-eval/tests/it/window_aggregates.rs:316-320 |
| FND-005 | low | `quantile` sorts up to `max_window_values` (2^20) rationals in `finish` without charging the meter, so sort work is outside the TR-5 budget. The bound holds through the limit, but the work goes unmetered. | qsl-eval/src/temporal/window.rs:388 |
| FND-006 | low | No test covers an event-position window with `a > 0`, which is the `checked_sub(a)` path that gives an empty window when `a > i`. The code is correct by reading, but the path is unpinned. | qsl-eval/src/temporal/window.rs:212-217 |
| FND-007 | low | A test comment says two errors "falsify the atom at position 1", but the assertion is `Violated { at: 0 }`, the activation position. The comment misleads. | qsl-eval/tests/it/window_aggregates.rs:531-538 |

## New findings (disposition pass 2)

Found at `e52200e1bd4915a04840647411cd0809683b5736` by the STD-163 spec review and confirmed against FR-327 "Undefined evaluation" and ADR-024 WA-4. Reviewed at `7fd0574d07a81097fb2f26031a9a397143201fbd`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | high | `TemporalOutcome::category` mapped every `Undefined`, an insufficient-data window included, to `Category::Violation`. FR-327 says a window's `insufficient-data` settles the clause in the O-16 category `undefined`, "never `UndefinedEvaluation` and never violation", and ADR-024 WA-4 says the same. Failure scenario: a quiet monitor window reads as a violation of the claim. | qsl-eval/src/temporal/outcome.rs:58-65 |
| FND-009 | low | An insufficient-data clause is still returned as `TemporalOutcome::Undefined(UndefinedEvaluation{at, cause: InsufficientData})`. Its category is now right, but FR-327 says this outcome is "never `UndefinedEvaluation`". The Rust struct shares its name with the wire cause `undefined-evaluation` (qsl-replay/src/outcome.rs:243-247). A later mapper from `TemporalOutcome::Undefined` to that cause would emit `refuted`/`undefined-evaluation` for a quiet window. No consumer exists yet. The variant doc ("it does not hold on it") is also wrong for this cause. Fix: give insufficient data its own outcome variant (pre-release, no shim needed). | qsl-eval/src/temporal/outcome.rs:37-38; qsl-eval/src/temporal/outcome.rs:58-65 |

## Dispositions

Round 1, reviewed at `e52200e1bd4915a04840647411cd0809683b5736`. Focused
checks: `cargo test -p qsl-eval` (533 integration tests, 82 unit tests, all
pass), `cargo clippy -p qsl-eval --all-targets -D warnings` (clean) and
`cargo fmt --check` (clean). No new findings.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `TemporalLetters` now returns `LetterStop` (Incomplete / WindowLimit / Window / Fault). The engine maps `WindowLimit` to `TemporalOutcome::WindowLimitReached` (category Incomplete) and a window refusal to `TemporalRefusal::Window`. `a_window_over_the_limit_settles_the_run_with_the_limit_record` drives `evaluate` to that outcome. |
| FND-002 | deferred | STD-163. Planner ruling: an undefined value, filter or predicate at a position makes that atom Undefined, never skips it. The spec is to state this before the seam changes. |
| FND-003 | fixed | `a_window_one_value_over_the_limit_stops` reads 10 values under limit 10 and stops at 11, so an off-by-one that admits `max + 1` now fails. |
| FND-004 | fixed | `each_window_position_visited_charges_one_work_unit` charges 20 when a filter admits 10 of 20, which pins per-position charging. The FR-194 wording fix is tracked in STD-163. |
| FND-005 | accepted-no-change | Module docs now state that the quantile sort is bounded by `max_window_values` and is not charged separately. It stays a documented design choice. |
| FND-006 | fixed | `an_event_position_window_with_a_nearest_offset_skips_the_newest_positions` covers `past[2, 3]` at positions 5 and 1. |
| FND-007 | fixed | The comment now says the atom is falsified at position 1 and the clause is violated at its activation, position 0. |

Round 2, reviewed at `7fd0574d07a81097fb2f26031a9a397143201fbd`, which includes
the merge of origin/main and the `property_automata.rs` adaptation to
`LetterStop`. Focused checks: `cargo test -p qsl-eval` (546 integration
tests, 82 unit tests, all pass), clippy with `-D warnings` and `cargo fmt
--check` both clean. FND-001 to FND-007 keep their round-1 outcomes.

| FND | outcome | reason |
| --- | --- | --- |
| FND-008 | fixed | `category()` maps `Undefined` with cause `InsufficientData` to `Category::Undefined`, and other causes stay `Violation`. `an_atom_with_too_few_values_makes_the_clause_undefined` asserts `Category::Undefined` through `evaluate` and fails on revert. `a_kernel_undefined_atom_is_a_violation` guards against an over-broad mapping; it stays green on a plain revert. |
| FND-009 | deferred | Reviewer recommendation, not an owner ruling: fold the variant into the STD-163 code follow-up, which reworks this same outcome mapping. Behaviour and category are correct today, and nothing maps `TemporalOutcome` to wire causes yet. |
