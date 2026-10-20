---
id: SR-12227
title: "Code review of quire-spec-language PR #1104: reduced/unreduced agreement and one max_transitions budget across bindings"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@f8702a5d971aa6e797ee8d9649b0a1f9ddae295c; git diff origin/main...HEAD: qsl-analyze/src/model_check/{concretise.rs, liveness.rs}, qsl-analyze/tests/it/quotient.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1104

## Summary

Ticket: QSL-541 (follow-up to #1066; fixes SR-12211 FND-007). One review pass
over the PR diff: `code-review` with its `rust-review` lane folded in.

Examined and sound:

- **One budget, charged once.** `Run` now holds `transitions_left` (what was
  left of `max_transitions` at the last charge) and `lifted` (reduced steps
  lifted since). Every lift starts with `concrete_budget() = transitions_left
  - lifted`, and every lift that becomes a counterexample passes through
  `steps_of`, which adds `Lift::charged()` to `lifted`. `charged()` equals
  the budget `Lift::edge` actually decremented: every reduced step, no
  concrete step. `into_steps` has no other caller in `liveness.rs`.
- **No double counting, no under-charging.** `charge_lifted` moves `lifted`
  into `Totals::charged_edges` with `Cell::take`, so each step is charged
  once. It runs after each exploration and at the start and end of every
  dead pass. So the first-phase prefix lifts are charged before the dead walk
  computes its budget. The dead walk's own lift is bounded by `*edges_left`
  (lifted is 0 there) and is charged after the walk's edges. Every binding
  loop iteration ends in `dead_phase`, so `lifted` is 0 when the next binding
  is explored. The second phase lifts every explored binding's lasso against
  the one remaining budget.
- **Measured.** A scratch probe (not committed) on the TC-586 fixture found
  `product_edges = 432`, shortest lasso 15. The smallest `max_transitions`
  that still returns `Violated` is 462 = 432 + 15 + 15: both lassos, each
  charged once, with nothing else charged.
- **The new test is not vacuous.** With only the `src/` changes reverted,
  `every_binding_concretises_within_the_one_max_transitions_budget` fails:
  the old per-binding budget returns `Violated` at `explored + 2*15 - 1`.
- **The agreement test can fail.** `every_reduced_verdict_agrees_with_the_unreduced_run`
  asserts the exact reduced outcome (`HoldsReduced` / `Violated`) and the
  exact unreduced outcome. `ComponentCertificateNotBuilt` is returned only
  at the end of `decide` when no binding has a lasso, no horizon cut and no
  reduction. A wrong reduced verdict, or an unreduced lasso, fails it.
- **No other behaviour change.** Concrete (trivial-group) lifts still spend
  nothing. The lifted steps join `charged_edges`, not `RunStats`. The only
  new effect is the intended one: lifted steps now reduce the budget of
  later dead passes and later explorations, as FR-161 "Length and limits"
  requires.
- Narrow tests run: `cargo test -p qsl-analyze --test it -- quotient::` at
  the reviewed sha, 10 passed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `dead_walk` overwrites `transitions_left` with `*edges_left` mid-walk, so the run's budget is briefly not the budget at the last charge. This is correct only because `dead_phase` charged `lifted` to 0 just before. Nothing asserts that invariant, and a later lift added before the walk would double-subtract or under-charge silently. Pass the walk's remaining edges to `dead_counterexample` as the lift budget, or `debug_assert_eq!(self.lifted.get(), 0)` at that point. | qsl-analyze/src/model_check/liveness.rs:1446-1448 |
| FND-002 | low | The TC-586 budget test pins only the stop side (`explored + 2*steps - 1` stops). An over-charging regression (a step charged twice) would also stop, so the test cannot catch double counting. The fixture's two lassos are both 15 steps, so `run_with(explored + 2 * steps)` returning `Violated` would pin exact charging. | qsl-analyze/tests/it/quotient.rs:466-479 |

## Verdict

Mergeable. No high findings. The shared `max_transitions` budget is charged
exactly once per reduced concrete step across bindings, the new test fails on
the old code, and the agreement test checks real verdicts. FND-001 and
FND-002 are low and do not block.

SR-12211 FND-007 is fixed by this PR (3245323dd). Its disposition row
belongs in `reviews/qsl-541-code-review.md`.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 44b0e3035: `debug_assert_eq!(self.lifted.get(), 0, "no lift pending at a dead walk")` just before `transitions_left` is set in `dead_walk`. |
| FND-002 | fixed | 44b0e3035: the budget test also asserts `run_with(explored + 2 * steps)` is `Violated` with the shortest lasso, which pins exact charging. |
