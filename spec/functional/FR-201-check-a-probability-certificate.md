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
The layer-6 facade entry `qsl_replay::check_probability_certificate` SHALL
verify the certificate in exact rationals with no engine present: it recompiles the
package, re-enumerates the product, recomputes every qualitative set by
graph algorithms, and checks the certificate's fixed-point, ranking and
gain–bias conditions (CE-2 to CE-4). It solves no equation and runs no
iteration. The checker is part of the qualified core (ADR-028 CE-7): an
EN-5 `proved` SHALL count only after this checker accepts its certificate.

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
    request: &ReplayRequest,                    // FR-098
    item: &ObligationIdentity,
    certificate: &ProbabilityCertificate,
    limits: &ExactProbLimits,
) -> Result<CertificateCheck, ReplayRefusal>;

pub enum CertificateCheck { Accepted { bound_side: Bound }, Rejected(CertificateDefect), Stopped(ExactProbLimit) }
```

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
- It SHALL recompile the package (FR-098), re-admit the subject, and
  re-enumerate the product through `ModelSystem` and the monitor, using
  `qsl-eval`'s property-automaton translation and enabledness (ADR-018
  LA-5) and its own product enumeration and graph algorithms, and no
  `qsl-analyze` code: every
  reachable product state for an `Upper`, `Exact` or `LongRun` certificate,
  and the states the certificate names and their successors for a `Lower`
  one, an unexplored successor counting as value 0.
- It SHALL recompute the decided states, `Prob0` and `Prob1`, the
  components and, with a fairness set, the maximal fair end components by
  FR-200's refinement, with its own graph code.
- With `F` the one-step operator of the objective, it SHALL check in exact
  rationals: **Upper**, `F(y) <= y` at every state; **Lower**, `x <= F(x)`
  at every named state and, at every non-target state with `x > 0`, a
  successor with positive probability, `x > 0` and a smaller rank, under the
  workload, the policy, or every action for a minimum; **Exact**, both with
  equality; for a finite-horizon product the rank SHALL be the remaining
  horizon read from the monitor state; **expected reward**, the same with
  the reward added and the `+∞` states recomputed; **LongRun**, per
  component the multichain optimality equations for `R · ([P] − ρ_C)` with
  gain 0, as equalities under the component's policy and inequalities over
  every action, plus a certificate for the combination.
- It SHALL accept when the conditions hold and the initial state's value is
  on the claim's side of the threshold, and report the side; otherwise it
  SHALL reject and name the first failing condition and state in canonical
  order.
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
| FR-201-AC-5 | An `Opt` model whose initial state has the actions `go`, which activates and then sets `M` to 1 with probability `1/4` and to 3 with `3/4`, and `skip`, which never activates. For `quantile 1/2 of M >= 3` over every scheduler, the `Upper` certificate with `y = 0` at the initial state and `−1/4` after `go` is accepted with side `AtLeast`: only `skip` is tight, and no activated state is reachable through it. The same claim over a variant whose `go` sets `M` to 1 and 3 with `1/2` each, with `y = 0` at both states, is rejected, naming the activated state after `go`, since `go` is tight. | Test (TC-636) |

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
