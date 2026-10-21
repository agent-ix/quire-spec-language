---
id: FR-195
title: "Check exact-only forms and route exact evidence"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-023
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-024
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-192
    type: depends_on
---
# FR-195: Check exact-only forms and route exact evidence

## Description

S3 SHALL extend FR-186's probabilistic claim check with the forms that only
exact evidence decides (ADR-028 XF-4, XF-5), with fairness sets on claims
`under every scheduler` (FS-1), and with claims that state no confidence
parameters (SP-4). EN-5's provider manifest SHALL advertise (`probabilistic-satisfaction`,
`exact`), and S3 SHALL mark each exact-only claim so EN-4 refuses it
(ADR-024 SV-5, XF-7). Routing by evidence kind is QSpec FR-290's
negotiation rule, which QSL does not restate.

## Use case

A modeller states `probabilistic FairTerminates … under every scheduler fair
strong Proc::flip { probability >= 0.99 [eventually holds(p.done)] }` with
no confidence parameters. S3 records an unbounded reachability claim over
fair schedulers. Requested with exact evidence it goes to EN-5; requested
with statistical evidence it settles `unsupported` with no engine run.

## Inputs

- A parsed `probabilistic` claim and the checked model (FR-185, FR-186).
- A request item for a `probabilistic-satisfaction` claim with its evidence
  kind.

## Outputs

FR-186's `ProbForm` gains:

```rust
pub enum ProbForm {
    // FR-186's forms, and:
    Reach { bound: Bound, target: ReachTarget },
    ExpectedReward { bound: Bound, reward: Identifier, until: CheckedPredicate },
}
pub enum ReachTarget {
    Eventually(CheckedPredicate),
    Until(CheckedPredicate, CheckedPredicate),
    Always(CheckedPredicate),
}
// CheckedProbabilisticClaim gains: pub fairness: Vec<FairnessConstraint> (FR-123)
impl CheckedProbabilisticClaim { pub fn exact_only(&self) -> bool; }
```

`ExpectedReward` names its reward by the model's reward `Identifier`. The
`expected elapsed` quantity names the elapsed time of a timed subject and is
FR-204's: it gives this form a `RewardRef` with an `Elapsed` member. No
subject is timed at S3, so S3 refuses `expected elapsed` as
`unsupported_construct`/`not-yet-implemented`.

## Behavior

### S3

- S3 SHALL admit `probability >= θ` or `<= θ` over `[eventually holds(P)]`,
  `[holds(A) until holds(B)]` or `[always holds(P)]` under the infinite-trace
  profile, with `P`, `A` and `B` state predicates and `0 < θ < 1`, as
  `ProbForm::Reach`.
- Each operand of the one operator SHALL be a `holds(P)` state predicate. If
  an operand is a temporal operator (`eventually eventually holds(P)`) or a
  constant (`eventually true`), S3 SHALL refuse the claim
  [`not-state-predicate`] (QSL-proposed, STD-188, an
  `invalid_probabilistic_model` cause), located at the operand. A constant is
  refused as well as an operator because EN-5 labels each model state by the
  truth of a checked state predicate, and a constant has none. Under the
  infinite-trace profile, every event that is not one of these three shapes
  is refused `unbounded-event` (FR-186).
- S3 SHALL admit `expected accumulate R until holds(B) <= c` or `>= c`, with
  `R` a reward of the model, as `ProbForm::ExpectedReward`. The expectation
  of an integer reward is not an integer, so `c` is not of `R`'s type but of
  the expectation's:
  - for an integer reward, a rational literal: `3`, `decimal(5, 1)`,
    `rational(6, 5)` or `0.5`, recorded exactly;
  - for a quantity reward, a quantity literal of `R`'s dimension, converted
    exactly into `R`'s unit (FR-186, QSpec FR-142);
  - `c` SHALL be a literal. A quotient such as `6/5` is not available,
    because the claim's profile prohibits `/`; it is written
    `rational(6, 5)`. A threshold that is no literal is refused
    `ill_typed`/`type-mismatch`, as is a bare number for a quantity reward. A
    quantity for an integer reward, and a quantity of another dimension than
    `R`'s, are refused `ill_typed`/`unit-mismatch`, naming the dimension;
  - S3 SHALL apply no sign or range check to `c`: an expectation is not a
    probability, so `0` and a negative rational (`rational(-1, 2)`) are
    admitted. A negative quantity cannot be written as a literal, because
    the unary minus is not admitted in the claim's profile;
  - if `R` names no reward of the subject's model, S3 SHALL refuse the claim
    `missing_declaration`/`missing-name`, located at `R`.
- Over a timed subject, S3 SHALL admit `expected elapsed until holds(B)`
  with `c` a duration (FR-204).
- S3 SHALL admit a fairness set (FR-123's weak and strong constraints, with
  their granularity) on a claim `under every scheduler`, and record it in
  the claim and its obligation identity. A fairness set on a claim under a
  workload stays refused (FR-186).
- `exact_only` SHALL be true for a `Reach` or `ExpectedReward` form, for a
  claim `under every scheduler`, and for a claim that states no confidence
  parameters.

### Capability

- The EN-5 provider manifest SHALL advertise (`probabilistic-satisfaction`,
  `exact`) only, and register through FR-075.
- EN-4's negotiation gate takes the checked claim, because the reduced claim
  `check_statistical` takes carries neither the scheduler nor the form. When
  `exact_only` is true for the checked claim, the gate SHALL refuse it before
  that reduced claim is built and before a sample is drawn, with the first
  cause that applies: `EveryScheduler` for a claim `under every scheduler`,
  then `ExactOnlyForm` for a `Reach` or `ExpectedReward` form, then
  `MissingConfidence` (FR-189). A claim whose `exact_only` is false passes
  the gate.
- The settlement map SHALL settle each of those refusals `unsupported` with
  its cause (FR-192).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-195-AC-1 | §15.6's `Terminates` and `FairTerminates` check as `Reach{AtLeast(99/100), Eventually(p.done)}` under `Every`, the second with fairness `[strong flip]`; their obligation identities differ. `probability >= 0.95 [holds(not m.delivered) until holds(m.delivered)]` over `Link` checks as `Until`. `probability >= 0.9 [eventually eventually holds(p.done)]` is refused `not-state-predicate` located at the inner `eventually`, as are `[holds(p.done) until eventually holds(p.done)]` (at the second `eventually`), `[always not holds(p.done)]` (at `not holds(p.done)`) and `[eventually true]` (at `true`). `[eventually[0, 5] holds(p.done)]` under the infinite-trace profile is refused `unbounded-event`, and so is `[eventually holds(p.done)]` under a bounded profile. | Test (TC-630) |
| FR-195-AC-2 | `expected accumulate cost until holds(m.delivered or m.attempts = 2) <= rational(6, 5)` over `Link` with a `cost` reward of 1 on both sends checks as `ExpectedReward` with threshold `6/5`; written `<= 6/5` it is refused, since the profile prohibits `/`. With an integer reward, `>= 3` checks with threshold 3, `<= 0` and `<= rational(-1, 2)` check, and `<= 5 ms` is refused `ill_typed`/`unit-mismatch`. With a quantity reward declared in `ms`, `<= 0.005 s` checks and records `5`; `<= 5 m` is refused `ill_typed`/`unit-mismatch`, and the bare `<= 3` is refused `ill_typed`/`type-mismatch`. A reward the model does not declare is refused `missing-name`. §15.5's `MeanTime` checks with the elapsed quantity and threshold `3 ms`; `expected elapsed until …` over a subject that is not timed is refused. | Test (TC-630) |
| FR-195-AC-3 | `exact_only` is true for each claim of AC-1 and AC-2 that checks and for `P95` with no confidence parameters, and false for `P95` under `Steady` with its confidence parameters. | Test (TC-630) |
| FR-195-AC-4 | EN-4's gate given the checked `FairTerminates` refuses `EveryScheduler`; given the `Link` expected-reward claim under a workload it refuses `ExactOnlyForm`; given a bounded event under a workload with no confidence parameters it refuses `MissingConfidence`; given `P95` under `Steady` with its confidence parameters it passes. Each refusal is returned before a `StatisticalClaim` is built and so before a sample is drawn, and for every claim of AC-1 to AC-3 that checks the gate refuses exactly when `exact_only` is true. | Test (TC-630) |
| FR-195-AC-5 | EN-5's manifest lists exactly (`probabilistic-satisfaction`, `exact`). Each of the three refusals of AC-4 settles `unsupported` with its cause (`every-scheduler`, `exact-only-form`, `missing-confidence`) through FR-192's map. | Test (TC-630) |

## Dependencies

- ADR-028 SP-3, SP-4, XF-4, XF-5, XF-7, XF-8, FS-1, RU-1, RU-2, RU-3;
  ADR-024 PF-10, SV-5, SV-9.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md),
  [FR-186](FR-186-check-probabilistic-claim-forms-at-s3.md),
  [FR-192](FR-192-settle-a-statistical-result-on-the-measured-axis.md).

## References

- QSpec half, which owns the exact-only forms, the fairness set on
  every-scheduler claims and the evidence kind `exact`: QSpec FR-407,
  FR-411 and FR-290 (Linear STD-137).
- Owning ticket: Linear QSL-371.
