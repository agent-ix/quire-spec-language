---
id: SR-13299
title: "Gap analysis of QSL-530 E9 PR 2b against FR-126 (AC-1..AC-15), TC-521 and TC-1085 (PR #1045)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@4be524040; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md (AC-1..AC-15, Bindings and ranking, Model-only pass, First phase), spec/test-cases/TC-521-explicit-state-model-check.md, spec/test-cases/TC-1085-liveness-model-only-pass-and-unsupported-refusals.md, spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md (§6, context); qsl-analyze/src/model_check/liveness.rs, qsl-analyze/tests/it/example_units.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-521
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1085
    type: reviews
---
# SR-13299: Gap analysis of QSL-530 E9 PR 2b against FR-126 (PR #1045)

## Summary

Ticket: QSL-530 (E9, PR 2b). Tools: `quoin matrix --json` (quoin 0.28.3,
quire 0.36.2, engine 0.50.2), exit 0. Plan completion: not assessed.

Matrix state of FR-126's acceptance criteria at 4be524040:

| AC | Status | Tagged by |
| --- | --- | --- |
| AC-10 | tagged | `the_shortest_lasso_across_bindings_wins_not_the_first_binding` |
| AC-11 | tagged | `a_safety_claim_is_bound_to_the_binding_undefined_at_the_shallowest_depth` |
| AC-12 | tagged | `the_model_only_pass_finds_the_undefined_letter_a_dead_automaton_leaves` |
| AC-14 | tagged | `a_later_binding_wins_with_a_strictly_shallower_finding` |
| AC-15 | tagged | `a_later_binding_is_explored_through_one_less_than_the_finding` |
| AC-1, AC-3, AC-5 | tagged | earlier PRs |
| AC-2, AC-4, AC-6 to AC-9, AC-13 | untagged | outside this PR (AC-1's and AC-4's `Holds` rows wait on FR-339) |

The tags are truthful. Each test uses the `#[trace("TC-nnn", "FR-126-AC-n")]`
form TC-521 asks for. AC-10 and AC-11 sit under TC-521, and AC-12, AC-14 and
AC-15 under TC-1085, which matches the TCs' `Scope:` lines.

The AC-1 test now asserts the binding (`a`) and the looping receiver (`b`),
as amended AC-1 states.

Spec against code, measured on the real engine with scratch-clone probes
(details in SR-13298):

- **Tie-breaks.** Within a binding the product wins a depth tie with the
  model-only pass (`dead_depth < live_depth`). Across bindings, a later
  binding replaces the best finding only when strictly shallower, so a tie
  keeps the earlier binding. Lassos are compared with strict `<` on
  `len()`.
- **The d-1 bound and depth 0.** The product horizon is `min(max_depth,
  d - 1)` and the pass bound is `min(live, d)`. A finding at depth 0 breaks
  the loop: with `a` at 2, 1 product state, and `b` is not explored.
- **Stopped rule.** A later binding that stops returns its `Stopped`. A
  binding capped by `max_depth` counts as finished.
- **Global budget.** `Totals::remaining` subtracts the product totals and
  the passes' charges from one `max_states` / `max_transitions`. The pass
  skips roots at or past its bound, and successors at bound - 1 are not
  created. AC-12 shows `max_states` 1 stopping on the pass's second state.
- **AC-15's derived values, measured:**
  - `max_states` 6 returns `Stopped{MaxStates, 6}`, and the stop is in `b`:
    `a`'s pass charges exactly 6.
  - `max_states` 1,000 returns `b` at position 1, prefix `upd(b)`, 3
    product states.
  - Both versions 0 at 1,000 returns `a` at position 2, prefix `upd(a)
    upd(a)`, 0 product states.
- **ADR-018 §6.** The weak `each` run reports 30 product states (15 per
  instance), 78 edges and 2 automaton states.
- **Determinism.** Equal output on repeated runs.

The gap is in FR-126's stop rule under a constrained budget (SR-13298
FND-001 and FND-002), and no test exercises that path.

## Verdict

**PASS for AC-10, AC-11, AC-12, AC-14 and AC-15 as written, but FR-126
"Bindings and ranking" is not met under budget pressure.** SR-13298's two
medium findings carry the fix. This review adds one medium coverage
finding, the untested stop-rule branches, and one low finding on how weakly
the AC-15 test asserts.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-126's stop rule has branches no test exercises. (a) The binding that holds the finding stops, or its model-only pass stops, before the later bindings are ranked. This is the path where SR-13298 FND-001 returns a deeper counterexample: versions (0,1) or (0,2), `max_states` 4 to 8. (b) A later binding stops on `max_transitions`, on the evaluation meter or on cancellation. FR-126 names all four causes, but AC-15 drives only `max_states`. Add a test that holds a later-binding `Stopped` for at least one non-`max_states` cause. Add another where the finding binding's own exploration exhausts the budget and the run must not return that binding's deeper finding. | qsl-analyze/tests/it/example_units.rs:523-559; qsl-analyze/src/model_check/liveness.rs:751-774 |
| FND-002 | low | The AC-15 test asserts less than AC-15 states. The `max_states` 1,000 row checks only `bound == "b"`, not "the AC-14 finding" (prefix `(0,1) -upd(b)-> (0,2)`, position 1). The `max_states` 6 row cannot tell `b`'s stop from `a`'s, because `a`'s pass alone exhausts any budget from 1 to 5 with the same `Stopped` shape. A regression that charged `a` one more state would still pass. Assert the AC-14 prefix and position at 1,000. Pin the stop to `b`, for example by also asserting that `max_states` 9 returns `b`'s finding while 6 stops, or by checking the stopped run's stats. | qsl-analyze/tests/it/example_units.rs:528-553 |
