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
parameters (SP-4). CG's `probabilistic-satisfaction` arm SHALL route an
item naming `exact` evidence to EN-5 and SHALL settle an item naming
`statistical` evidence for an exact-only claim `unsupported` at negotiation
(XF-7). EN-5's provider manifest SHALL advertise
(`probabilistic-satisfaction`, `exact`).

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
    ExpectedReward { bound: Bound, reward: RewardRef, until: CheckedPredicate },
}
pub enum ReachTarget {
    Eventually(CheckedPredicate),
    Until(CheckedPredicate, CheckedPredicate),
    Always(CheckedPredicate),
}
pub enum RewardRef { Named(Identifier), Elapsed }      // Elapsed over a timed subject
// CheckedProbabilisticClaim gains: pub fairness: Vec<FairnessConstraint> (FR-123)
impl CheckedProbabilisticClaim { pub fn exact_only(&self) -> bool; }
```

## Behavior

### S3

- S3 SHALL admit `probability >= θ` or `<= θ` over `[eventually holds(P)]`,
  `[holds(A) until holds(B)]` or `[always holds(P)]` under the infinite-trace
  profile, with `P`, `A` and `B` state predicates and `0 < θ < 1`, as
  `ProbForm::Reach`. A temporal operator inside `P`, `A` or `B` SHALL be
  refused.
- S3 SHALL admit `expected accumulate R until holds(B) <= c` or `>= c`, with
  `R` a reward of the model and `c` of `R`'s type, and over a timed subject
  `expected elapsed until holds(B)` with `c` a duration, as
  `ProbForm::ExpectedReward`. Units follow FR-186.
- S3 SHALL admit a fairness set (FR-123's weak and strong constraints, with
  their granularity) on a claim `under every scheduler`, and record it in
  the claim and its obligation identity. A fairness set on a claim under a
  workload stays refused (FR-186).
- `exact_only` SHALL be true for a `Reach` or `ExpectedReward` form, for a
  claim `under every scheduler`, and for a claim that states no confidence
  parameters.

### Negotiation

- The `probabilistic-satisfaction` arm SHALL route an item naming `exact` to
  a candidate advertising (`probabilistic-satisfaction`, `exact`), and never
  to EN-4.
- An item naming `statistical` evidence for a claim with `exact_only` true
  SHALL settle `unsupported` at negotiation, with cause `ExactOnlyForm`,
  `EveryScheduler` or `MissingConfidence` (FR-192), and no engine runs.
- One claim requested with both evidence kinds SHALL give two items with two
  terminal records.
- The EN-5 provider manifest SHALL advertise (`probabilistic-satisfaction`,
  `exact`) and register through FR-075.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-195-AC-1 | §15.6's `Terminates` and `FairTerminates` check as `Reach{AtLeast(99/100), Eventually(p.done)}` under `Every`, the second with fairness `[strong flip]`; their obligation identities differ. `probability >= 0.95 [holds(not m.delivered) until holds(m.delivered)]` over `Link` checks as `Until`. `probability >= 0.9 [eventually eventually holds(p.done)]` is refused. | Test (TC-630) |
| FR-195-AC-2 | `expected accumulate cost until holds(m.delivered or m.attempts = 2) <= 6/5` over `Link` with a `cost` reward of 1 on both sends checks as `ExpectedReward`; §15.5's `MeanTime` checks with `RewardRef::Elapsed` and threshold `3 ms`; `expected elapsed until …` over an untimed subject is refused. | Test (TC-630) |
| FR-195-AC-3 | `exact_only` is true for each claim of AC-1 and AC-2 and for `P95` with no confidence parameters, and false for `P95` under `Steady` with its confidence parameters. | Test (TC-630) |
| FR-195-AC-4 | With EN-4 and EN-5 registered: `FairTerminates` naming `exact` routes to EN-5; naming `statistical` settles `unsupported`, `EveryScheduler`, with no engine run; `P95` with confidence parameters requested with both kinds gives two items, one routed to EN-4 and one to EN-5; the `Link` expected-reward claim under a workload naming `statistical` settles `unsupported`, `ExactOnlyForm`. | Test (TC-630) |

## Dependencies

- ADR-028 SP-3, SP-4, XF-4, XF-5, XF-7, XF-8, FS-1, RU-1, RU-2, RU-3;
  ADR-024 PF-10, SV-5, SV-9.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md),
  [FR-186](FR-186-check-probabilistic-claim-forms-at-s3.md),
  [FR-192](FR-192-settle-a-statistical-result-on-the-measured-axis.md).

## References

- QSpec half, which owns the exact-only forms, the fairness set on
  every-scheduler claims and the evidence kind `exact`: Linear STD-137.
- Owning ticket: Linear QSL-371.
