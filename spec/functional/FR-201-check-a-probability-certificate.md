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
iteration. The checker is part of the qualified core (ADR-029 RU-2): an
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
    pub policy: Option<Vec<(ProductKey, SchedulerChoice)>>,
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

- The checker SHALL refuse a certificate whose identity differs from the
  item's.
- It SHALL recompile the package (FR-098), re-admit the subject, and
  re-enumerate the product through `ModelSystem` and the monitor: every
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
- A checker run stopped by a budget SHALL return `Stopped` naming it.

### Verdict path

- The driver SHALL run the checker on the certificate that left S6c over
  E11 and settle `proved` only on `Accepted`; `Rejected` SHALL settle
  `inconclusive`, `CertificateRejected`; `Stopped` SHALL settle V-7.
- The checker's arithmetic and graph algorithms SHALL be code separate from
  EN-5's iterative and linear-algebra methods.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-201-AC-1 | §15.4 at threshold 0.95: the `Exact` certificate with values `24/25`, `4/5`, 1 at delivered states and 0 at the two-loss state is accepted with side `AtLeast`. The same certificate with `4/5` replaced by `9/10` is rejected, naming the first state in canonical order where the minimum over actions differs from the certified value; the item settles `inconclusive`, `CertificateRejected`. | Test (TC-636) |
| FR-201-AC-2 | §15.3's `LongRun` certificate (gain `1800/1801`, bias 0 and `−2000/1801`) is accepted; with the bias at `down` changed to `−2001/1801` it is rejected. §15.1's dyadic `Lower` certificate at 64 bits, ranked by remaining horizon, is accepted with side `AtLeast`. | Test (TC-636) |
| FR-201-AC-3 | A `Lower` certificate for §15.4 with its ranking omitted is rejected; one whose identity names a different claim is refused before any re-enumeration; §15.6's `FairTerminates` `Exact` certificate is accepted only after the checker recomputes that no fair end component exists, and the same certificate with its fairness set removed is rejected. | Test (TC-636) |
| FR-201-AC-4 | A proof whose certificate the checker accepts settles `proved`; with a checker `max_states` of 2 the check stops and the item settles `Incomplete(ResourceExhausted)` naming `max_states`. | Test (TC-636) |

## Dependencies

- ADR-028 SP-5, AR-4, CE-1 to CE-7, FS-9; ADR-029 RU-2, CB-2 (the qualified
  core); ADR-013 O-09.
- [FR-098](FR-098-execute-a-replay-request.md),
  [FR-196](FR-196-build-the-probabilistic-product.md),
  [FR-200](FR-200-decide-every-scheduler-claims-over-fair-schedulers.md).

## References

- QSpec half, which owns the certificate wire and the checker's conditions:
  Linear STD-137.
- ADR-029 (owning record of the qualified core and RU-2) is on its own draft
  branch: Linear QSL-390.
- Owning ticket: Linear QSL-371.
