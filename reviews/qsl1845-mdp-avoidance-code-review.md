---
id: SR-20261109172912000901
title: "QSL-1845 MDP avoidance code-review"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-1845; PR 2645; unbounded qualitative target avoidance and regression tests"
relationships:
  - {target: ix://agent-ix/quire-spec-language/FR-198, type: references}
  - {target: ix://agent-ix/quire-spec-language/FR-196, type: references}
---
## Summary

Independent frozen review of f6573c58875adb8d77288e68e867829ed58d452b against actual base ca30727e2039a5b6d521e316f20f7bfc7b85b71f. Ticket: QSL-1845. No in-scope defect found. Code-review includes the Rust review lane.

## Verdict

PASS. Ready for merge after artifact custody and the lead's required gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Analysis

The preserved greatest non-target safe set admits a staying action, and includes decided non-target terminal rejection. Reverse existential closure through positive edges is precisely positive-probability target avoidance: a finite non-target path reaches a safe continuation with positive probability; conversely a finite induced chain with positive nonreach has a reachable non-target bottom class under an optimal deterministic memoryless policy. Off-path successors need not be safe. Target guards prevent crossing the target boundary. Graph::new removes zero mass and checks normalized actions. min_prob0 retains its universal-successor action condition and almost_sure_reach its nested attractor.

Distinct PostState keys 10/11 form genuine actions a=(self,goal,fail)/3 and b=(self/4,goal/2,fail/4). Their fixed-policy goal values are 1/2 and 2/3; Bellman min/max choose those extrema. Tests assert these exact values, equality fallback, policy/arithmetic stops, acyclic min2/3, target precedence, rejection terminals, zero edges, safe-loop min0/max1, and a real build_product pending-state scheduler choice. Reward tests follow FR-198's positive-nonreach infinity convention even at zero cost; a proper zero-cost action gives minimum0 while maximum stays infinite; geometric target leak with cost1 has expectation2. These are actual production API calls, not substituted solvers or forged checked source.

No new graph structure, API, dependency, wire/checker profile, admission rule, budget reset, gate narrowing, unsafe code, async lifecycle, or duplicated production rule is introduced. No applicable AssuranceProfile exists under spec/.

## Coverage

Plan completion: not assessed

Scoped manual FR-198-AC-1/2/3/4 and FR-196-AC-4 binding inspection; these regressions cover relevant clauses, not every scenario of each broad AC. Full computed matrix and repository-wide gap audit excluded by dispatch. Optional gap semantic fan-out was not requested and was not run; mathematical/code-test soundness inspection is part of the code review. No spec or plan diff: spec-review and subanalyses inapplicable.

## Verification and Limits

Accepted author logs: /tmp/qsl1845-red.log shows four intended failures and three controls passing on old code; /tmp/qsl1845-green.log shows all seven passing on restored code; /tmp/qsl1845-unbounded-unit.log shows 18 existing tests passing; fmt and scoped all-target/all-feature Clippy logs pass. No redundant Cargo, aggregate, Kani or full matrix run. Qualitative loops already lacked a limits argument; this review does not qualify full budget coverage or cyclic MDP certificates. No source, branch or GitHub edits performed.

## Reviewed Units

```yaml
clean: true
scope:
- id: FR-198-AC-1
  path: spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md
  role: examined
  excerpt: '§15.6 `Terminates`: the live state is in `Prob0E`, so the minimum of `Pr(eventually
    holds(p.done))` is 0 with no iteration and the policy takes `wait`; the maximum
    is 1 (`Prob1E`). `probability >= 0.95 [eventually holds(m.delivered)]` over `Link`
    gives the minimum `24/25`.'
- id: FR-198-AC-2
  path: spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md
  role: examined
  excerpt: '`Link`''s `cost` claim (FR-195-AC-2): the target `delivered or attempts
    = 2` is reached with probability 1 under every scheduler; the minimum is `11/10`
    (`send_a`) and the maximum `6/5` (`send_b`), exact, and the `<= 6/5` bound is
    met at equality. `expected accumulate cost until holds(m.delivered) <= 3` has
    value `+∞` under every scheduler, decided by the graph alone. For FR-196''s no-fairness
    quiescent x>=1/cap2 elapsed oracle, infimum1 and supremum+infinity follow from
    the finite-wait component; the randomized 3/4-delay, 1/4-observe witness has exact
    initial value5>3 and observes almost surely'
- id: FR-198-AC-3
  path: spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md
  role: examined
  excerpt: A `Loop` model whose live state steps to itself, to `goal` and to `fail`,
    each with probability `1/3`, over every scheduler with one action, has `Pr(eventually
    holds(goal)) = 1/2`. Interval iteration's lower bound at the live state rises
    and its upper bound falls at each sweep, each bounding `1/2`; with `max_iterations`
    3 and threshold `1/2`, policy iteration decides `1/2` at equality. Under a workload
    the same value comes from one exact solve.
- id: FR-198-AC-4
  path: spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md
  role: examined
  excerpt: With `max_policy_iterations` 0 and AC-3's equality threshold, the result
    is `Stopped(MaxPolicyIterations)`. Two runs of one request return equal results
    and equal policies. Exhausting the derived component or randomized-evidence construction
    budget returns the actual stop; it does not substitute a deterministic policy
    or observation bound.
- id: qsl-analyze/src/exact_probabilistic/unbounded.rs
  path: qsl-analyze/src/exact_probabilistic/unbounded.rs
  role: examined
  excerpt: A finite positive-probability path into the safe set suffices;
- id: qsl-analyze/tests/it/exact_mdp_avoidance.rs
  path: qsl-analyze/tests/it/exact_mdp_avoidance.rs
  role: examined
  excerpt: Exact qualitative avoidance on unrestricted finite MDP products.
- id: qsl-analyze/tests/it/main.rs
  path: qsl-analyze/tests/it/main.rs
  role: examined
  excerpt: mod exact_mdp_avoidance;
- id: FR-196-AC-4
  path: spec/functional/FR-196-build-the-probabilistic-product.md
  role: examined
  excerpt: FR-187-AC-3's `Health` variant with two post-states per draw builds, over
    every scheduler, an intermediate state per draw with two probability-1 actions,
    and the monitor does not advance on it; under a workload the build stops with
    `Unsupported(NotMarkov)`; the variant with no post-state stops with `Unsupported(NotMarkov)`
    under both. Two builds of one request are equal.
findings: []
bindings:
- test_id: two_actions_with_cyclic_positive_failure_keep_fractional_extrema
  ac_id: FR-198-AC-3
  trace: correct
- test_id: fractional_minimum_observes_policy_and_arithmetic_budgets
  ac_id: FR-198-AC-4
  trace: correct
- test_id: avoidance_controls_preserve_target_absorption_and_scheduler_choices
  ac_id: FR-198-AC-1
  trace: correct
- test_id: avoidance_controls_preserve_target_absorption_and_scheduler_choices
  ac_id: FR-198-AC-3
  trace: correct
- test_id: acyclic_failure_sibling_does_not_require_other_successors_to_be_targets
  ac_id: FR-198-AC-3
  trace: correct
- test_id: positive_nonreach_has_infinite_expected_until_even_with_zero_costs
  ac_id: FR-198-AC-2
  trace: correct
- test_id: target_leak_without_failure_keeps_expected_reward_finite
  ac_id: FR-198-AC-2
  trace: correct
- test_id: pending_post_state_resolutions_remain_scheduler_choices_after_the_draw
  ac_id: FR-196-AC-4
  trace: correct
- test_id: pending_post_state_resolutions_remain_scheduler_choices_after_the_draw
  ac_id: FR-198-AC-1
  trace: correct
```
