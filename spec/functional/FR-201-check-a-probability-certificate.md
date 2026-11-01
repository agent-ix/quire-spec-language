---
id: FR-201
title: "Check a probability certificate"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-023
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-200
    type: depends_on
---
# FR-201: Check a probability certificate

## Description

EN-5 SHALL emit every proof with a `ProbabilityCertificate` (ADR-028 CE-1).
QSpec FR-412 defines the certificate contract and the exact conditions its
checker verifies. The layer-6 facade entry
`qsl_replay::check_probability_certificate` SHALL apply that contract using
QSL's recompile, product enumeration and graph code, without an engine
present. It solves no equation and runs no iteration. The checker is part of
the qualified core (ADR-028 CE-7): an EN-5 `proved` SHALL count only after
this checker accepts its certificate.

## Use case

A reviewer receives a `proved` result for §15.4's claim at threshold 0.95.
The checker recompiles the model, rebuilds the product, checks that the
certificate's values `24/25` and `4/5` are the minimum over both actions at
each live state, and accepts. A certificate with one value altered is
rejected, and the item stays `inconclusive`.

## Inputs

```rust
pub struct ProbabilityCertificate {
    pub identity: ObligationIdentity,           // ADR-013 O-09
    pub objective: Objective,                   // targets or decided states, reward, min/max/workload, fairness set
    pub kind: CertificateKind,                  // Lower, Upper, Exact, LongRun
    pub values: Vec<(ProductKey, Rational)>,
    pub ranking: Option<Vec<(ProductKey, u64)>>,  // Lower and Exact
    pub policy: Option<Vec<(ProductKey, WitnessEntry)>>, // closed FR-202 choices/distributions
    pub components: Option<Vec<ComponentCertificate>>, // LongRun: value and gain–bias per component
}

pub fn check_probability_certificate(
    wire: ReplayRequestWire,                    // FR-098
    subject: &ModelTraceSubject,                // FR-128
    replay_limits: ReplayLimits,                 // FR-098 replay.input_bytes bound
    item: &ObligationIdentity,
    certificate: &ProbabilityCertificate,
    limits: &ExactProbLimits,
) -> Result<CertificateCheck, ReplayRefusal>;

pub enum CertificateCheck { Accepted { bound_side: Bound }, Rejected(CertificateRejection), Stopped(ExactProbLimit) }
```

The checker uses FR-338's `CertificateRejection{rule, at}`. FR-338's
`CertificateRule` gains `InequalityFails` and `ThresholdMissed`; the checker
also uses FR-169's `RankMissing` and `RankBroken` and FR-163's `Malformed`.
FR-338's `CertificateLocus` gains `ProbabilisticProductState(ProductKey)`,
the complete FR-196 product key; the checker also uses FR-163's
`CertificateMember`. QSpec FR-331 writes the first as its probabilistic
product state form `{product_key}` and the second as `{member}`.

| Failure | `rule` | `at` |
| --- | --- | --- |
| An `Upper`, `Lower`, `Exact`, expected-reward or `LongRun` condition fails at a state, a policy entry is missing, has the wrong mode or names a choice that is not enabled there, or a recurrent component under the policy lacks observation progress or fairness | `InequalityFails` | `ProbabilisticProductState` of the first failing state in canonical order; for a component, its first state |
| For a quantile claim with `y(s0) = 0`, the tight-action search reaches an activated state | `InequalityFails` | `ProbabilisticProductState` of the first reachable activated state |
| A non-target state with `x > 0` has no rank | `RankMissing` | `ProbabilisticProductState` of that state |
| A ranked non-target state with `x > 0` has no successor of positive probability, `x > 0` and a smaller rank | `RankBroken` | `ProbabilisticProductState` of that state |
| The conditions hold, but the initial state's value is not on the claim's side of the threshold | `ThresholdMissed` | `ProbabilisticProductState` of the initial state |
| A `Lower` or `Exact` certificate without `ranking`, a lower bound on a maximum without `policy`, or a `LongRun` certificate without `components` | `Malformed` | `CertificateMember` naming `ranking`, `policy` or `components` |

## Behavior

### Emission

- EN-5 SHALL build a certificate for every `proved` outcome, check it with
  this checker before emitting it, and on rejection raise precision or move
  to its exact method; it SHALL emit no proof without an accepted
  certificate.
- From an exact value it SHALL emit an `Exact` certificate; from a dyadic
  interval a `Lower` and an `Upper` certificate; from a long-run result a
  `LongRun` certificate with the gain–bias pairs of FR-199.

### Checking

- Digital timed certificate objectives SHALL select ADR-028 §13a from the
  checked subject/claim, independently of the authored fairness vector.
  The checker SHALL rederive the original digital continuation graph,
  observation edges, observation-admissible components and R, target-avoiding
  admissible components, and pretarget positive-reward waiting components
  with admissible exits. A finite upper maximum bound SHALL fail if a
  checked positive waiting component makes the supremum +infinity.
  Existing qualitative +infinity states need no fabricated finite Rational
  row; their proof is graph-recomputed, never an asserted delay-forever
  attaining scheduler. These graph checks use checker-owned algorithms and
  actual budgets; they are not equation-solving/iteration.
- Policy entries SHALL be FR-202's closed WitnessEntry. Randomized entries
  use distinct enabled choices and positive reduced exact probabilities
  summing to1. The checker SHALL apply their actual weighted one-step
  operator and check every reachable recurrent induced-chain component for
  observation progress and the actual fairness set. Goal absorption cannot
  bypass checking an admissible continuation on the original subject.
  Missing choice, wrong mode, or a scheduled-identity alias for IdleObserve
  is rejected rather than projected from action None.
- For exact-only timed Reach/elapsed, goals are observation edges and the
  operator uses terminal continuation1/0 respectively, V(post-key)
  otherwise; pure delay does not read the predicate. The initial observed
  goal terminates before reward. A public post-key shared by delay and
  observation does not become a synthetic goal-state key. For finite
  expected elapsed, the Lower/Exact properness condition is a positive
  probability lower-rank successor **or goal observation edge of terminal
  rank0**, even when the target value is0. Every reachable pretarget
  recurrent component under the evidence policy must have such an exit.
  Upper/Exact value and rank coverage for this edge-goal objective is every
  reachable as-yet-unhit state, with no value row required for a post-goal
  visit or internal sink; every non-goal successor still requires its actual
  row. The original post-goal continuation remains fully covered by the
  checked admissibility policy, not a missing-value/default action.
  This replaces the positive-value successor proviso below only for this
  expected-reward use; Reach retains its original proviso.

- The checker SHALL refuse a certificate whose identity differs from the
  item's.
- It SHALL decode `wire` through FR-098's canonical `ByteProvision` and
  recompile path, passing `replay_limits` to `ReplayRequest::decode` so the
  encoded request is bounded by `replay.input_bytes` before provision access;
  it SHALL re-admit `subject` and re-enumerate the product through
  `ModelSystem` and the monitor, using
  `qsl-eval`'s property-automaton translation and enabledness (ADR-018
  LA-5) and its own product enumeration and graph algorithms, and no
  `qsl-analyze` code. It SHALL NOT accept or reuse an EN-5 solver product as
  trusted replay data: every
  reachable product state for an `Upper`, `Exact` or `LongRun` certificate,
  and the states the certificate names and their successors for a `Lower`
  one, an unexplored successor counting as value 0.
- It SHALL recompute the decided states, `Prob0` and `Prob1`, and components
  with its own graph code. When the objective has a fairness set, it SHALL
  derive the maximal fair end components using FR-200's refinement before
  applying QSpec FR-412's certificate conditions.
- It SHALL apply QSpec FR-412's exact-rational conditions and threshold-side
  rule, and QSpec FR-331's rejection vocabulary. It SHALL accept when the
  conditions hold and the initial state's value is on the claim's side of the
  threshold, and report the side; otherwise it SHALL reject with the rule and
  locus the table above gives the first failing condition and state in
  canonical order. QSL's internal rule and locus variants map to the published
  forms as the table above specifies. It SHALL check the `Malformed` members
  before any re-enumeration.
- For a `quantile q of M >= c` claim over every scheduler, proved by an
  `Upper` certificate `y` for `max E[Y]`, `Y = [activated] · ([M < c] − q)`
  (ADR-028 XF-2, CE-4): when `y(s0) < 0` the checker SHALL accept with side
  `AtLeast`. When `y(s0) = 0`, it SHALL compute the tight actions, those
  with `Q_y(s, a) = y(s)` in exact rationals, and search the product graph
  from the initial state using tight actions only; it SHALL accept when no
  activated state is reachable, and otherwise reject, naming the first
  reachable activated state in canonical order.
- A checker run stopped by a budget SHALL return `Stopped` naming it.

### Verdict path

- QSL's settlement map in `qsl-replay` SHALL run the checker on the
  certificate that left S6c over E11 and settle `proved` only on `Accepted`; `Rejected` SHALL settle
  `inconclusive`, `CertificateRejected`; `Stopped` SHALL settle V-7.
- The checker's arithmetic and graph algorithms SHALL be code separate from
  EN-5's iterative and linear-algebra methods.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-201-AC-1 | §15.4 at threshold 0.95: the `Exact` certificate with values `24/25`, `4/5`, 1 at delivered states and 0 at the two-loss state is accepted with side `AtLeast`. The same certificate with `4/5` replaced by `9/10` is rejected, naming the first state in canonical order where the minimum over actions differs from the certified value; the item settles `inconclusive`, `CertificateRejected`. | Test (TC-636) |
| FR-201-AC-2 | §15.3's `LongRun` certificate (component value `1800/1801`, reward `[up] − 1800/1801`, gain 0, bias 0 at `up` and `−2000/1801` at `down`) is accepted; with the bias at `down` changed to `−2001/1801` it is rejected. §15.1's dyadic `Lower` certificate at 64 bits, ranked by remaining horizon, is accepted with side `AtLeast`. | Test (TC-636) |
| FR-201-AC-3 | A `Lower` certificate for §15.4 with its ranking omitted is rejected; one whose identity names a different claim is refused before any re-enumeration; §15.6's `FairTerminates` `Exact` certificate is accepted only after the checker recomputes that no fair end component exists, and the same certificate with its fairness set removed is rejected. With no authored fairness, FR-196's geometric timed policy is checked for derived TS-5 progress and its exact values5/4/3; rank3/2/1 has a positive-probability goal edge to rank0. Missing observation progress, an Observe-as-Delay policy, a finite maximum3 or a fabricated infinite-wait infinity witness is rejected. | Test (TC-636) |
| FR-201-AC-4 | A proof whose certificate the checker accepts settles `proved`; with a checker `max_states` of 2 the check stops and the item settles `Incomplete(LimitReached{…})` naming `max_states`. The same applies while rederiving R/waiting components or checking randomized observation evidence, with the actual consumed value/setting retained. | Test (TC-636) |
| FR-201-AC-5 | An `Opt` model whose initial state has the actions `go`, which activates and then sets `M` to 1 with probability `1/4` and to 3 with `3/4`, and `skip`, which never activates. For `quantile 0.5 of (M) >= 3` (`M` is a placeholder for the claim's parenthesised measure) over every scheduler, the `Upper` certificate with `y = 0` at the initial state and `−1/4` after `go` is accepted with side `AtLeast`: only `skip` is tight, and no activated state is reachable through it. The same claim over a variant whose `go` sets `M` to 1 and 3 with `1/2` each, with `y = 0` at both states, is rejected, naming the activated state after `go`, since `go` is tight. | Test (TC-636) |
| FR-201-AC-6 | Rejection rules and loci: AC-1's certificate with `4/5` replaced by `9/10` is rejected `InequalityFails` at the `ProbabilisticProductState` of that first state; AC-2's `LongRun` certificate with the changed bias, `InequalityFails` at the `down` state; AC-3's `Lower` certificate with its ranking omitted, `Malformed` at `CertificateMember("ranking")` before any re-enumeration; AC-2's `Lower` certificate with one named non-target state's rank removed, `RankMissing` at it, and with that rank raised to the rank of its only positive successor, `RankBroken` at it; AC-1's accepted certificate offered for a `>=` threshold above its initial state's value, `ThresholdMissed` at the initial state; AC-5's rejected variant, `InequalityFails` at the activated state after `go`. Each rejection is written with QSpec FR-331's rule spelling at its `{product_key}` or `{member}` form. | Test (TC-1197) |

## Dependencies

- ADR-028 SP-5, AR-4, CE-1 to CE-7 (CE-7 records the qualified-core
  placement), FS-9; ADR-013 O-09.
- [FR-098](FR-098-execute-a-replay-request.md),
  [FR-196](FR-196-build-the-probabilistic-product.md),
  [FR-200](FR-200-decide-every-scheduler-claims-over-fair-schedulers.md).

## References

- QSpec half, which owns the certificate wire and the checker's conditions:
  QSpec FR-412 (Linear STD-137).
- ADR-029 (owning record of the qualified core and RU-2) is on its own draft
  branch: Linear QSL-390.
- Owning ticket: Linear QSL-371.
