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
parameters (SP-4). EN-5's provider manifest SHALL advertise `probabilistic-satisfaction` with the
evidence kind `exact` (its mode is not yet fixed), and S3 SHALL mark each exact-only claim so EN-4 refuses it
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
- If an operand of the one operator is a temporal operator
  (`eventually eventually holds(P)`) or a constant (`eventually true`), S3
  SHALL refuse the claim [`not-state-predicate`] (QSL-proposed, STD-188, an
  `invalid_probabilistic_model` cause), located at the operand. Each operand
  is a `holds(P)` state predicate because EN-5 labels each model state by the
  truth of a checked state predicate, which a constant does not have.
- Under the infinite-trace profile, S3 SHALL refuse `unbounded-event`
  (FR-186) for every event that is not one of these three shapes.
- S3 SHALL admit `expected accumulate R until holds(B) <= c` or `>= c`, with
  `R` a reward of the model, as `ProbForm::ExpectedReward`.
- The threshold `c` SHALL be a literal of the expectation's type, which is not
  `R`'s type because the expectation of an integer reward is not an integer:
  for an integer reward a rational literal (`3`, `decimal(5, 1)`,
  `rational(6, 5)` or `0.5`), recorded exactly; for a quantity reward a
  quantity literal of `R`'s dimension, converted exactly into `R`'s unit
  (FR-186, QSpec FR-142).
- If `c` is not a literal, or is a bare number for a quantity reward, S3
  SHALL refuse the claim `ill_typed`/`type-mismatch`.
- If `c` is a quantity for an integer reward, or a quantity of another
  dimension than `R`'s, S3 SHALL refuse the claim `ill_typed`/`unit-mismatch`,
  naming the dimension.
- S3 SHALL apply no sign or range check to `c`: `0` and a negative rational
  (`rational(-1, 2)`) are admitted, because an expectation is not a
  probability.
- If `R` names no reward of the subject's model, S3 SHALL refuse the claim
  `missing_declaration`/`missing-name`, located at `R`.
- If a claim is `expected elapsed until holds(B)`, S3 SHALL refuse it
  `unsupported_construct`/`not-yet-implemented`, because no subject is timed
  at S3. FR-204 owns the timed form, with `c` a duration.

The claim's profile prohibits `/` and the unary minus, so a quotient is
written `rational(6, 5)` and a negative quantity cannot be written.
- S3 SHALL admit a fairness set (FR-123's weak and strong constraints, with
  their granularity) on a claim `under every scheduler`, and record it in
  the claim and its obligation identity. A fairness set on a claim under a
  workload stays refused (FR-186).
- `exact_only` SHALL be true for a `Reach` or `ExpectedReward` form, for a
  claim `under every scheduler`, and for a claim that states no confidence
  parameters.
- S3 SHALL write one requirement record for each checked probabilistic
  claim (FR-186), as FR-165 does for a state-graph claim: keyed by the
  claim node's `claim` occurrence key (ADR-013 O-07), with
  `Requirements{kind: probabilistic-satisfaction, extent:
  Unbounded{domains}}`, with `domains` naming the claim as FR-165 names a
  state-graph claim's (FR-057, ADR-014 A-3), its result bound the
  `Boolean` type node (FR-092), no direct conversion context and an empty
  path condition. No other capability kind SHALL be written for it.
- S3 SHALL record on each checked probabilistic claim's
  `probabilistic-satisfaction` requirement record its evidence kind:
  `exact` when `exact_only` is true, and `statistical` otherwise. It is the
  evidence kind FR-075's request builder writes when the request names
  none, and it enters neither the claim's normal form nor its obligation
  identity.

### Capability

- The EN-5 provider manifest SHALL advertise `probabilistic-satisfaction` with
  the evidence kind `exact` only (its mode is not yet fixed), and register through FR-075.
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
| FR-195-AC-2 | `expected accumulate cost until holds(m.delivered or m.attempts = 2) <= rational(6, 5)` over `Link` with a `cost` reward of 1 on both sends checks as `ExpectedReward` with threshold `6/5`; written `<= 6/5` it is refused, since the profile prohibits `/`. With an integer reward, `>= 3` checks with threshold 3, `<= 0` and `<= rational(-1, 2)` check, and `<= 5 ms` is refused `ill_typed`/`unit-mismatch`. With a quantity reward declared in `ms`, `<= 0.005 s` checks and records `5`; `<= 5 m` is refused `ill_typed`/`unit-mismatch`, and the bare `<= 3` is refused `ill_typed`/`type-mismatch`. A reward the model does not declare is refused `missing-name`. §15.5's `MeanTime` and any other `expected elapsed until …` are refused `unsupported_construct`/`not-yet-implemented`. | Test (TC-630) |
| FR-195-AC-3 | `exact_only` is true for each claim of AC-1 and AC-2 that checks and for `P95` with no confidence parameters, and false for `P95` under `Steady` with its confidence parameters. | Test (TC-630) |
| FR-195-AC-4 | EN-4's gate given the checked `FairTerminates` refuses `EveryScheduler`; given the `Link` expected-reward claim under a workload it refuses `ExactOnlyForm`; given a bounded event under a workload with no confidence parameters it refuses `MissingConfidence`; given `P95` under `Steady` with its confidence parameters it passes. Each refusal is returned before a `StatisticalClaim` is built and so before a sample is drawn, and for every claim of AC-1 to AC-3 that checks the gate refuses exactly when `exact_only` is true. | Test (TC-630) |
| FR-195-AC-5 | EN-5's manifest lists `probabilistic-satisfaction` with the evidence kind `exact` only. Each of the three refusals of AC-4 settles `unsupported` with its cause (`every-scheduler`, `exact-only-form`, `missing-confidence`) through FR-192's map. | Test (TC-630) |
| FR-195-AC-6 | The use case's `FairTerminates` (`under every scheduler`, no confidence parameters) has one requirement record, keyed by its claim node's `claim` occurrence, with kind `probabilistic-satisfaction`, extent `Unbounded` with `domains` naming `FairTerminates`, the `Boolean` result bound, no conversion context, an empty path condition and evidence kind `exact`; a `Reach` claim under a workload with confidence parameters records `exact`; FR-186's `P95` (a quantile form under `Steady` with `alpha`, `beta` and `indifference`) records `statistical`. `P95`'s obligation identity is equal across a request naming no evidence selection, one naming `exact` and one naming both kinds (QSpec FR-407-AC-6). | Test (TC-1163) |

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
