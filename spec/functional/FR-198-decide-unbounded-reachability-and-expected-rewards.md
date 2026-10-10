---
id: FR-198
title: "Decide unbounded reachability and expected rewards"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-023
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-197
    type: depends_on
---
# FR-198: Decide unbounded reachability and expected rewards

## Description

EN-5 SHALL decide `Reach` and `ExpectedReward` forms (ADR-028 XF-4, XF-5) by
qualitative precomputation, maximal end-component collapse, interval
iteration over dyadic intervals with outward rounding, and, when the
interval still straddles the threshold, exact policy iteration with exact
sparse linear solves (UR-1 to UR-7). Plain value iteration with a
convergence threshold SHALL never decide a verdict.

## Use case

EN-5 decides `Terminates` over every scheduler. The graph step finds that
the scheduler can keep the coin unflipped forever, so the minimum is 0 with
no arithmetic, and the item is refuted by the scheduler that always waits.

## Inputs

- FR-196's product for a `Reach` or `ExpectedReward` claim, and the
  fairness set when the claim states one (FR-200 transforms the objective).
  Digital timed subjects additionally retain ADR-028 §13a's derived TS-5
  observation admissibility, even with an empty authored fairness set.
- FR-203's limits: `max_iterations`, `precision_bits`,
  `max_precision_bits`, `max_rational_bits`, `max_policy_iterations`.

## Outputs

```rust
pub enum UnboundedResult {
    Exact { values: Vec<ExtRational>, policy: Option<Vec<(ProductKey, WitnessEntry)>> },   // UR-4
    Bounds { lower: Vec<Dyadic>, upper: Vec<Dyadic>, precision_bits: u32, sweeps: u64, policy: Option<Vec<(ProductKey, WitnessEntry)>> },
    PrecisionBudget { lower: Dyadic, upper: Dyadic },
    Stopped(ExactProbLimit),
}
pub enum ExtRational { Finite(Rational), PlusInfinity }
```

## Behavior

### Derived timed observation admissibility

For FR-204 digital timed subjects, EN-5 SHALL apply ADR-028 §13a instead of
unrestricted memoryless extremum attainment. Results are infima/suprema over
almost-surely observation-admissible schedulers and the actual authored
fairness set, if any. Initial observation, resolved discrete steps and
IdleObserve check predicates; pure delays do not. Targets/failures are
observation edges; analysis absorption is not a public monitor/key. Recompute
the original digital continuation graph and its observation-admissible end
components before absorption. R is the almost-sure winning region for
reaching their union. Retain finite-prefix choices exactly when all
positive-probability successors stay in R. No waiting bound, clock-cap
inflation, fairness declaration or altered workload probability is added.

For Reach, maximize in R for the supremum; obtain the infimum by the
complement reachability of observation-admissible target-avoiding components,
with actual FR-200 fairness refinement where applicable. For elapsed-until,
minimize the target-reaching stochastic shortest path in R. For the
supremum, graph-recompute +infinity both for admitted positive-probability
target nonreachability and for any reachable pretarget positive-reward waiting
end component in R with an admissible exit. Its infinite wait may be
inadmissible; arbitrary finite waits followed by that exit make the
supremum unbounded. Otherwise collapse zero-reward waiting components and
use the finite exact methods below. Empty admissible continuations cannot
serve as a witness or be silently treated as an unrestricted extremum.

The objective's one-step operator uses the actual elapsed reward and a
terminal continuation0 on a goal observation edge, V(post-key) otherwise
(Reach uses terminal1). An equal post-key reached by pure delay is not a
goal. Initial B true terminates before reward. Recurrent admissibility is
checked on the original subject continuation, not an absorbed goal sink.

### Reachability

- EN-5 SHALL compute by graph algorithms alone `Prob0` and `Prob1` under a
  workload, and `Prob0A`, `Prob0E`, `Prob1A` and `Prob1E` over every
  scheduler, and fix the value 0 or 1 at those states.
- Over every scheduler, for a maximum it SHALL decompose the remaining
  states into maximal end components and collapse each one with no target
  state into one state that keeps the actions leaving it.
- It SHALL iterate the Bellman operator from 0 and from 1 at once over
  dyadic intervals at `precision_bits`, rounding outward, and stop when the
  interval at every initial product state lies on one side of the
  threshold, or at `max_iterations` sweeps.
- When the interval still contains the threshold at `max_iterations` or at
  `max_precision_bits`, it SHALL run policy iteration in exact rationals:
  evaluate a memoryless deterministic policy by fraction-free elimination
  (Bareiss), improve it action by action, and stop at a policy no action
  improves, at most `max_policy_iterations` times. Under a workload it SHALL
  run one exact linear solve.
- `holds(A) until holds(B)` SHALL fix 0 at states where neither holds and
  read `B` states as targets; `always holds(P)` SHALL be decided as
  `1 − Pr(eventually holds(not P))` with minimum and maximum exchanged.

### Expected rewards

- EN-5 SHALL give the value `+∞` at every state that reaches `B` with
  probability below 1 under some scheduler, for a maximum, or under every
  scheduler, for a minimum, decided by the graph alone; under a workload,
  at every state outside `Prob1`.
- On the remaining states it SHALL collapse end components of zero reward,
  obtain an upper starting vector as sound value iteration does, and run
  interval iteration with the reward added to the operator, then exact
  policy iteration as for reachability.
- A `<= c` bound SHALL fail and a `>= c` bound SHALL hold at an initial
  state whose value is `+∞`.

### Result

- For the derived TS-5 case, a finite threshold refutation SHALL use an
  actual admitted FR-202 WitnessEntry policy. Exact-rational randomized
  choices are permitted with no authored fairness set; independently check
  every reachable recurrent induced-chain component has observation edges
  and meets actual fairness. A nonattained supremum is not assigned an
  inadmissible attaining policy. A positive waiting component supplies
  finite threshold evidence by mixing its waiting choices with a checked
  admissible exit/progress policy, with positive rational epsilon; evaluate
  the chain exactly and decrease epsilon until the finite bound is crossed,
  or return the actual precision/rational/policy budget stop. No fixed
  epsilon or waiting cap is a semantic restriction. The unrestricted policy
  selection rules below apply outside this derived case.

- The witness policy SHALL be the final policy of policy iteration or, from
  interval iteration, the action attaining the bound at each state in the
  final upper iterate (maximum) or lower iterate (minimum), ties broken by
  canonical transition order.
- For a maximum, EN-5 SHALL choose the policy on UR-2's collapsed MDP and
  expand it on the product: at each collapsed MEC, the chosen leaving
  action at the MEC state that owns it, and at every other MEC state the
  first action, in canonical order, on a shortest path inside the MEC to
  that state (ADR-028 UR-7).
- A dyadic interval from interval iteration SHALL yield a `Lower` and an
  `Upper` certificate candidate at once (FR-201).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-198-AC-1 | §15.6 `Terminates`: the live state is in `Prob0E`, so the minimum of `Pr(eventually holds(p.done))` is 0 with no iteration and the policy takes `wait`; the maximum is 1 (`Prob1E`). `probability >= 0.95 [eventually holds(m.delivered)]` over `Link` gives the minimum `24/25`. | Test (TC-633) |
| FR-198-AC-2 | `Link`'s `cost` claim (FR-195-AC-2): the target `delivered or attempts = 2` is reached with probability 1 under every scheduler; the minimum is `11/10` (`send_a`) and the maximum `6/5` (`send_b`), exact, and the `<= 6/5` bound is met at equality. `expected accumulate cost until holds(m.delivered) <= 3` has value `+∞` under every scheduler, decided by the graph alone. For FR-196's no-fairness quiescent x>=1/cap2 elapsed oracle, infimum1 and supremum+infinity follow from the finite-wait component; the randomized 3/4-delay, 1/4-observe witness has exact initial value5>3 and observes almost surely. A claimed maximum3 and a delay-forever attaining witness fail. | Test (TC-633) |
| FR-198-AC-3 | A `Loop` model whose live state steps to itself, to `goal` and to `fail`, each with probability `1/3`, over every scheduler with one action, has `Pr(eventually holds(goal)) = 1/2`. Interval iteration's lower bound at the live state rises and its upper bound falls at each sweep, each bounding `1/2`; with `max_iterations` 3 and threshold `1/2`, policy iteration decides `1/2` at equality. Under a workload the same value comes from one exact solve. | Test (TC-633) |
| FR-198-AC-4 | With `max_policy_iterations` 0 and AC-3's equality threshold, the result is `Stopped(MaxPolicyIterations)`. Two runs of one request return equal results and equal policies. Exhausting the derived component or randomized-evidence construction budget returns the actual stop; it does not substitute a deterministic policy or observation bound. | Test (TC-633) |
| FR-198-AC-5 | A `Mec` model whose live state has the actions `a_stay`, back to itself with probability 1, and `b_exit`, to `goal` or `fail` with `1/2` each: over every scheduler the maximum of `Pr(eventually holds(goal))` is `1/2`, and the witness policy takes `b_exit` at the live state, though `a_stay` comes first in canonical order and attains the same iterate value. `probability <= 1/4` over the same event settles `refuted` on that policy, and its evidence replays. | Test (TC-633) |

## Dependencies

- ADR-028 XF-4, XF-5, UR-1 to UR-7, AR-2.
- [FR-196](FR-196-build-the-probabilistic-product.md),
  [FR-197](FR-197-decide-finite-horizon-forms-by-exact-backward-induction.md)
  (dyadic arithmetic and precision doubling).

## References

- QSpec half, which owns the semantics of the unbounded forms and the
  `+∞` rule: QSpec FR-411 (Linear STD-137).
- Owning ticket: Linear QSL-371.
