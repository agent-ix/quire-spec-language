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
(FR-127); one it rejects settles `inconclusive`, `CertificateRejected`; one
whose certificate names a `package_id` other than the recompiled package's
settles `inconclusive`, `StaleDependency` (FR-282).

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
    pub closure: ClosureCertificate,          // FR-338; its package_id is the certificate's
    pub components: Vec<Component>,           // topological order
}
pub struct Component { pub states: Vec<ProductStateRef>, pub witness: ComponentWitness }
pub enum ComponentWitness {
    Trivial,
    MissingAcceptance { set: u32 },
    UnfairWeak { constraint: u32 },           // index into the resolved constraints
    UnfairStrong { constraints: Vec<u32>, sub: Vec<Component> },  // every failing strong constraint
    UnfairVisibility { constraints: Vec<u32>, sub: Vec<Component> }, // ADR-025 MF-2: every failing visibility pair; a pair resolves to one constraint per (thread, location): thread bytes, then location bytes
}
pub enum ProofCertificate {
    Closure(ClosureCertificate),
    Component(ComponentCertificate),
}

pub fn check_components(
    request: &CertificateRequest<'_>,
    certificate: &ComponentCertificate,
    limits: &ModelCheckLimits,
    cancel: &Cancel,
) -> TemporalCertificateCheck; // FR-338; existing stop causes only
```

A `ComponentCertificate` names its `package_id` once, as its closure's
`package_id` (FR-338); its components carry none.

`UnfairWeak`'s index is into the clause's resolved constraints: one per
`whole` constraint, and one per transition identity of an `each`
constraint, in FR-123's order.

## Outputs

- `Accepted` when closure and component checks complete successfully.
- `Rejected(CertificateRejection{rule, at})` on a certificate defect.
- `PackageMismatch{certificate, recompiled}` when the closure's `package_id`
  differs from the recompiled package's (FR-338), with no rule evaluated.
- `Stopped(IncompleteCause)` with the exact existing limit/value/setting or
  cancellation source when checker work cannot complete (FR-338, FR-127).

## Behavior

- The checker SHALL compare the recompiled package's `package_id` with the
  closure's `package_id` first, before the closure check and any component
  rule.
- When the two `package_id`s differ, the checker SHALL return
  `PackageMismatch{certificate, recompiled}` naming both, evaluating no
  closure, partition, edge or witness rule (FR-282); FR-127 settles it
  `Inconclusive(StaleDependency{requested, recompiled})` with `requested`
  the closure's `package_id`.
- The checker SHALL run FR-338's check on the certificate's closure,
  without the monitor-rejection part of `BadState`; a deadlocked state is
  not bad for a TP-4 item, whose terminal state reads by its stutter edge
  (ADR-018 DL-6). It SHALL pass the same request limits and Cancel handle
  to the closure check, and immediately propagate `Rejected` or `Stopped`
  unchanged.
- The checker SHALL charge component enumeration, partition and
  acceptance/fairness work to the same counters, continuing the closure
  check's consumption within the one invocation (FR-338). The checker SHALL
  charge once each product state, edge or automaton state that the
  component rules materialize first, and SHALL NOT charge again one that
  the closure check charged. The checker SHALL charge the subject's FR-120
  evaluation meter once per model state for each FR-120 enabled-set or
  fairness-enabledness evaluation the component rules perform, and the
  checker SHALL reuse, charging nothing, a value the closure check already
  evaluated. The closure check evaluates the successor expansion and the
  atoms of each state and no constraint's enabled set; the first component
  rule that reads a model state's enabled set charges it, and every later
  read of that model state, from any component, `sub` level, constraint or
  product state sharing it, reuses it.
- The checker SHALL check the Cancel handle at each charge and at each
  component, listed state and computed edge it reads. When component work
  reaches a limit, the checker SHALL return
  `Stopped(IncompleteCause::LimitReached{limit, value, setting})` with the
  actual consumed value and the request setting; when the handle is
  cancelled, it SHALL return `Stopped(IncompleteCause::Cancelled{source})`,
  as FR-338 specifies.
  `Accepted` requires every component check to finish; no stop is translated into a defect or an accepted proof.
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
  sub-component and every sub-component's witness holding (ADR-019 AM-7);
  The visibility fairness member resolves to one resolved constraint per (thread, location) pair, in ascending order of the thread's identity bytes and then the location's identity bytes, after the clause's other resolved constraints; `UnfairVisibility.constraints` indexes them.
  `UnfairVisibility{constraints, sub}` needs every listed visibility pair to
  have an edge with both ends in the component on which the thread loads the
  location and none on which it loads it from its mo-last message, and `sub`
  to partition the component's states over the subgraph without the edges that
  load any listed pair, with every witness below holding over that subgraph.
  An edge so removed is exempt from `BackwardEdge` and from every witness
  below the `UnfairVisibility` (ADR-025 MF-2).
- The checker SHALL accept otherwise.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-339-AC-1 | Accepted vector: FR-126-AC-1's weak `each` outcome returns a `ComponentCertificate` whose accepting components `{(*, 0, q1)}` and `{(*, 1, q1)}` carry `UnfairWeak` naming the `each` constraint's identity `upd(b)`, and `check_components` accepts it. | Test (TC-526) |
| FR-339-AC-2 | Rejected vectors: AC-1's certificate with the witness of `{(*, 0, q1)}` changed to `UnfairWeak` naming `upd(a)`, which edges inside the component take, is rejected `WitnessFails` at that component's first state; with those two components listed before the components that reach them, `BackwardEdge`; with one state listed in two components, `NotPartition`; with a component's witness `Trivial` over a state with a self-edge, `WitnessFails`. | Test (TC-526) |
| FR-339-AC-3 | Accepted vector: FR-228-AC-5's synthesized SCC with `r` the only accepting state yields a component certificate whose component `{p, q, r}` carries `UnfairVisibility` naming the pair (`b`, `ℓ`), with `sub` `{r}` (`Trivial`) then `{p, q}` (`MissingAcceptance`), and `check_components` accepts it although the removed edge `q → r` goes from `{p, q}` to the earlier `{r}`; the same certificate with the witness `UnfairWeak` over that pair, or with `q → r` not removed (so `BackwardEdge` fires), is rejected. | Test (TC-526) |
| FR-339-AC-4 | Package identity. AC-1's certificate carries in its closure the `package_id` of the package EN-1 explored, equal to the recompiled one, and is accepted. The same certificate with its closure's `package_id` replaced by that of the same unit with the claim's target edited to `c.versionNumber = 1` returns `PackageMismatch{certificate, recompiled}` naming both identities, and settles `inconclusive`, `unsettled`, `Inconclusive(StaleDependency{requested, recompiled})` with `requested` the closure's `package_id`, never `CertificateRejected` and never `proved`. That stale certificate with AC-2's `upd(a)` witness (a `WitnessFails` defect) still returns `PackageMismatch`, because no rule is evaluated. | Test (TC-1141) |
| FR-339-AC-6 | Charges. AC-1's certificate checked with `max_states`, `max_transitions` and `max_automaton_states` equal to the product states, edges and automaton states its closure holds is `Accepted`; with `max_states` one below that count the closure check returns `Stopped(LimitReached{MaxStates, value, setting})` with the consumed value and the request's setting, never `Rejected` or `Accepted`. Component-work exhaustion: the `Ring` unit (population `rings`, field `v: Int[0, 2]`, universe `{r}`, initial `v` 0, operations `a` from 0 to 1, `x` from 0 to 0, `b` from 1 to 2 and `c` from 2 to 0, no `terminal` declaration) with `always eventually holds(r.v = 1)` under a weak constraint on `a` (no granularity) returns `Holds{Exhaustive}` over five product states `(0,q0)`, `(0,q1)`, `(1,q0)`, `(2,q0)`, `(2,q1)` (canonical breadth-first indices 0 to 4) and nine edges, with two automaton states, and a certificate whose components are listed `{(0,q0), (1,q0), (2,q0)}` (`MissingAcceptance`), `{(2,q1)}` (`Trivial`), `{(0,q1)}` (`UnfairWeak` naming `a`, enabled at `v` 0 and taken by no edge inside, whose one edge is `x`). Checked with `max_states` 5, `max_transitions` 9 and `max_automaton_states` 2 it is `Accepted`, since the closure check charges all five product states, nine edges and two automaton states and the three components charge none of them. Let `M` be the evaluation-meter units the closure check consumes (the same certificate with no components) and `E` the units FR-120's enabled-set evaluation of the model state `v` 0 charges: the first two components read no enabled set and charge nothing, and the third charges `E` once, however many of its product states share `v` 0. With the meter budget `M + E` the certificate is `Accepted`; with `M + E - 1` the checker returns `Stopped(LimitReached{EvaluationMeter, value, setting})` at that enabled-set evaluation during the third component, with the consumed value `M + E - 1` and the request's meter setting, and the counters still 5, 9 and 2. AC-1's certificate checked with a Cancel handle cancelled at its first check after the closure check completes returns `Stopped(Cancelled{source: Requested})`. Each stop settles `incomplete` (FR-127), never `CertificateRejected` or `proved`. | Test (TC-526) |

## Dependencies

- ADR-018 PC-4, PC-5, FA-2, FA-4, LA-3; ADR-019 AM-7 (`UnfairStrong`).
- [FR-338](FR-338-check-an-en-1-closure-certificate.md) (the closure check
  and the rejection type), [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (the certificate's producer), [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the settlement that runs the check).

## References

- Owning ticket: Linear QSL-366.
