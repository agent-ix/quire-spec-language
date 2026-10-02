---
id: FR-196
title: "Build the probabilistic product"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-023
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-187
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-195
    type: depends_on
---
# FR-196: Build the probabilistic product

## Description

QSL's layer-5 `exact_probabilistic` module SHALL build, for a probabilistic
claim, the finite product on which EN-5 computes (ADR-028 §2, SCH-2): the
MDP over FR-187's actions and draws, or under a workload the DTMC over
FR-187's step probabilities, in product with FR-126's deterministic monitor
for a bounded event and with an accumulator for a bounded measure. It SHALL
reuse `model_check`'s product `TransitionSystem`, monitor translations, edge
retention and SCC decomposition, and SHALL retain every product edge with
its action, draw and exact probability.

## Use case

EN-5 decides `P95` at `5 ms`. It builds the product of `Service` with the
event's monitor and a `duration` accumulator that saturates above `5 ms`,
every edge carrying its exact step probability, and finds every path
decided within 11 positions.

## Inputs

```rust
pub struct ExactProbRequest<'a> {
    pub subject: ModelSubject<'a>,                // FR-125
    pub claim: &'a CheckedProbabilisticClaim,     // FR-195
    pub limits: ExactProbLimits,                  // FR-203
}
```

## Outputs

```rust
pub struct ProbProduct {
    pub states: Vec<ProductKey>,                  // canonical discovery order
    pub kind: ProductKind,                        // Dtmc or Mdp
    pub edges: Vec<ProbEdge>,
    pub initial: Vec<(u32, Option<Binding>, ProductStateId)>,
    pub decided: Vec<(ProductStateId, Verdict)>,  // accepting or rejecting monitor states
}
pub struct ProductKey { pub model: StateKey, pub monitor: Option<u32>, pub accumulator: Option<Accumulator>, pub pending: Option<(ScheduledIdentity, Vec<Value>)> }
pub struct ProbEdge { pub from: ProductStateId, pub action: Option<ScheduledIdentity>, pub draw: Option<Vec<Value>>, pub to: ProductStateId, pub probability: Rational, pub rewards: Vec<(Identifier, Value)> }
```

## Behavior

- Under a workload, the product SHALL be a DTMC whose edges are FR-187's
  `weighted_steps`; a `NotMarkov` from FR-187 SHALL stop the build with
  `Unsupported(NotMarkov{…})`.
- Under every scheduler, the product SHALL be an MDP whose actions at a
  state are FR-187's `actions`. A draw with one post-state SHALL be an edge
  with the draw's probability, summed over draws that reach the same
  post-state. A draw with several post-states SHALL lead, with its
  probability, to an intermediate state keyed by (state, action, drawn
  vector), whose actions are its post-states, each with probability 1
  (SCH-2). A draw with no post-state SHALL stop the build with
  `Unsupported(NotMarkov{…})`.
- For a bounded event, the product SHALL pair each model state with the
  state of FR-126's deterministic monitor for the formula, read at model
  states only, never at intermediate states, with FR-126's closure at
  terminal model states. Accepting and rejecting monitor states SHALL be
  absorbing decided states.
- For a bounded measure, the product key SHALL add the activation flag and
  the accumulated reward or step count since activation, saturating at a single value above the threshold `c`, since only
  `M <= c` and `M < c` are read; for a weighted fraction, both sums,
  saturating where the comparison is decided.
- For a `Reach`, `ExpectedReward` or `LongRunFraction` form, EN-5 SHALL use
  the MDP or DTMC itself as the product, with the predicates as state labels and
  rewards on edges.
- If an atom, a state predicate, a measure or a reward the claim reads
  evaluates `Undefined` at a product state the build creates, then
  EN-5 SHALL stop the build at the first such state in canonical breadth-first
  order and return a refutation whose evidence is an `Undefined` path to
  it, with `UndefinedEvaluation{where, cause}` (ADR-028 XV-8, ADR-018
  UE-1).
- The initial product states SHALL be each subject initial state, per `over`
  binding, paired with the monitor's successor on position 0.
- Exploration SHALL use FR-101's canonical breadth-first order, so the
  product, its state numbering and its edge order are functions of the
  subject, the claim and the limits. Reaching `max_states`,
  `max_transitions` or `max_automaton_states` SHALL stop the build and name
  the limit (FR-203).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-196-AC-1 | §15.1's product (ADR-024 §7.1 `NoFault`) is a DTMC whose undecided states hold the healthy model state at one position each, at most 1,001 of them, plus one accepting and one rejecting decided state. Every undecided state's outgoing probabilities sum to 1, and every path reaches a decided state within 1,001 positions. | Test (TC-631) |
| FR-196-AC-2 | §15.2's product for `P95` at `5 ms` carries the `duration` accumulator, which takes the values `0`, `1 ms`, `2 ms`, `3 ms`, `4 ms` and the saturated value `6 ms`; the activation flag is set from position 1. The edge for `attempt` with `d = 3 ms`, `outcome = Ok` carries `343/5000`. | Test (TC-631) |
| FR-196-AC-3 | §15.4's `Deliver` over every scheduler is an MDP with two actions at each live state, whose draws carry `9/10` and `1/10` (`send_a`) and `4/5` and `1/5` (`send_b`); under `Even` it is a DTMC with FR-187-AC-2's probabilities. | Test (TC-631) |
| FR-196-AC-4 | FR-187-AC-3's `Health` variant with two post-states per draw builds, over every scheduler, an intermediate state per draw with two probability-1 actions, and the monitor does not advance on it; under a workload the build stops with `Unsupported(NotMarkov)`; the variant with no post-state stops with `Unsupported(NotMarkov)` under both. Two builds of one request are equal. | Test (TC-631) |
| FR-196-AC-5 | Over a `Coin` model (object `c` with `v: Int[0, 1]`, initially 0; operation `flip(random b: Int[0, 1] ~ {0: 1, 1: 1})` with postcondition `self.v = b`; workload `Even` with weight `flip` 1) and the claim `probability >= 1/2 [ always[0,3] holds(1 / (1 - c.v) = 1) ]` under `Even`, evidence `exact`, the build stops at the product state with `v = 1` at position 1 and returns a refutation whose evidence is the `Undefined` path `flip` with `b = 1`, probability `1/2`, with `UndefinedEvaluation{where: 1, cause: division-by-zero}`. | Test (TC-641) |

## Dependencies

- ADR-028 SP-2, PR-1 to PR-6, SCH-2; ADR-018 SM-3, SM-6, EN-1.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (product, monitors, edge retention),
  [FR-187](FR-187-give-model-transitions-step-probabilities-and-rewards.md),
  [FR-195](FR-195-check-exact-only-forms-and-route-exact-evidence.md).

## References

- QSpec half, which owns the probabilistic product's semantics: QSpec
  FR-406 and FR-411 (Linear STD-137).
- Owning ticket: Linear QSL-371.
