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
    Stopped(IncompleteCause),              // LimitReached{limit, value}, Cancelled{source} or ResourceExhausted
}
```

`Stopped` has the shape of FR-338's `TemporalCertificateCheck::Stopped`. A
reached limit is `IncompleteCause::LimitReached{limit, value}` (FR-126's
`ReachedLimit`), a cancellation is `IncompleteCause::Cancelled{source}`, which
names no limit, and an FR-120 expansion stop that names no limit and is not an
internal failure is `IncompleteCause::ResourceExhausted`. An internal failure
is a fault of the checker, not a `Stopped` and not a rejection.

The checker uses FR-338's `CertificateRejection{rule, at}`. It uses FR-338's
`BadState` and FR-163's `Malformed`, and FR-338's `CertificateRule` gains
`InitialMismatch`, `SuccessorsDiffer`, `StepFails` and `PostDiffers`. FR-338's
`CertificateLocus` gains
`SimulationStep { position: u64, edge: Option<u64>, verdict: Option<FailedStep> }`:
the certificate position, the edge when the rule concerns one, and the failing
`check_step` verdict for `StepFails`.

`FailedStep` is `check_step`'s verdict (FR-141) restricted to the three
alternatives that judge the relation: `Fails(RefinementFailure)`,
`Undefined(UndefinedEvaluation)` and `Undetermined`. It excludes `Passes`,
`Stopped` and `Fault`, which are never a rejection. A step's `Undetermined`
carries the evaluation outcome that caused it, which is either a refusal or an
incomplete evaluation naming the limit reached (the work units of the
evaluation meter, for a meter that ran out), together with FR-141's
`MappingUndetermined` or `UndecidedSuccessor` cause. The checker classifies a
read as meter-only exactly when that outcome is an incomplete evaluation whose
limit is the evaluation meter.

**Wire form.** The terminal record writes the locus as `{"position": "<n>",
"edge": "<n>" or null, "verdict": <verdict> or null}`, with unsigned integers
as decimal strings; `verdict` is `null` for every rule but `StepFails`. A
verdict is an object whose `kind` is one of QSpec FR-379's `refinement_failure`
names, plus `UndefinedEvaluation` and `Undetermined`, and whose other members are the verdict's own:

| `kind` | Members |
| --- | --- |
| `InitialNotAbstract` | `initial` (decimal string, the concrete initial state's index) |
| `StutterChanged` | `position` |
| `AbstractStepRejected` | `position`, `transition` (the transition identity encoding of QSpec FR-364, from FR-181), `cause` |
| `NoAbstractMatch` | `position` |
| `UndefinedEvaluation` | `where` and `cause` (FR-127's `UndefinedEvaluation` encoding), the step verdict `Undefined`; QSpec FR-379's `Undefined{position, row}` is the same failure as the engine writes it, and `check_step` never returns that form |
| `Undetermined` | `cause` (`MappingUndetermined` or `UndecidedSuccessor`) and `evaluation` (`{"outcome": "refused"}` or `{"outcome": "incomplete", "limit": <limit spelling>}`, where the limit spelling is `evaluation-meter` for the evaluation meter (FR-127's model-check limit spelling) and quire-exact's `LimitKind` name for another counter of the evaluation, in snake case such as `integer_bits`) |

`position` inside a verdict is the refinement position FR-141 names and need
not equal the locus `position`. A `cause` is `{"kind": "Precondition",
"clause": <clause name>}`, `{"kind": "Frame", "code": <frame code spelling>}` or
`{"kind": "Postcondition"}`.

## Behavior

- **Identity.** The checker SHALL refuse with `stale_dependency`/
  `content-mismatch`, naming both values, a certificate whose refinement
  node differs from the item's.
- **Package identity.** FR-144 "Certificate identity" owns the refusal of a
  certificate whose `concrete_package`, `abstract_package` or `obligation`
  differs from the item's recompile; the checker reads the refinement node only.
- **Shape.** Before any recomputation, the checker SHALL reject with rule
  `Malformed`, at position `min(len(successors), len(positions))`, a
  certificate whose `successors` and `positions` differ in length.
- **Initial count.** Before any recomputation, the checker SHALL reject with
  rule `Malformed`, at position `len(initial)`, a certificate whose `initial`
  and the concrete model's initial states differ in count.
- **Unreached position.** Before any recomputation, the checker SHALL reject
  with rule `Malformed`, at the first such position, a certificate with a
  position that no initial state reaches and no edge listed at a lower-indexed
  position reaches.
- **Initial states.** For each concrete initial state `i`, the checker SHALL
  run `check_initial`, and SHALL reject with rule `InitialMismatch` at
  `SimulationStep{position: initial[i], edge: None}` unless it passes with that
  position or Meter stops applies.
- **Undecided state.** For each position, when a contract of the concrete
  model is undefined or refused there, so its successors are not known, the checker SHALL reject with rule `BadState` at
  that position.
- **Successors.** For each position, the checker SHALL compute the concrete
  successors with FR-120's `ModelSystem` and SHALL reject with rule
  `SuccessorsDiffer` at that position unless they are exactly the listed edges'
  transitions, in canonical order. A position with no concrete successor lists
  one edge whose `transition` is `None`, the terminal stutter edge of ADR-018
  SM-4. Post-states are judged by the step rule, not by this one.
- **Steps.** For each edge, the checker SHALL run `check_step` (FR-141) from
  the position and SHALL reject with rule `StepFails`, carrying the verdict,
  when it returns `Fails`, `Undefined`, or an `Undetermined` that Meter stops
  does not cover.
- **Post.** For each edge whose step passes, the checker SHALL reject with rule
  `PostDiffers`, at that position and edge, unless the step's post position and
  `Taken` equal the edge's.
- **Meter stops.** When the verdict of `check_initial` or `check_step` is an
  `Undetermined` that is meter-only, the checker SHALL stop with
  `Stopped(LimitReached{EvaluationMeter, value})`, with `value` the evaluation
  meter's work units, and not reject: a certificate is rejected only when the
  checker found it wrong (ADR-018 PC-2).
- **History.** The checker and the producer SHALL identify a position by its
  concrete state key, the
  typed canonical key of its history values (FR-138's `HistoryValues`, in the
  product key's order) and its sorted abstract state keys. Wherever the checker
  compares a listed position with a position it computed, at an initial state
  and at an edge's post position, it SHALL compare all three, so a certificate
  that lists two product states which differ only in history as one position is
  rejected by the rule of that comparison (`InitialMismatch` or `PostDiffers`)
  and never accepted. A refinement without history rows has the empty history
  key at every position. The producer SHALL write each stored product state's
  history key into its position.
- **Order.** The checker SHALL apply its rules in this order and report the
  first outcome: identity; the shape, initial-count and unreached-position
  rules; the `max_states` stop; the initial states in index order; then each
  position in index order, where `BadState` precedes `SuccessorsDiffer`, which
  precedes each edge's `max_transitions` stop, `StepFails` and `PostDiffers` in
  edge order.
- **Stops.** A stop SHALL end the check at the point it is reached, before any
  later rejection, and a stop SHALL NOT turn into a rejection.
- **Count of positions.** The checker SHALL count positions against
  `max_states`, with checked arithmetic.
- **Count of edges.** The checker SHALL count edges against `max_transitions`,
  with checked arithmetic.
- **Limit stop.** The checker SHALL stop at a reached limit with
  `Stopped(LimitReached{limit, value})`.
- **Step stop.** When a `check_step` it calls returns `StepVerdict::Stopped`, the
  checker SHALL end the check with `Stopped(LimitReached{limit, value})` naming
  that limit.
- **Named expansion stop.** When an FR-120 expansion of a position stops at a
  named limit, the checker SHALL end the check with
  `Stopped(LimitReached{limit, value})`. A concrete contract that is
  incomplete stops the expansion this way, because FR-120 turns every
  incomplete contract into an expansion stop and never into a finding; FR-120
  labels each such stop, the counter limits of the exact kernel included, with
  limit `EvaluationMeter`.
- **Unnamed expansion stop.** When an FR-120 expansion stops naming no limit
  and is not an internal failure, the checker SHALL end the check with
  `Stopped(ResourceExhausted)`.
- **Cancellation.** The checker SHALL end a cancelled check with
  `Stopped(Cancelled{source})`.
- **Purity.** The checker SHALL read no path, environment variable, clock
  or search location, and its result SHALL be a function of the
  certificate, the byte provision and the limits.
- **Production.** When FR-142's safety half returns `Holds`, the layer-A
  engine SHALL write the certificate from its retained product: every stored
  product position, each concrete initial state's position, and every
  retained edge with the `Taken` that `check_step` returned.
- **Settlement.** FR-144 SHALL settle the safety half `proved` only on
  `Accepted`, V-6 `CertificateRejected{rule, at}` (ADR-018 PC-2) on
  `Rejected`, and V-7 on `Stopped` (FR-144 "Certificate").

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-149-AC-1 | ADR-020 §8's `CasRefinesCounter` without its `ensure` row: the engine's certificate holds one position per stored product state, and the checker returns `Accepted`. | Test (TC-556) |
| FR-149-AC-2 | The same certificate with the `Taken` of the `commitA` edge from `(0, 0, t, 0, f)` changed to `Stutter` returns `Rejected` with rule `PostDiffers` at that position and edge; with that edge's post index pointing at the initial position, rule `PostDiffers`; with one listed edge removed, rule `SuccessorsDiffer` at that position. Each settles `inconclusive`, `CertificateRejected{rule, at}`. | Test (TC-556) |
| FR-149-AC-3 | A certificate built for the lost-update model by listing its states with the `commitB` edge from `(1, 0, f, 0, t)` returns `Rejected` with rule `StepFails` and verdict `AbstractStepRejected{transition: inc(c), cause: Postcondition}` in its locus, so a false relation is never accepted. | Test (TC-556) |
| FR-149-AC-4 | The AC-1 certificate checked with `max_transitions` 1 returns `Stopped(LimitReached{MaxTransitions, 1})`; a certificate naming another refinement node is refused `stale_dependency`/`content-mismatch`. Checking the AC-1 certificate twice gives equal results. | Test (TC-556) |
| FR-149-AC-5 | A valid certificate is never rejected for a meter it ran out of, in each of four places, each returning `Stopped(LimitReached{EvaluationMeter, B})`, not `Rejected`: (a) the unmetered certificate of the light ring fixture (`RingIsQueue`) checked under a meter B that runs out in the initial states' mapping rows; (b) the same certificate under a larger B that runs out in a step's mapping row; (c) a certificate of a model whose concrete contract costs more than B, which stops the position's successor expansion; (d) a `HeavyEnq` certificate (FR-141-AC-9), built under a larger meter, checked under B, where the abstract side runs out. | Test (TC-556) |
| FR-149-AC-6 | In a certificate for a model whose chain of enabled operations ends in a terminal state (`CasRefinesCounter` without its `ensure` row, with `peek` and the second thread's `begin` given preconditions that are never enabled, so the states form a chain that ends in a terminal state), the terminal state's position lists one edge whose `transition` is `None` and which returns to the position; the checker accepts it, and the same certificate with that edge removed returns `Rejected` with rule `SuccessorsDiffer` at the position. | Test (TC-556) |
| FR-149-AC-7 | A certificate whose `successors` list is one shorter than `positions` is rejected `Malformed` at the shorter length; one whose `initial` omits a concrete initial state, at `len(initial)`; one with a position that only an edge listed at a higher-indexed position reaches, at that position; each before any recomputation. A position at which a concrete contract is undefined or refused (not out of meter) returns `Rejected` with rule `BadState`; the fixture is a concrete model with a precondition `6 mod (2 - self.value) >= 0`, which is undefined at value 2. | Test (TC-556) |
| FR-149-AC-8 | The terminal record of AC-3's rejection writes the locus `{"position": "<n>", "edge": "<n>", "verdict": {"kind": "AbstractStepRejected", "position": "<n>", "transition": <ModelTransition encoding of inc(c)>, "cause": {"kind": "Postcondition"}}}`, writes `"verdict": null` for AC-2's `PostDiffers`, and writes each other kind's members as the wire-form table lists them. | Test (TC-556) |
| FR-149-AC-9 | FR-144 refuses `stale_dependency`/`content-mismatch`, before the checker runs, a certificate whose `concrete_package` differs from the recompile, and one whose `obligation` differs; the checker is not called in either case. | Test (TC-556) |
| FR-149-AC-10 | A check cancelled between positions returns `Stopped(Cancelled{source})`, and an FR-120 expansion stop that names no limit, produced by a concrete model whose state constraint has a family that evaluation refuses, returns `Stopped(ResourceExhausted)`; FR-144 settles each V-7. | Test (TC-556) |
| FR-149-AC-11 | Over `RegisterHistory` with a history field that makes two stored product states differ only in history, the engine's certificate carries the history key in each position and the checker returns `Accepted`; the same certificate with the second state's position given the first state's history key returns `Rejected` with rule `PostDiffers` (or `InitialMismatch` when the position is an initial one), and never `Malformed`. | Test (TC-556) |

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

- ADR-020. QSpec half: QSpec FR-379 (Linear STD-133; ADR-020 QS-12), whose
  `refinement_failure` kind names the wire form above reuses; the
  `SimulationStep` locus and the `Undetermined` verdict are not yet stated on
  the QSpec half (FR-331's `at` for this checker). The
  certified-proof ruling is recorded on Linear QSL-390.
