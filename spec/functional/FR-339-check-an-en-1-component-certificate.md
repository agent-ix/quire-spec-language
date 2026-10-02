---
id: FR-339
title: "Check an EN-1 component certificate"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-015
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
---
# FR-339: Check an EN-1 component certificate

## Description

`qsl-replay`'s `check_components` SHALL accept a `ComponentCertificate`
that EN-1 returns with a liveness (TP-4) proof exactly when its closure
passes FR-338's check and its components partition the closure in
topological order, each with a witness that no fair accepting cycle stays
inside it (ADR-018 PC-4, PC-5, LA-3). Every cycle lies inside one
component, so an accepted certificate proves that no fair accepting cycle
exists. A proof whose certificate it accepts settles `proved`, `Certified`
(FR-127); one it rejects settles `inconclusive`, `CertificateRejected`.

## Use case

A verification operator relies on a liveness proof under weak fairness. The
core cannot trust the engine's SCC search, so it checks a partition of the
product graph whose components each carry a reason a fair run cannot cycle
in it: no cycle at all, a missing acceptance set, or a fairness constraint
it starves.

## Inputs

- FR-338's `CertificateRequest` for a TP-4 clause.
- A `ComponentCertificate`.

```rust
pub struct ComponentCertificate {
    pub closure: ClosureCertificate,          // FR-338
    pub components: Vec<Component>,           // topological order
}
pub struct Component { pub states: Vec<ProductStateRef>, pub witness: ComponentWitness }
pub enum ComponentWitness {
    Trivial,
    MissingAcceptance { set: u32 },
    UnfairWeak { constraint: u32 },           // index into the resolved constraints
    UnfairStrong { constraints: Vec<u32>, sub: Vec<Component> },  // every failing strong constraint
}
pub enum ProofCertificate {
    Closure(ClosureCertificate),
    Component(ComponentCertificate),
}

pub fn check_components(request: &CertificateRequest<'_>, certificate: &ComponentCertificate)
    -> Result<(), CertificateRejection>;
```

`UnfairWeak`'s index is into the clause's resolved constraints: one per
`whole` constraint, and one per transition identity of an `each`
constraint, in FR-123's order.

## Outputs

- `Ok(())` when the certificate is accepted.
- FR-338's `CertificateRejection{rule, state}` otherwise.

## Behavior

- The checker SHALL run FR-338's check on the certificate's closure,
  without the monitor-rejection part of `BadState`; a deadlocked state is
  not bad for a TP-4 item, whose terminal state reads by its stutter edge
  (ADR-018 DL-6).
- The checker SHALL reject with `NotPartition` when the components do not
  partition the closure's reached states.
- The checker SHALL reject with `BackwardEdge` when an edge it computed
  goes from a component to an earlier one.
- The checker SHALL reject with `WitnessFails` when a component's witness
  does not hold: `Trivial` needs one state with no edge to itself;
  `MissingAcceptance{set}` needs no state of the component in that
  acceptance set; `UnfairWeak{constraint}` needs that weak constraint
  enabled, by FR-120's enabled set of the model state, at every state of
  the component and taken by no edge with both ends in it;
  `UnfairStrong{constraints, sub}` needs every listed strong constraint
  taken by no edge with both ends in the component, and `sub` to partition
  exactly the component's states at which none of them is enabled, with
  every edge between two of them going to the same or a later
  sub-component and every sub-component's witness holding (ADR-019 AM-7).
- The checker SHALL accept otherwise.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-339-AC-1 | Accepted vector: FR-126-AC-1's weak `each` outcome returns a `ComponentCertificate` whose accepting components `{(*, 0, q1)}` and `{(*, 1, q1)}` carry `UnfairWeak` naming the `each` constraint's identity `upd(b)`, and `check_components` accepts it. | Test (TC-526) |
| FR-339-AC-2 | Rejected vectors: AC-1's certificate with the witness of `{(*, 0, q1)}` changed to `UnfairWeak` naming `upd(a)`, which edges inside the component take, is rejected `WitnessFails` at that component's first state; with those two components listed before the components that reach them, `BackwardEdge`; with one state listed in two components, `NotPartition`; with a component's witness `Trivial` over a state with a self-edge, `WitnessFails`. | Test (TC-526) |

## Dependencies

- ADR-018 PC-4, PC-5, FA-2, FA-4, LA-3; ADR-019 AM-7 (`UnfairStrong`).
- [FR-338](FR-338-check-an-en-1-closure-certificate.md) (the closure check
  and the rejection type), [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (the certificate's producer), [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the settlement that runs the check).

## References

- Owning ticket: Linear QSL-366.
