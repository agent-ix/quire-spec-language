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
- FR-203's limits: `max_iterations`, `precision_bits`,
  `max_precision_bits`, `max_rational_bits`, `max_policy_iterations`.

## Outputs

```rust
pub enum UnboundedResult {
    Exact { values: Vec<ExtRational>, policy: Option<Vec<ActionChoice>> },   // UR-4
    Bounds { lower: Vec<Dyadic>, upper: Vec<Dyadic>, precision_bits: u32, sweeps: u64, policy: Option<Vec<ActionChoice>> },
    PrecisionBudget { lower: Dyadic, upper: Dyadic },
    Stopped(ExactProbLimit),
}
pub enum ExtRational { Finite(Rational), PlusInfinity }
```

## Behavior

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

- The witness policy SHALL be the final policy of policy iteration or, from
  interval iteration, the action attaining the bound at each state in the
  final upper iterate (maximum) or lower iterate (minimum), ties broken by
  canonical transition order.
- A dyadic interval from interval iteration SHALL yield a `Lower` and an
  `Upper` certificate candidate at once (FR-201).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-198-AC-1 | §15.6 `Terminates`: the live state is in `Prob0E`, so the minimum of `Pr(eventually holds(p.done))` is 0 with no iteration and the policy takes `wait`; the maximum is 1 (`Prob1E`). `probability >= 0.95 [eventually holds(m.delivered)]` over `Link` gives the minimum `24/25`. | Test (TC-633) |
| FR-198-AC-2 | `Link`'s `cost` claim (FR-195-AC-2): the target `delivered or attempts = 2` is reached with probability 1 under every scheduler; the minimum is `11/10` (`send_a`) and the maximum `6/5` (`send_b`), exact, and the `<= 6/5` bound is met at equality. `expected accumulate cost until holds(m.delivered) <= 3` has value `+∞` under every scheduler, decided by the graph alone. | Test (TC-633) |
| FR-198-AC-3 | A `Loop` model whose live state steps to itself, to `goal` and to `fail`, each with probability `1/3`, over every scheduler with one action, has `Pr(eventually holds(goal)) = 1/2`. Interval iteration's lower bound at the live state rises and its upper bound falls at each sweep, each bounding `1/2`; with `max_iterations` 3 and threshold `1/2`, policy iteration decides `1/2` at equality. Under a workload the same value comes from one exact solve. | Test (TC-633) |
| FR-198-AC-4 | With `max_policy_iterations` 0 and AC-3's equality threshold, the result is `Stopped(MaxPolicyIterations)`. Two runs of one request return equal results and equal policies. | Test (TC-633) |

## Dependencies

- ADR-028 XF-4, XF-5, UR-1 to UR-7, AR-2.
- [FR-196](FR-196-build-the-probabilistic-product.md),
  [FR-197](FR-197-decide-finite-horizon-forms-by-exact-backward-induction.md)
  (dyadic arithmetic and precision doubling).

## References

- QSpec half, which owns the semantics of the unbounded forms and the
  `+∞` rule: QSpec FR-411 (Linear STD-137).
- Owning ticket: Linear QSL-371.
