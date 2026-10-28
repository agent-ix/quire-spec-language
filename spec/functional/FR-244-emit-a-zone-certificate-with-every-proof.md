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
  - target: ix://agent-ix/quire-spec-language/FR-149
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-430
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
item it proves. A timed refinement's safety half (FR-430) is such a
proof: its `Holds` carries the same certificate over the refinement product.

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
        pub nodes: Vec<CertNode>,                   // (state key, automaton state, canonical Dbm, locked: bool)
    pub edges: Vec<CertEdge>,                   // (node, transition identity, target node)
    pub initial: NodeIndex,
    pub components: Option<Vec<CertComponent>>, // CF-2 forms only
    pub divergence: Option<Vec<DivergenceWitness>>, // time-lock-freedom item only
    pub admitted: DivergenceWitness,            // every proof: TD-6 vacuity witness
    pub refinement: Option<RefinementPart>,     // refinement form only
}

pub struct RefinementPart {                     // FR-430
    pub refinement: NodeKey,                    // FR-149
    pub concrete_package: PackageId,
    pub abstract_package: PackageId,
    pub positions: Vec<CertifiedPosition>,      // FR-149's type, unchanged
    pub taken: Vec<Option<Taken>>,              // FR-141's Taken, one per entry of `edges`, in order; None for every edge whose target is in the locked region, the failing edge into the entry included
}
```

## Behavior

- **Reachability part.** For a TT-1, TT-2 or TT-3 item, or a
  deadlock-freedom item, the certificate SHALL list nodes, each a discrete
  state key, a claim-automaton state and a canonical DBM, and for each node
  and each transition identity enabled from it one edge to a named target
  node whose zone covers the successor under the aLU simulation with the
  listed LU bounds. It SHALL name the node that covers the initial symbolic
  state. No listed node SHALL be bad (a rejecting automaton state, or a deadlocked
  valuation for the deadlock-freedom item) unless it is marked `locked`.
- **Locked nodes.** For each violating symbolic state FR-239 continued past
  because its violating points are time-locked, the certificate SHALL list
  the violating part `Z ∩ Bad` as its own node marked `locked`, with its
  edges, so the certificate's edges from it cover its forward closure.
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
- **Refinement form.** For the safety half of a refinement whose concrete
  model is timed (FR-430), the certificate SHALL be this same
  `ZoneCertificate` with `refinement` present and the reachability part read
  over the refinement product, with these and only these differences:
  - a node's second component (`CertNode.automaton_state`, a `usize`) is an
    index into `refinement.positions` (FR-149's `CertifiedPosition`:
    concrete state key, history values, sorted abstract states) in place of
    a claim-automaton state. The **locked region** is the `locked` node
    entered by a failing step and every node reached from it over
    certificate edges; each of its nodes carries the reserved index
    `positions.len()`, which names no position, and only the entry node
    carries the `locked` mark;
  - every edge is a discrete step (a delay is never an edge), and no node is
    bad by an automaton state: instead, `check_step` (FR-141) SHALL pass on
    every edge between two positioned nodes, the target's position and the
    edge's `Taken` being those `check_step` yields, and the only edge it may
    fail on is an edge into a `locked` entry node, the post-state of the
    failing step (FR-430);
  - the initial node's position is the one `check_initial` yields for the
    concrete initial state, and no certificate edge may leave the locked
    region for a positioned node;
  - `refinement`, `concrete_package` and `abstract_package` SHALL be those
    of the item, as FR-149 requires of its certificate;
  - edges inside the locked region are concrete steps only, are not judged by
    `check_step` and carry `taken` `None`, as does the failing edge into the locked entry node (whose `Taken` does not exist). `taken` is serialized with the edges in canonical edge order, one entry per edge, `None` as an absent value, so equal runs give byte-equal bytes.
  The certificate has no other new member, and the obligation identity is
  the refinement item's.
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
| FR-244-AC-4 | Over a `Lock` model (one object with `phase: {A, B}` and a clock `x` never reset; `go` moves `A` to `B` with no guard; `time invariant when phase = B { x <= 1 ms }`; `ping`, enabled in `B`, changes no data field), `always holds(c.phase = A)` is proved, and its certificate lists the node `(B, x <= 1)` marked `locked`, with `ping` edges that stay in `B` and reach no quiescent node and no cycle of positive total delay. | Test (TC-699) |
| FR-244-AC-5 | The safety half of FR-430-AC-1's `TReq` refinement returns a certificate with `refinement` present, each node's second component an index into `positions`, every edge a discrete `send` step, an `admitted` path, and the refinement item's obligation identity; FR-245 accepts it. FR-430-AC-6's `Holds` certificate lists the post-state of `undo` as a `locked` node with no position, entered by the failing `undo` edge. Two runs give byte-equal certificates. | Test (TC-699) |

## Dependencies

- ADR-026 §8.1 CF-1, CF-2, CF-5, TD-6; ADR-013 O-09.
- [FR-239](FR-239-check-a-timed-claim-by-symbolic-zone-search.md),
  [FR-240](FR-240-decide-timed-liveness-and-time-lock-freedom-on-the-symbolic-graph.md).

## References

- QSpec half: QSpec FR-418 (Linear STD-139) owns the zone certificate wire
  and its place in QSpec FR-331 (ADR-026 OV-12).
- S. Wimmer and J. von Mutius, 2020; S. Wimmer, F. Herbreteau and J. van de
  Pol, 2020 (ADR-026 References).
