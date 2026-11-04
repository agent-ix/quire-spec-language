---
id: SR-20261104020301
title: "Code review — QSL-1609 restore the root it suite on main (PR #2241)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ada7a08c1 trial-merged on origin/main 37b4a8ffb; PR #2241 diff: src/temporal.rs, tests/support/front_end.rs, tests/it/{clause_kind_canonical,family_outcome_layering,request_builder,unbounded_declarations,value_parity_profile_selections}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-501
    type: reviews
---

## Summary

Ticket: QSL-1609. Code review with the rust-review lane folded in. The product change is `checked_interval` in `src/temporal.rs`: `CheckedInterval::Timed` was refused with `Binding{Interval}` since #2169 lowers timestamped-window step intervals to Timed tick intervals. A closed integer interval is now written as the wire tick interval (decimal of the numerator), anything else still refuses. Only the finite-window profile reaches this route with a Timed interval: `checked_profile` returns `None` for `InfiniteTrace` and `Timed` before `checked_operations` runs, and event-position and fixed-sample carry `Steps`. A negative lower bound and `lower > upper` cannot exist (`TimedInterval::new` refuses them), the checked clause has no unbounded Timed upper (`upper` is a `Rational`), and a bound beyond i64 is refused by the evaluator's `integer()` as `Binding{Interval}`, as it was for a Steps bound. The five test edits restate fixtures and allowlists to current contracts. I reverted `src/temporal.rs` to origin/main in the trial merge: `checked_repeated_results_retain_identity_inputs`, `checked_profiles_keep_distinct_closed_boundary_results`, `checked_open_incomplete_and_empty_window_outcomes_stay_distinct` and `checked_successor_authenticates_each_emitted_profile_and_clock_before_evaluation` fail (4 of 58), and all pass with the fix, so the fix is non-vacuous against the old behavior.

## Verdict

**MERGEABLE.** The diagnosis is correct and the Linear guesses were rightly corrected on three rows. No assertion is weakened and nothing is skipped or ignored. Two low findings.

## Scope

- `checked_interval` Timed arm: examined against FR-501 timestamped-window ticks, FR-043 (one tick is one admitted event or sample), and the lowering twin `tick_bounds` in `qsl-semantics/src/check/lowering/temporal.rs`.
- Other checked routes: unchanged (Steps arm and `None` arm untouched; `checked_profile` refuses Timed and InfiniteTrace first).
- `tests/support/front_end.rs` `emitted_with_default_clocks`: examined; only `request_builder::check` calls it, and `emitted_with_authority` and `emitted` keep `Clocks::Unselected`.
- `clause_kind_canonical` count 3: live list is `src/checking/bindings.rs`, `src/protocol_artifact/native/layout.rs`, `qsl-semantics/src/check/identity.rs`; the count equals the live list.
- `family_outcome_layering`: root `Cargo.toml` has `qsl-eval` under `[dev-dependencies]` (added by #2195), used by `tests/it/call_requirements_items.rs`; the test now forbids normal and build edges and requires the dev edge, as it does for `qsl-route`.
- `value_parity_profile_selections`: protocol `Flow` uses the declared protocol profile `p`; the test still asserts both lock rows (`protocol_profile`, `temporal_profile`), an empty `profile_selections` packet and replay.
- `unbounded_declarations`: matcher resolves via `WireGraph::of_package_value(..).key_of_value` and still asserts `matches.len() == 1` per vector key.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new refusal arms of `checked_interval` (open end, non-integer bound) have no test of their own, and the closed-integer predicate duplicates `tick_bounds` in the lowering, which is private to `qsl-semantics`. The arms are defensive today (the finite-window check admits only closed integral intervals), but nothing pins them and the two copies of the rule can drift. Fix: add a small unit test beside the function, or expose the lowering's predicate and call it. | src/temporal.rs:1299 |
| FND-002 | low | `default_clock_selections` panics for `FixedSampleFalseExtension` and supplies the arbitrary authority `"fixture"` for event-position. Any future `request_builder::check` fixture that uses a fixed-sample clause panics inside shared support instead of naming its own selection through a typed failure. Fix: return the selection for fixed-sample from a documented default or take the selections as a parameter. | tests/support/front_end.rs:224 |
