---
id: FR-187
title: "Give model transitions step probabilities and rewards"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-022
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-024
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: depends_on
---
# FR-187: Give model transitions step probabilities and rewards

## Description

FR-120's `ModelSystem` SHALL enumerate a random parameter over its support
and SHALL give, at each state, the probabilistic structure of its
successors: the **actions** (scheduled identities, ADR-024 PM-2) with each
action's draw distribution, and under a workload the exact step probability
of each transition (PM-3), together with each transition's reward values
(PM-5). A scheduled identity and drawn vector with no post-state, or with
several under a workload, `ModelSystem` SHALL report as `NotMarkov` (PM-3). Both
EN-4 (FR-189) reads this structure and does not compute it itself.

## Use case

The statistical engine samples one step of `Service` at a `Busy` state under
`Steady`: it needs the enabled operations with their weights, the drawn
values of `d` and `outcome` with their probabilities, the post-state, and
the `duration` reward of the step.

## Inputs

- A `ModelSystem` over a checked package with FR-185's declarations.
- A model state, and for step probabilities a workload.

## Outputs

```rust
pub struct ScheduledIdentity { pub operation: WireNodeId, pub receiver: ObjectKey, pub arguments: Vec<Value> } // non-random
pub struct Action {
    pub identity: ScheduledIdentity,
    pub draws: Vec<Draw>,                       // canonical order of random vectors
}
pub struct Draw {
    pub random: Vec<Value>,                     // one per random parameter, declared order
    pub probability: Rational,                  // product of the PM-1 probabilities
    pub successors: Vec<(TransitionId, ModelState)>, // FR-120 post-states, canonical order
    pub rewards: Vec<(Identifier, Value)>,
}
pub struct WeightedStep { pub transition: TransitionId, pub post: ModelState, pub probability: Rational, pub rewards: Vec<(Identifier, Value)> }
pub struct NotMarkov { pub state: StateKey, pub transition: ScheduledIdentity, pub random: Vec<Value>, pub post_states: u32 }

impl ModelSystem {
    pub fn actions(&self, state: &ModelState) -> Result<Vec<Action>, ExpansionStop>;
    pub fn weighted_steps(&self, state: &ModelState, workload: &Workload)
        -> Result<Result<Vec<WeightedStep>, NotMarkov>, ExpansionStop>;
}
```

## Behavior

- FR-120's enumeration SHALL range a random parameter over its support in
  declared order, so the support is its finite domain for enumeration, for
  FR-101's requires-bound pre-check and for the transition identity.
- `ModelSystem` SHALL carry a transition's random arguments in its
  transition identity and omit them from its scheduled identity.
- `actions` SHALL return, in canonical transition order, each scheduled
  identity whose effective precondition holds at the state and that has at
  least one post-state for some drawn vector. Each `Draw` SHALL carry the
  product of its values' PM-1 probabilities, so the draws of an action sum
  to exactly 1.
- `weighted_steps` SHALL give each enabled operation `o` probability
  `w(o) / Σ w(O(s))` over the enabled operations, each of its enabled
  scheduled identities `1 / |E_o(s)|`, and each draw its probability; a
  step's probability SHALL be the product, in exact rationals. Steps that
  reach the same post-state by the same transition identity are one step.
  The probabilities at a non-terminal state SHALL sum to exactly 1.
- When a scheduled identity and drawn vector give no post-state, or, under
  `weighted_steps`, more than one, `weighted_steps` SHALL return
  `NotMarkov` naming the state key, the scheduled identity, the drawn
  vector and the number of post-states. `actions` SHALL return the several
  post-states of a draw for the scheduler to choose among, and SHALL report
  a draw with none as `NotMarkov` through the caller.
- At a terminal state (FR-125) the only step SHALL be the terminal stutter
  step, with probability 1 and every reward 0.
- A step's reward values SHALL be its operation's rewards evaluated on the
  transition's arguments, random ones included; an operation without a
  reward of a name contributes 0 for it.
- When `ModelSystem::new` admits a model, it SHALL evaluate every reward on
  every argument vector of its operation's finite parameter domains and
  refuse admission with `AdmissionFailure::NegativeReward{operation, reward,
  arguments}` for the first negative value in canonical order.
- An undecided contract conjunction during expansion SHALL stop with
  FR-120's `ContractUndetermined`, as today.
- Weights play no part in the support graph: the transitions that
  `weighted_steps` gives positive probability SHALL be exactly FR-120's
  successors (ADR-024 PM-6).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-187-AC-1 | `Service` (ADR-024 §7.2) at the state `phase = Busy, attempts = 0` under `Steady`: `weighted_steps` returns six steps of `attempt`, with probabilities `441/500` (`1 ms`, `Ok`), `9/500` (`1 ms`, `Fail`), `343/5000`, `7/5000`, `147/5000`, `3/5000`, summing to 1; the `duration` reward is the drawn `d`. At `phase = Idle` the only step is `request` with probability 1 and `duration` 0. | Test (TC-622) |
| FR-187-AC-2 | The `Link` model (object `m: Msg { delivered: Bool, attempts: Int[0, 2] }`, initially undelivered with 0 attempts; operations `send_a(random lost: Bool ~ { true: 1, false: 9 })` and `send_b(random lost: Bool ~ { true: 1, false: 4 })`, each with precondition `not self.delivered and self.attempts < 2` and postcondition `self.attempts = pre(self.attempts) + 1 and self.delivered = not lost`; `terminal when self.delivered or self.attempts = 2`) under the workload `Even` (weight 1 on each operation) at its initial state: `send_a` with `lost = false` has probability `9/20`, `send_a` lost `1/20`, `send_b` delivered `2/5`, `send_b` lost `1/10`. `actions` at the same state returns two actions in canonical order, each with draws summing to 1. At a delivered state the only step is the stutter step with probability 1. | Test (TC-622) |
| FR-187-AC-3 | A variant of `Health` (ADR-024 §7.1) whose `tick` postcondition leaves a second Boolean field unconstrained: `weighted_steps` at the initial state returns `NotMarkov` naming the state key, `tick`, the drawn `fault` value and 2 post-states; `actions` returns one action whose draws each carry two successors. A variant whose postcondition is unsatisfiable for `fault = true` returns `NotMarkov` with 0 post-states from both. | Test (TC-622) |
| FR-187-AC-4 | A reward `refund = n - 3` on an operation with parameter `n: Int[0, 5]` refuses admission with `NegativeReward` naming `n = 0`. Over every state of `Service`, the set of transitions with positive probability under `Steady` equals FR-120's successor set. | Test (TC-622) |

## Dependencies

- ADR-024 PM-1 to PM-6; ADR-018 SM-3, SM-4.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) (enumeration,
  successor relation, admission), [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  (terminal stutter), [FR-185](FR-185-declare-random-parameters-workloads-and-rewards.md).

## References

- QSpec half, which owns the step probability, the Markov condition and the
  support graph in QSpec FR-181's successor relation: QSpec FR-406 (Linear
  STD-137).
- Owning ticket: Linear QSL-371.
