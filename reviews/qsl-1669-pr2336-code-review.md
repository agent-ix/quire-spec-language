---
id: SR-2336001
title: 'Rust review of PR #2336: finite-window profile refuses timed intervals, tick bounds held as steps, code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@b619cd3abf128955efad055e687af23e9246c482; git diff origin/main...HEAD (PR #2336, branch qsl-1669-finite-window-intervals); ticket QSL-1669, spec QSL-1611 (FR-326-AC-6, TC-836, FR-337-AC-24, FR-233-AC-2)'
review_set: subset
---

## Summary

Ticket: QSL-1669. PR: quire-spec-language#2336, head b619cd3ab. Method: rust-review (rust-style) and code-review, one pass, trial-merged on origin/main 19f33d8fb. The diff deletes the accepted-timed-interval path (checker Timestamped reading, `checked_interval` Timed arm, lowering tick branch, qsl-eval closed-integral arms) and adds the cause `TimedIntervalUnderStepCount` (`ill_typed`/`type-mismatch` at the interval). Every FR-326-AC-6 example behaves as the spec states. The change beyond the ticket narrows nothing: a timed interval under every step reading was already refused (`not-yet-implemented`), now with the FR-233-AC-2 code. The dense Timed profile is untouched.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-326-AC-6 | spec/functional/FR-326-admit-temporal-operators-by-the-unit-s-temporal-profile.md | examined |
| FR-233-AC-2 | spec/functional/FR-233-bind-a-model-claim-to-model-time-or-model-steps.md | examined |
| FR-337-AC-24 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| checked_operators | qsl-semantics/src/check/temporal_clause.rs:1037-1120 | examined |
| temporal_interval | qsl-semantics/src/check/lowering/temporal.rs:792-810 | examined |
| checked_interval | src/temporal.rs:1280-1310 | examined |
| Window::of, shape, operation_of | qsl-eval/src/temporal/plan.rs:60-95; qsl-eval/src/temporal/evidence/work.rs:180-210; qsl-eval/src/temporal/evidence/package.rs:425-460 | examined |
| hyper timed reading | qsl-semantics/src/check/hyper_clause.rs:1355-1361 | examined |
| TemporalCause::TimedIntervalUnderStepCount | qsl-semantics/src/check/refusal.rs:1686-1696 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A timed interval in a hyper clause under the timed profile (`IntervalReading::TimedUnread`) now refuses `ill_typed`/`type-mismatch` with a cause documented as "the clause counts steps or ticks". That clause counts time. FR-233-AC-2 names `model-steps` only, and FR-488-AC-18 refuses the whole clause `unsupported_construct`/`not-yet-implemented`, so the old `not-yet-implemented` cause was the true one. No feature is lost (the clause was refused before), but a valid form gets a wrong-cause refusal and no test covers the path. | qsl-semantics/src/check/temporal_clause.rs:1075-1081; qsl-semantics/src/check/hyper_clause.rs:1357 |
| FND-002 | medium | FR-337-AC-24 and TC-524 have no tagged test. The lowering unit test `a_tick_interval_is_a_step_interval_and_a_timed_one_is_a_fault` cites the AC in prose only, and nothing asserts that `eventually[0,5]` under the profile emits `temporal_interval {lower: "0", upper: "5"}` or that the emitter fault writes no node. Add `#[trace("FR-337-AC-24", "TC-524")]` and an emit-level assertion in qsl-package. | qsl-semantics/src/check/lowering/temporal.rs:839-868 |
| FND-003 | low | `TimedInterval::closed_integral_bounds` in qsl-foundation has no production caller after this PR (only its own unit test). Pre-release rule: delete the superseded code and its test. | qsl-foundation/src/bound.rs:537; qsl-foundation/src/bound.rs:631 |
| FND-004 | low | The restated reach test `a_timed_interval_has_no_tick_reach_and_a_finite_window_interval_is_one` dropped the old `future_reach` and `formula_nodes` assertions for a tick interval; the step-window half asserts only `history_span`. FR-523-AC-3 states `formula_nodes` 2 for `eventually[0,5]`. Assert (5, 5, 2) on the step window clause. The test name also promises more than its body shows. | qsl-eval/tests/it/temporal_rulings.rs:1283-1303 |
| FND-005 | low | The planner arm `Window::of` for `CheckedInterval::Timed` (`step-interval-has-offsets` fault) survives mutation: replacing it with `Closed {0, 0}` leaves qsl-eval (1126 it tests) green. Same for the `operation_of` Timed key arm in evidence/package.rs. Defensive arms for impossible states need one test each, or the arm should be removed by making the type unable to hold it. | qsl-eval/src/temporal/plan.rs:93; qsl-eval/src/temporal/evidence/package.rs:445 |

## Verdict

Mergeable after FND-001 and FND-002 are fixed in this PR (both small); FND-003 to FND-005 may ride along. No production `unwrap`, `expect`, panic or `#[allow]` was added; the new `expect`s are in test code.

Evidence, head trial-merged with origin/main 19f33d8fb:

- `cargo fmt --check`, `check --locked --workspace --all-targets`, `clippy -D warnings` (default and `--no-default-features`), arch-lint `qualified-core` and `api-surface --qsl-only`: pass.
- Tests pass: qsl-semantics lib with `--features test-support` (812 passed), qsl-semantics `it` (1359 passed), qsl-eval (190 lib, 1126 it, 43), qsl-replay lib (1075), qsl-package lib (665), root `it` (884), qsl-foundation lib.
- Failing tests that fail identically on origin/main 19f33d8fb: `the_type_and_lowering_matches_have_no_catch_all_arm`, `no_qsl_cst_type_is_re_exported_to_the_assembler`, `model_conformance_boundary::c06_two_selections_canonical_order`.
- Trial merges with PR #2309 (qsl-eval evidence package/work/plan, temporal_clause) and PR #2337: textually clean, `check --workspace --all-targets` passes, qsl-eval tests pass on both (#2337 also qsl-replay lib).

Mutation checks, each fails a test:

- Remove the refusal in `checked_operators`: `a_timed_interval_under_a_step_count_refuses_type_mismatch_at_the_interval` and `the_finite_window_profile_admits_only_integral_closed_tick_intervals` fail.
- Refusal located at the operator span instead of the interval: both fail.
- Restore a Timed arm in `src/temporal.rs` `checked_interval`: `a_timed_interval_is_an_interval_binding_refusal` fails.
- Lowering accepts a Timed interval under ticks: `a_tick_interval_is_a_step_interval_and_a_timed_one_is_a_fault` fails.
- `ClauseWork` accepts a closed integral Timed interval: `a_timed_interval_has_no_tick_reach_and_a_finite_window_interval_is_one` fails.
- Planner `Window::of` Timed arm accepted: survives (FND-005).
