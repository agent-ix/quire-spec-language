---
id: FR-244
title: "Emit a zone certificate with every zone-engine proof"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-240
    type: depends_on
---
# FR-244: Emit a zone certificate with every zone-engine proof

## Description

Every `Holds` outcome of EN-6's zone search SHALL carry a zone certificate
built from the retained symbolic graph, which a small checker verifies
without trusting the search (ADR-026 CF-1, CF-2, CF-5). A safety form
carries a reachability certificate; a liveness form and the
time-lock-freedom item add a component numbering with each component's
reason. Every certificate carries a witness that the subject admits a
time-divergent behaviour (ADR-026 TD-6) and the obligation identity of the
item it proves.

## Use case

An assessor does not trust a 10,000-line search engine. The engine's proof
comes with a certificate that a few hundred lines of independent code can
check, so the verdict rests on the checker alone.

## Inputs

- The retained symbolic graph, the LU bounds and the scale factor of a
  completed FR-239 or FR-240 run with no violation.

## Outputs

```rust
pub struct ZoneCertificate {
    pub obligation: ObligationIdentity,         // ADR-013 O-09
    pub scale: BigInt,                          // FR-238
    pub lu_bounds: Vec<LuBounds>,               // per discrete state and clock
    pub nodes: Vec<CertNode>,                   // (state key, automaton state, canonical Dbm)
    pub edges: Vec<CertEdge>,                   // (node, transition identity, target node)
    pub initial: NodeIndex,
    pub components: Option<Vec<CertComponent>>, // CF-2 forms only
    pub divergence: Option<Vec<DivergenceWitness>>, // time-lock-freedom item only
    pub admitted: DivergenceWitness,            // every proof: TD-6 vacuity witness
}
```

## Behavior

- **Reachability part.** For a TT-1, TT-2 or TT-3 item, or a
  deadlock-freedom item, the certificate SHALL list nodes, each a discrete
  state key, a claim-automaton state and a canonical DBM, and for each node
  and each transition identity enabled from it one edge to a named target
  node whose zone covers the successor under the aLU simulation with the
  listed LU bounds. It SHALL name the node that covers the initial symbolic
  state. No listed node SHALL be bad: a rejecting automaton state, or a
  deadlocked valuation for the deadlock-freedom item.
- **Büchi part.** For a TT-4 item and the time-lock-freedom item, the
  engine SHALL give each node a component index such that every edge goes
  to a node of equal or greater index, and give each component its reason: the
  acceptance set it lacks (the divergence set included), or a weak
  fairness constraint enabled at every valuation of every node in it and
  taken by no edge inside it. A strong fairness constraint SHALL state its
  reason by a nested numbering of the component with the nodes that enable
  it removed.
- **Divergence part.** For the time-lock-freedom item, each node SHALL name
  a path in the certificate to a divergent target: a quiescent node, or a
  component that does not fail for want of the divergence set.
- **Admitted part.** Every certificate SHALL carry `admitted`: a path of
  certificate edges from the node that covers the initial symbolic state
  to a quiescent node or to a node of a component with a cycle of positive
  total delay, so the proof is not vacuous (ADR-026 TD-6, CF-1).
- The certificate SHALL carry the obligation identity of the item, the
  canonical identity digest that binds it to the exact subject and claim.
- The certificate SHALL be canonical: nodes in discovery order, edges in
  canonical transition order, so equal runs give byte-equal certificates.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-244-AC-1 | `NoLateReply` over `Rpc` with `T = 4 ms`, run on the zone search, returns a certificate with a reachability part, no components, every edge's target zone covering its successor, an `admitted` path from the initial node to a quiescent node (`Idle`), and the item's obligation identity; FR-245 accepts it. | Test (TC-699) |
| FR-244-AC-2 | FR-240-AC-1's liveness proof returns a certificate with components in which every edge respects the numbering and each component names its reason; FR-240-AC-2's fair proof names `fair weak serve` as the reason of the component that holds the `idle` loop. | Test (TC-699) |
| FR-244-AC-3 | The time-lock-freedom item over the unmodified `Rpc` returns a certificate whose divergence part names, for every node, a path to a quiescent node. Two runs give byte-equal certificates. | Test (TC-699) |

## Dependencies

- ADR-026 §8.1 CF-1, CF-2, CF-5, TD-6; ADR-013 O-09.
- [FR-239](FR-239-check-a-timed-claim-by-symbolic-zone-search.md),
  [FR-240](FR-240-decide-timed-liveness-and-time-lock-freedom-on-the-symbolic-graph.md).

## References

- QSpec half: QSpec FR-418 (Linear STD-139) owns the zone certificate wire
  and its place in QSpec FR-331 (ADR-026 OV-12).
- S. Wimmer and J. von Mutius, 2020; S. Wimmer, F. Herbreteau and J. van de
  Pol, 2020 (ADR-026 References).
