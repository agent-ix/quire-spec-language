---
id: FR-149
title: "Check a refinement's simulation certificate in the qualified core"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-018
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-141
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-142
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-144
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: depends_on
---
# FR-149: Check a refinement's simulation certificate in the qualified core

## Description

When the explicit-state product proves a refinement's safety half (FR-142),
QSL's layer-A engine SHALL hand over the simulation relation it found as a
`SimulationCertificate`, and QSL's layer-6 crate `qsl-replay` SHALL hold
`check_simulation_certificate`, which accepts the certificate only when the
relation holds the concrete initial states and is closed under the concrete
successor relation with every step passing `check_step` (ADR-020 CT-1 to
CT-3). The checker reads nothing from the engine's search but the
certificate, so a certified `proved` does not depend on the engine (ADR-029
CB-2).

## Use case

A verification lead wants a refinement proof that counts as qualified
evidence. The model checker proves the compare-and-set counter refines the
abstract counter and writes down the relation it found. A small checker in
the qualified core recomputes every concrete step from that relation and
accepts it, so the proof stands without trusting the model checker's
search.

## Inputs

- The certificate:

```rust
pub struct SimulationCertificate {
    pub refinement: NodeKey,
    pub concrete_package: PackageId,
    pub abstract_package: PackageId,
    pub obligation: ObligationIdentity,
    pub positions: Vec<CertifiedPosition>,
    pub initial: Vec<u64>,                 // one per concrete initial state, in order
    pub successors: Vec<Vec<CertifiedEdge>>, // per position, FR-120 canonical order
}

pub struct CertifiedPosition {
    pub concrete: StateKey,
    pub history: HistoryValues,             // QSpec FR-378's encoding
    pub abstract_states: Vec<StateKey>,     // sorted
}

pub struct CertifiedEdge {
    pub transition: Option<ModelTransition>, // None: the terminal stutter edge (ADR-018 SM-4)
    pub post: u64,                          // index into `positions`
    pub taken: Taken,                       // FR-141
}
```

- The byte provision of FR-145 (both packages' sources and the subjects'
  initial snapshots) and the request's model-check limits.

## Outputs

```rust
pub enum CertificateCheck {
    Accepted,
    Rejected(CertificateRejection),        // FR-338's: first failing rule and locus
    Stopped(IncompleteCause),              // LimitReached{limit, value} or Cancelled{source}
}
```

`Stopped` has the shape of FR-338's `TemporalCertificateCheck::Stopped`: a
reached limit is `IncompleteCause::LimitReached{limit, value}` (FR-126's
`ReachedLimit`) and a cancellation is `IncompleteCause::Cancelled{source}`,
which names no limit.

The checker uses FR-338's `CertificateRejection{rule, at}`. FR-338's
`CertificateRule` gains `Malformed`, `InitialMismatch`, `BadState`,
`SuccessorsDiffer`, `StepFails` and `PostDiffers`, and its `CertificateLocus`
gains `SimulationStep { position: u64, edge: Option<u64>, verdict: Option<FailedStep> }`:
the certificate position, the edge when the rule concerns one, and the failing
`check_step` verdict for `StepFails`.

`FailedStep` is `check_step`'s verdict (FR-141) restricted to the three
alternatives that judge the relation: `Fails(RefinementFailure)`,
`Undefined(UndefinedEvaluation)` and `Undetermined(InconclusiveCause)`. It
excludes `Passes`, `Stopped` and `Fault`, which are never a rejection.

The terminal record writes the locus as `{"position": "<n>", "edge": "<n>" or
null, "verdict": <verdict or null>}`, with integers as decimal strings. A
`verdict` is an object `{"kind": <kind>, ...}` that holds the verdict's kind and
its own fields: `initial-not-abstract`, `stutter-changed`,
`abstract-step-rejected` (with `transition` and `cause`, so AC-3's
`transition: inc(c)` and `cause: Postcondition` reach the record),
`no-abstract-match`, `undefined-evaluation` and `undetermined`. `verdict` is
`null` for every rule but `StepFails`.

## Behavior

- **Production.** When FR-142's safety half returns `Holds`, the layer-A
  engine SHALL write the certificate from its retained product: every stored
  product position, each concrete initial state's position, and every
  retained edge with the `Taken` that `check_step` returned.
- **Identity.** The checker SHALL refuse with `stale_dependency`/
  `content-mismatch`, naming both values, a certificate whose refinement
  node differs from the item's.
- **Package identity.** FR-144 SHALL compare the certificate's
  `concrete_package`, `abstract_package` and `obligation` with the item's
  recompile (FR-098) before it calls the checker, and SHALL refuse a
  difference with the same `stale_dependency`/`content-mismatch`; the checker
  reads the refinement node only.
- **Shape.** Before any recomputation, the checker SHALL reject with rule
  `Malformed` a certificate whose `successors` and `positions` differ in
  length, whose `initial` and the concrete model's initial states differ in
  count, or in which a position is reached by no initial state and no listed
  edge, at the first such position.
- **Initial states.** For each concrete initial state `i`, the checker SHALL
  run `check_initial` and SHALL reject with rule `InitialMismatch` at
  `SimulationStep{position: initial[i], edge: None}` unless it passes with
  that position.
- **Undecided state.** For each position, when a contract of the concrete
  model is undecided there, so its successors are not known, the checker SHALL
  reject with rule `BadState` at that position.
- **Successors.** For each position, the checker SHALL compute the concrete
  successors with FR-120's `ModelSystem` and SHALL reject with rule
  `SuccessorsDiffer` at that position unless they are exactly the listed edges'
  transitions, in canonical order. A position with no concrete successor lists
  one edge whose `transition` is `None`, the terminal stutter edge of ADR-018
  SM-4. Post-states are judged by the step rule, not by this one.
- **Steps.** For each edge, the checker SHALL run `check_step` (FR-141)
  from the position and SHALL reject with rule `StepFails`, carrying the
  verdict, when it returns `Fails`, `Undefined` or `Undetermined`, and with
  rule `PostDiffers` unless its post position and `Taken` equal the edge's,
  each at that position and edge.
- **Meter stops.** When a step verdict is `Undetermined` only because a mapping
  row or a concrete contract ran out of the evaluation meter, or a `BadState`
  finding does, the checker SHALL stop with
  `Stopped(LimitReached{EvaluationMeter, value})` and SHALL NOT reject: a
  certificate is rejected only when the checker found it wrong (ADR-018 PC-2).
- **Order.** The checker SHALL check positions in index order and report
  the first rejection.
- **Limits.** The checker SHALL count positions against `max_states` and
  edges against `max_transitions`, with checked arithmetic, and SHALL stop
  at a reached limit with `Stopped(LimitReached{limit, value})`. A
  `StepVerdict::Stopped` from a `check_step` it calls SHALL end the check with
  `Stopped`, naming that limit and its value, and a cancellation SHALL end it
  with `Stopped(Cancelled{source})`.
- **Purity.** The checker SHALL read no path, environment variable, clock
  or search location, and its result SHALL be a function of the
  certificate, the byte provision and the limits.
- **Settlement.** FR-144 SHALL settle the safety half `proved` only on
  `Accepted`, V-6 `CertificateRejected{rule, at}` (ADR-018 PC-2) on
  `Rejected`, and V-7 on `Stopped`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-149-AC-1 | ADR-020 §8's `CasRefinesCounter` without its `ensure` row: the engine's certificate holds one position per stored product state, and the checker returns `Accepted`. | Test (TC-556) |
| FR-149-AC-2 | The same certificate with the `Taken` of the `commitA` edge from `(0, 0, t, 0, f)` changed to `Stutter` returns `Rejected` with rule `PostDiffers` at that position and edge; with that edge's post index pointing at the initial position, rule `PostDiffers`; with one listed edge removed, rule `SuccessorsDiffer` at that position. Each settles `inconclusive`, `CertificateRejected{rule, at}`. | Test (TC-556) |
| FR-149-AC-3 | A certificate built for the lost-update model by listing its states with the `commitB` edge from `(1, 0, f, 0, t)` returns `Rejected` with rule `StepFails` and verdict `AbstractStepRejected{transition: inc(c), cause: Postcondition}` in its locus, so a false relation is never accepted. | Test (TC-556) |
| FR-149-AC-4 | The AC-1 certificate checked with `max_transitions` 1 returns `Stopped(LimitReached{MaxTransitions, 1})`; a certificate naming another refinement node is refused `stale_dependency`/`content-mismatch`. Checking the AC-1 certificate twice gives equal results. | Test (TC-556) |
| FR-149-AC-5 | A certificate for `HeavyEnq` (FR-141-AC-9), built under a meter larger than B so that `HeavyEnq` holds and the checker accepts it there, checked under the meter B returns `Stopped(LimitReached{EvaluationMeter, B})` whether the meter ran out in a mapping row or in a concrete contract, not `Rejected`. | Test (TC-556) |
| FR-149-AC-6 | In a certificate for a model with a terminal state, that state's position lists one edge whose `transition` is `None` and which returns to the position; the checker accepts it, and the same certificate with that edge removed returns `Rejected` with rule `SuccessorsDiffer` at the position. | Test (TC-556) |
| FR-149-AC-7 | A certificate whose `successors` list is one shorter than `positions`, one whose `initial` omits a concrete initial state, and one with a position no edge and no initial state reaches each return `Rejected` with rule `Malformed`, before any recomputation; a position at which a concrete contract is undecided for a reason other than the meter returns `Rejected` with rule `BadState`. | Test (TC-556) |
| FR-149-AC-8 | The terminal record of AC-3's rejection writes the locus `{"position": "<n>", "edge": "<n>", "verdict": {"kind": "abstract-step-rejected", "transition": ..., "cause": "Postcondition"}}`, and writes `"verdict": null` for AC-2's `PostDiffers`. | Test (TC-556) |

## Dependencies

- ADR-020 §6a CT-1 to CT-4, RS-1; ADR-018 PC-1, PC-2, PC-5 and LA-3;
  ADR-029 CB-2 and RU-2.
- [FR-338](FR-338-check-an-en-1-closure-certificate.md) and
  [FR-339](FR-339-check-an-en-1-component-certificate.md), EN-1's closure
  and component checkers, beside which this checker sits in `qsl-replay`.
- [FR-141](FR-141-decide-one-concrete-step-against-the-abstract-model.md)
  (`check_initial`, `check_step`),
  [FR-142](FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md)
  (the product that writes the certificate),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (`ModelSystem`), [FR-098](FR-098-execute-a-replay-request.md) (recompile),
  [FR-144](FR-144-request-and-settle-a-refinement-item.md) (settlement).
- ADR-018 PC-1 and PC-2 give the certification label and the
  `CertificateRejected` cause; QSpec carries them on the terminal record
  (ADR-020 QS-12).

## References

- ADR-020. QSpec half: QSpec FR-379 (Linear STD-133; ADR-020 QS-12). The
  certified-proof ruling is recorded on Linear QSL-390.
