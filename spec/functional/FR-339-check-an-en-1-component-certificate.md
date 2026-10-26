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
  - target: ix://agent-ix/quire-specification/FR-370
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-212
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-228
    type: references
  - target: ix://agent-ix/quire-specification/FR-430
    type: references
  - target: ix://agent-ix/quire-specification/FR-436
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-221
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-222
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: depends_on
---
# FR-339: Check an EN-1 component certificate

## Description

`qsl-replay`'s `check_components` SHALL accept a `ComponentCertificate`
that EN-1 returns with a liveness (TP-4) proof, over a model subject or a
protocol subject (FR-215), exactly when its closure
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

- FR-338's `CertificateRequest` for a TP-4 clause over a model subject, or
  FR-338's `ProtocolCertificateRequest`, with the run's memory and instance
  bounds, for a TP-4 clause over a protocol subject.
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
) -> Result<TemporalCertificateCheck, ClosureRefusal>; // FR-338; existing stop causes only

pub fn check_protocol_components(
    request: &ProtocolCertificateRequest<'_>,   // FR-338
    certificate: &ComponentCertificate,
    limits: &ModelCheckLimits,
    cancel: &Cancel,
) -> Result<TemporalCertificateCheck, ClosureRefusal>; // FR-338; existing stop causes only
```

Both entry points return FR-338's `ClosureRefusal` exactly when their
closure check returns one, and component work adds no refusal.

A `ComponentCertificate` names its `package_id` once, as its closure's
`package_id` (FR-338); its components carry none.

`MissingAcceptance`'s `set` indexes the generalized Büchi acceptance sets
of the item's automaton in FR-338's order, by the canonical bytes of their
identities. For a TP-4 clause with a guarded `on each` activation (context
activation `"each-when"`, FR-338 "Guarded TP-4 clause") the sets include
the activation set `["activation"]`, which is index 0; a state belongs to it
exactly when its key's phase is `"active"`.

`UnfairWeak`'s index is into the clause's resolved constraints: one per
`whole` constraint, and one per transition identity of an `each`
constraint, in FR-123's order of the clause's constraints; the transition
identities of one `each` constraint are taken over the union of every
binding's transitions, in ascending order of their FR-101 canonical
transition key bytes, by the producer and the checker alike.

### Protocol subjects

Over a protocol subject (FR-215), the clause's resolved constraints SHALL be
read from its fairness set (FR-212, FR-228), in the set's order, the JCS
bytes of its members (QSpec FR-430):

- one resolved constraint per `whole` member;
- one resolved constraint per `flush(b)` member: QSpec FR-436 derives one
  such member per thread `b`, whose target names `b`'s thread path and whose
  one class is `b`'s `flush` steps, so the member resolves to its constraint
  whether or not any reached state enables it;
- for any other `each` member, one per class of it (QSpec FR-430's `each`
  classes) that a step of an edge the closure check computed, or a step
  enabled at a reached state, belongs to, ascending by the canonical bytes of
  the class's identity (its thread path, or its thread path and application);
- the visibility pairs last, as the `UnfairVisibility` rule below states.

At a protocol state, a class SHALL be enabled when one of its steps is
enabled in the protocol system (FR-212), memory steps included (FR-221,
FR-222). No reached state of an accepted certificate is bound-limited, since
FR-338 "Memory bounds" rejects one with `SuccessorMissing`, so every enabled
store is an expanded edge. Under `tso`, `flush(b)` is
enabled exactly when `b`'s buffer is non-empty (ADR-025 MS-4). The closure
check computes these enabled sets, so the component rules read them as they
read a model state's.

## Outputs

- `Accepted` when closure and component checks complete successfully.
- `Rejected(CertificateRejection{rule, at})` on a certificate defect.
- `PackageMismatch{certificate, recompiled}` when the closure's `package_id`
  differs from the recompiled package's (FR-338), with no rule evaluated.
- `Stopped(IncompleteCause)` with the exact existing limit/value/setting or
  cancellation source when checker work cannot complete (FR-338, FR-127).

## Behavior

- `check_protocol_components` SHALL apply every rule of this requirement as
  `check_components` does, running FR-338's `check_protocol_closure` on the
  certificate's closure in place of `check_closure` and reading the resolved
  constraints and enabled sets by "Protocol subjects".
- The closure checks (`check_closure`, `check_protocol_closure`) SHALL
  refuse a closure certificate offered for a TP-4 item with the typed
  refusal `LivenessNeedsComponents`, since a closure holds no acceptance
  check; FR-127 settles the refusal `failed`, never `proved`.
- The component checks SHALL refuse an item that is not a TP-4 clause with
  the typed refusal `NotLiveness`.
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
- The checker's component work (enumeration, partition, `sub` levels and
  every witness rule) SHALL read only facts the closure check computed: the
  reached product states, their edges, the automaton states, each model
  state's enabled set (the labels of its FR-120 transitions, FR-130,
  ADR-019 FS-1) and each state's phase. It SHALL evaluate no contract
  clause or atom and SHALL materialize no product state, edge or automaton
  state, since the components partition the closure's reached states.
  Component work therefore SHALL charge none of `max_states`,
  `max_transitions` and `max_automaton_states` and SHALL NOT consume the
  subject's FR-120 evaluation meter. The meter is not cumulative across the
  invocation: FR-120 gives each clause evaluation a fresh `Meter` with the
  request's budget (FR-107), so every meter stop of a component check comes
  from a clause evaluation inside the closure check.
- The checker SHALL check the Cancel handle at each component, listed state
  and computed edge component work reads; cancellation is the only stop
  component work can reach. When the handle is cancelled during component
  work, the checker SHALL return `Stopped(IncompleteCause::Cancelled{source})`,
  as FR-338 specifies. A limit stop of a component check SHALL be the
  closure check's own `Stopped(IncompleteCause::LimitReached{limit, value,
  setting})`, propagated unchanged.
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
- For a guarded TP-4 clause, the checker SHALL run the closure check by
  FR-338's guarded rules, re-reading the `when` condition at every reached
  state, and SHALL decide `MissingAcceptance{set: 0}` from the phases of the
  component's states: it holds exactly when no state of the component is
  active. A component of waiting states has no fair accepting cycle by that
  witness whatever fairness it meets; a component that holds an active
  state needs another witness.
- The checker SHALL accept otherwise.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-339-AC-1 | Accepted vector: FR-126-AC-1's weak `each` outcome returns a `ComponentCertificate` whose accepting components `{(*, 0, q1)}` and `{(*, 1, q1)}` carry `UnfairWeak` naming the `each` constraint's identity `upd(b)`, and `check_components` accepts it. | Test (TC-526) |
| FR-339-AC-2 | Rejected vectors: AC-1's certificate with the witness of `{(*, 0, q1)}` changed to `UnfairWeak` naming `upd(a)`, which edges inside the component take, is rejected `WitnessFails` at that component's first state; with those two components listed before the components that reach them, `BackwardEdge`; with one state listed in two components, `NotPartition`; with a component's witness `Trivial` over a state with a self-edge, `WitnessFails`. | Test (TC-526) |
| FR-339-AC-3 | Accepted vector: FR-228-AC-5's synthesized SCC with `r` the only accepting state yields a component certificate whose component `{p, q, r}` carries `UnfairVisibility` naming the pair (`b`, `ℓ`), with `sub` `{r}` (`Trivial`) then `{p, q}` (`MissingAcceptance`), and `check_components` accepts it although the removed edge `q → r` goes from `{p, q}` to the earlier `{r}`; the same certificate with the witness `UnfairWeak` over that pair, or with `q → r` not removed (so `BackwardEdge` fires), is rejected. | Test (TC-526) |
| FR-339-AC-5 | Fixture: ADR-018 §6's example unit with universe `{a, b}`, both versions 0, and a checked package that carries the clause's `temporal_clause` node in the shape S4 emits for a guarded `on each` activation (FR-337-AC-6), built by the test as in FR-338-AC-10, the condition read by the test's own letters. `eventually holds(c.versionNumber = 2)` under infinite-trace `on each` with the condition `c.versionNumber = 1` and a weak `each` constraint on `attemptUpdate` returns a `ComponentCertificate` in which, for binding `a`, the component of the active states at `(1,0)`, `(1,1)`, `(1,2)` carries `UnfairWeak` naming `upd(a)` and the component of the waiting states carries `MissingAcceptance{set: 0}` (symmetrically for `b`); `check_components` accepts it and FR-127 settles `proved`. The same clause with the condition `c.versionNumber = 7` and no fairness constraint gives a certificate whose every non-trivial component carries `MissingAcceptance{set: 0}`, accepted. Rejected vectors on the first certificate: the waiting component's witness changed to `UnfairWeak` naming `upd(a)` is `WitnessFails` (edges inside it take `upd(a)`); the active component's witness changed to `MissingAcceptance{set: 0}` is `WitnessFails` at its first state; the closure with binding `a`'s active state at `(1,0)` removed is `SuccessorMissing` at it; with the condition undefined at version 1, `BadState` at binding `a`'s waiting state at `(0,0)`, whose expansion reads it. | Test (TC-1215) |
| FR-339-AC-4 | Package identity. AC-1's certificate carries in its closure the `package_id` of the package EN-1 explored, equal to the recompiled one, and is accepted. The same certificate with its closure's `package_id` replaced by that of the same unit with the claim's target edited to `c.versionNumber = 1` returns `PackageMismatch{certificate, recompiled}` naming both identities, and settles `inconclusive`, `unsettled`, `Inconclusive(StaleDependency{requested, recompiled})` with `requested` the closure's `package_id`, never `CertificateRejected` and never `proved`. That stale certificate with AC-2's `upd(a)` witness (a `WitnessFails` defect) still returns `PackageMismatch`, because no rule is evaluated. | Test (TC-1141) |
| FR-339-AC-6 | Charges. AC-1's certificate checked with `max_states`, `max_transitions` and `max_automaton_states` equal to the product states, edges and automaton states its closure holds is `Accepted`; with `max_states` one below that count the closure check returns `Stopped(LimitReached{MaxStates, value, setting})` with the consumed value and the request's setting, never `Rejected` or `Accepted`. Component work charges nothing: the `Ring` unit (population `rings`, field `v: Int[0, 2]`, universe `{r}`, initial `v` 0, operations `a` from 0 to 1, `x` from 0 to 0, `b` from 1 to 2 and `c` from 2 to 0, no `terminal` declaration) with `always eventually holds(r.v = 1)` under a weak constraint on `a` (no granularity) returns `Holds{Exhaustive}` over five product states `(0,q0)`, `(0,q1)`, `(1,q0)`, `(2,q0)`, `(2,q1)` (canonical breadth-first indices 0 to 4) and nine edges, with two automaton states, and a certificate whose components are listed `{(0,q0), (1,q0), (2,q0)}` (`MissingAcceptance`), `{(2,q1)}` (`Trivial`), `{(0,q1)}` (`UnfairWeak` naming `a`, enabled at `v` 0 by the closure's enabled set and taken by no edge inside, whose one edge is `x`). Checked with `max_states` 5, `max_transitions` 9 and `max_automaton_states` 2 it is `Accepted`: the closure check charges all five product states, nine edges and two automaton states and the three components charge none of them. Under the default meter budget the certificate is `Accepted`, as FR-338's closure check of its closure alone is. Under a meter budget whose `work_units` is 0, the closure check's first clause or atom evaluation at the initial state is denied, and `check_components` returns `Stopped(LimitReached{EvaluationMeter, value, setting})` equal in limit, value and setting to the stop FR-338's closure check of the same closure returns under that budget, before any component rule runs; for every meter budget the certificate is accepted exactly when its closure check alone completes. AC-1's certificate checked with a Cancel handle cancelled at its first check after the closure check completes returns `Stopped(Cancelled{source: Requested})`. Each stop settles `incomplete` (FR-127), never `CertificateRejected` or `proved`. | Test (TC-526) |
| FR-339-AC-7 | Protocol subject. FR-228-AC-2's `tso` proof of `eventually holds(c.r = 1)`, checked by `check_protocol_components` under its `ProtocolCertificateRequest` with `max_store_buffer` and `max_messages` `None`, returns a `ComponentCertificate` whose resolved constraints are its fairness set's members in JCS order, the `flush` members of `w` and `r` each resolving to exactly one constraint (QSpec FR-436), `r`'s although `r` never stores. In it, the component of the product states at which `w` is done with buffer `[x := 1]` and `r` polls carries `UnfairWeak` naming `w`'s `flush` class, the only weak constraint enabled at every state of it and taken by no edge inside it (the scheduler constraint of branch `w` is not enabled there, since `flush` is in no thread class); the component at which `w` has not stored and `r` polls carries `UnfairWeak` naming the scheduler constraint of branch `w`; and `check_protocol_components` accepts it. Rejected vectors, each `WitnessFails`: the first component's witness changed to name the scheduler constraint of branch `w`, or `r`'s `flush` class (`r`'s buffer is empty at every state); the second component's witness changed to name `w`'s `flush` class (`w`'s buffer is empty before its store). | Test (TC-526) |
| FR-339-AC-8 | A closure is not a liveness proof. The weak `each` claim over the ADR-018 §6 subject with letters that never hold returns `Violated`; a `ClosureCertificate` grown from the checker's own `InitialMissing` and `SuccessorMissing` rejections of that claim, until it holds every reached product state, is refused by `check_closure` with `LivenessNeedsComponents` and, as the certificate of a `Holds`, settles `failed` (FR-127), never `proved`; the component certificate of AC-1's claim settles `proved`, `certified`. `check_components` for the `always holds(c.value <= 3)` item over the `Counter` subject returns `NotLiveness`. Over `Spin` (FR-228-AC-2's unit), `check_protocol_components` accepts the proof's certificate, rejects its components reversed `BackwardEdge` and emptied `NotPartition`, returns `PackageMismatch` for a foreign `package_id`, and `check_protocol_closure` and `check_protocol_components` return `LivenessNeedsComponents` and `NotLiveness` for a closure of the liveness proof and components for the deadlock-freedom item. | Test (TC-526) |

## Dependencies

- ADR-018 PC-4, PC-5, FA-2, FA-4, LA-3; ADR-019 AM-7 (`UnfairStrong`).
- [FR-338](FR-338-check-an-en-1-closure-certificate.md) (the closure check
  and the rejection type), [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (the certificate's producer), [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the settlement that runs the check).

## References

- Owning ticket: Linear QSL-366.
