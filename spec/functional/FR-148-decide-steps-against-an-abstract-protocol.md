---
id: FR-148
title: "Decide concrete steps against an abstract protocol"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-018
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-141
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-142
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-137
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-145
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-177
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-379
    type: depends_on
---
# FR-148: Decide concrete steps against an abstract protocol

## Description

For a refinement whose abstract side is a protocol subject (FR-147),
`check_initial` and `check_step` (FR-141) SHALL track the set of abstract
protocol states consistent with the behaviour so far, closed under the
abstract internal steps, and match each visible concrete step to an
abstract step with its observation label (ADR-027 PR-2, PR-3; ADR-020 AX-2,
MC-1). The abstract side's steps come from `ProtocolSystem` (ADR-027 TS-1)
through the one application function `ModelSystem` uses (TS-3). The same
product (FR-142) and replay (FR-145) run QSpec FR-177's further checks
(divergence-freedom, refusal sets, terminal success and assumption
weakening) as QSpec states them (ADR-020 QS-8).

## Use case

The model checker explores a concrete compare-and-set model against an
abstract protocol that increments twice. Each commit must be one of the
protocol's increments, in order; the other concrete steps are internal. A
concrete model that commits without incrementing, or that commits more
often than the protocol allows, is refuted with a prefix that replays.

## Inputs

- A `CheckedRefinement` with `AbstractSide::Protocol` (FR-147).
- The abstract protocol subject's `ProtocolSystem` (ADR-027 TS-1), passed to
  FR-141 as `AbstractSystem::Protocol`, built
  with the derived universes (ADR-020 RM-2) and the request's bindings, from
  the checked package that holds the clause: the declaring package, or the
  dependency's `CheckedPackage` that `AbstractSide::Protocol::dependency`
  names (FR-147).
- FR-141's positions and steps.

## Outputs

- FR-141's `StepVerdict`, with `abstract_states` a set of abstract protocol
  states (model state with protocol state) sorted by ADR-027 PS-8 key.
- The failures FR-141 names and the QSpec FR-379 `RefinementFailure` kinds
  for protocol refusal and terminal-success failures (ADR-020 QS-6, QS-8).
- A typed product-level assumption-relation result. A satisfied relation
  contributes no failure. A relation that cannot be established returns
  `ProtocolAssumptionNotEstablished`, never a QSpec `RefinementFailure`.
  When it carries a counterexample witness, the witness is a paired concrete
  and abstract protocol prefix/lasso with the exact checked assumption
  identities and replay data defined below.

`AssumptionIdentity` is typed data, not a display label. It carries `side`
(`implementation` or `specification`), `origin` (`authored`, `scheduler` or
`memory`), fairness `kind` (`weak` or `strong`), `granularity` (`whole` or
`each`), the exact source-checked target identity, and the selected
operation/receiver/argument identity when the selected member form applies.
`ProtocolAssumptionWitness` carries a `reason`, the exact implementation
identity and specification identity or identities, and either (a) for
`strengthened`, a concrete prefix and loop plus the corresponding abstract
protocol prefix and loop, or (b) for `relation-undetermined`, those paired
paths when a path has been evaluated. The abstract path SHALL be a member of
the product candidate relation at every paired position; both loops SHALL
close at their respective entry states.

## Behavior

### Closure

- The **internal closure** of a set SHALL be the set together with every
  abstract protocol state reachable from a member by abstract steps whose
  class the refinement's internal list names (`abstract_internal`). Each closure step computed SHALL count
  toward FR-101's `max_transitions`.

### Initial states (PR-3)

- `check_initial` SHALL pair each concrete initial state with the abstract
  initial protocol states (ADR-027 FO-3, the root settled from `run`) whose
  visible fields equal `map(c0, h0)`, take the internal closure, and fail
  `InitialNotAbstract` when the result is empty.

### Steps

- A concrete step whose row targets an abstract operation or node SHALL be
  **visible** with that target's observation label: for an operation, any
  `attempt` or `cattempt` step of that operation with the row's receiver
  and arguments (and the concrete result when bound); for a node, a step of
  that node whose binder record agrees with the row's expressions on every
  field the row gives.
- A visible step SHALL pass when some member of the pre set has an enabled
  abstract step with its label whose post-state's visible fields equal the
  post mapped state. The post set SHALL be the internal closure of all such
  post-states; an empty post set SHALL fail `NoAbstractMatch{position}`.
- A `stutter` step SHALL be **internal**: it SHALL pass when the post mapped
  state's visible fields equal the pre's, leaving the set unchanged, and
  fail `StutterChanged{position}` otherwise.
- An `any` step SHALL pass as internal or as visible with the label of any
  abstract step enabled from the set, with the union of both post sets.

### Further checks

- **Divergence-freedom.** The product (FR-142) SHALL decide QSpec FR-177's
  divergence-freedom on the safety product itself, with no fairness filter:
  a cycle of concrete steps whose rows are `stutter`, at every position of
  which every member of the candidate set enables a visible abstract step,
  is an observable divergence. The first such cycle in canonical order
  SHALL refute with a lasso and `RefinementFailure::Divergence` naming that
  abstract step class (QSpec FR-379). This check reads no `F_A`, so it runs
  with hidden abstract fields; FR-147's `unsupported` liveness half is the
  `F_A` half only.
- **Refusal sets and terminal success.** The product SHALL apply QSpec
  FR-177's refusal-set and terminal-success checks to each reachable product
  state whose incoming step rules passed. At each state it SHALL first check
  `RefusalNotPermitted`, then `TerminalSuccessMismatch`, before checking
  liveness failures. A refusal-set violation SHALL report QSpec FR-379's
  `RefusalNotPermitted{position, label}`, where `label` is the full typed
  `ObservationLabel`; a terminal-success violation SHALL report
  `TerminalSuccessMismatch{position, direction}`. The position, stable-state
  predicates, canonical label selection, failure precedence and replay
  behavior SHALL be exactly those in QSpec FR-379. Both are finite-prefix
  safety failures over concrete steps. A model abstract subject SHALL NOT
  report either protocol-only kind.
- **Assumption weakening.** For each reachable product behaviour, the
  product SHALL treat the checked concrete `assume` constraints as the
  implementation environment assumption `I`, and the checked abstract
  protocol's authored fairness, derived scheduler fairness and memory
  fairness constraints as the specification environment assumption `S`.
  It SHALL NOT include `ensure` constraints in `I` or `S`: those are
  guarantees, not environment assumptions (FR-137; ADR-020 RM-6). The
  product SHALL prove the implication `S => I` over the paired behaviour
  space: every concrete path that has a corresponding abstract protocol
  path satisfying all of `S` SHALL satisfy every constraint in `I` under
  the concrete model's own enabledness and taken-step semantics. Thus the
  implementation assumption is no stronger than the specification's.
  An empty `I` passes, and an empty `S` is the unconstrained assumption. It
  SHALL evaluate weak and strong fairness and `each` and `whole` granularity
  by their source-side class semantics (ADR-018 FA-1 to FA-4; ADR-027 PA-1 to
  PA-3, SE-1). It SHALL compare each constraint using that source-side
  enabledness and taken-step semantics; it SHALL NOT equate unlike operation,
  scheduler or memory target identities by name or spelling.

  `ProtocolAssumptionNotEstablished` is a QSL product outcome, not a
  `RefinementFailure` and not a QSpec FR-379 counterexample kind. It SHALL
  carry a typed reason (`strengthened` or `relation-undetermined`) and the
  involved `AssumptionIdentity` values. Each identity SHALL include its side
  (`implementation` or `specification`), origin (`authored`, `scheduler` or
  `memory`), fairness kind (`weak` or `strong`), granularity (`whole` or
  `each`), exact checked target identity, and, for `each`, the selected
  operation/receiver/argument identity when that member form applies. The
  identity SHALL preserve the source's typed checked identity; display names
  are not identity.

  When a reachable paired lasso satisfies every constraint in `S` and
  violates a named constraint in `I`, the result SHALL use reason
  `strengthened` and carry that exact implementation identity plus the
  paired concrete and abstract prefix/lasso. This witness is an explicit
  exception to the QSpec FR-379 liveness-counterexample rule requiring a
  lasso fair under concrete `F_C`: it necessarily violates the named member
  of `I`. Because QSpec FR-379 expressly does not define an assumption
  failure kind, this witness SHALL settle the QSL item `inconclusive`,
  `unsettled`, with the typed `ProtocolAssumptionNotEstablished` cause; it
  SHALL NOT settle `refuted` or `proved`. If incomparable source-class
  identities prevent the implication from being established and no violating
  paired lasso is available, the result SHALL use reason
  `relation-undetermined` and carry both exact typed identities. A configured
  product limit that prevents deciding the relation SHALL instead settle
  `incomplete` with its limit cause. Neither outcome SHALL settle proved.
  Replay (FR-145) SHALL recompile both admitted subjects, reconstruct the
  paired prefix/lasso and recompute both sides' predicates and identities;
  it SHALL reproduce the typed outcome only when the same reason, identities
  and (when present) both paths agree.
- A `proved` protocol refinement SHALL be labelled `Uncertified` (ADR-018
  PC-1, ADR-020 CT-4): FR-149's simulation-certificate checker covers the
  model-subject step relation and does not certify these protocol-subject
  step decisions; the further checks also have no core certificate checker.
- Replay (FR-145) SHALL recompute the abstract sets with `ProtocolSystem`
  and rerun the same check at the failing position or over the loop.

## Acceptance Criteria

Fixtures: `Twice` and `CasTwice` (FR-147).

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-148-AC-1 | `check_initial` on `CasTwice`'s concrete initial state gives one abstract state, at rest at `a1` with `value` 0. After `beginA`, `commitA` the set holds the state at rest at `a2` with `value` 1; after a second commit, the state with `value` 2 and the protocol finished, since the closure takes the internal `finish`. | Test (TC-553) |
| FR-148-AC-2 | `CasTwice` settles `proved`, basis `closed-scope`, as QSpec FR-379 settles a completed explicit-state run with no failure, holding for the concrete initial states and universe `counters = {c}`, labelled `Uncertified` (ADR-018 PC-1), since FR-149 does not certify protocol-subject step decisions and the further checks of QSpec FR-177 have no core certificate checker (ADR-020 CT-4). | Test (TC-553) |
| FR-148-AC-3 | The lost-update model (ADR-020 §8) with `peek` removed, against `Twice` with the same rows, settles `refuted` with `NoAbstractMatch` at the commit that leaves `value` unchanged, and the counterexample replays to `reproduced-with-evaluated-witness`. | Test (TC-553) |
| FR-148-AC-4 | `CasTwice` against the protocol `Once` (`a1` then `finish`) settles `refuted` with `NoAbstractMatch{position: 4}`, whose step is the second commit, and the counterexample replays. | Test (TC-553) |
| FR-148-AC-5 | `CasTwice` with `peek` kept in the concrete model and mapped `-> stutter` settles `refuted`, basis `decisive-counterexample`, by QSpec FR-177's divergence-freedom check with divergence observable, with a lasso whose loop is one `peek` step at the initial state, where the abstract `a1` is enabled. It refutes with no `assume` or `ensure` row present, because QSpec FR-177-AC-6 refutes an observable internal loop under every fairness set, including an empty one. | Test (TC-553) |
| FR-148-AC-6 | When the abstract protocol has one thread whose only enabled node maps to concrete operation `inc`, its derived weak whole scheduler constraint implies the matching concrete weak whole `assume inc` over every reachable paired lasso; the assumption-weakening check passes. | Test (TC-553) |
| FR-148-AC-7 | When the abstract protocol declares adversarial scheduling and has no memory or authored fairness constraint, while the concrete model declares weak whole fairness for an operation that remains enabled on an infinite stutter loop, the loop satisfies empty `S` and violates `I`; the check returns `inconclusive`, `unsettled`, with `ProtocolAssumptionNotEstablished{reason: strengthened}` and the exact identity plus paired path, and replay reproduces the same predicates. It is not a QSpec `RefinementFailure`. | Test (TC-553) |
| FR-148-AC-8 | With concrete `assume` constraints on two distinct operation classes and an abstract protocol fairness constraint on only one class, a reachable lasso that satisfies the protocol constraint but violates the other concrete assumption yields `ProtocolAssumptionNotEstablished{reason: strengthened}` with the involved incomparable typed identities and paired paths; when a product limit prevents deciding the relation, settlement is incomplete and never proved. | Test (TC-553) |
| FR-148-AC-9 | For a single finite transition class, a strong each constraint on the specification side implies a weak whole constraint for the same operation on the implementation side; a weak specification constraint does not imply a strong implementation constraint on a lasso where the operation is enabled infinitely often but not continuously and is never taken. The first relation passes; the second returns the typed non-proof outcome with a replayable paired witness. | Test (TC-553) |
| FR-148-AC-10 | For an abstract `tso` protocol with `scheduling adversarial`, the specification assumption set includes the derived `fair weak each flush(thread)` memory constraint. A reachable paired lasso that cycles a buffered `tick` write and its flush, never takes the continuously enabled `inc`, satisfies the memory fairness constraint but violates the concrete `assume fair weak whole inc`; the product returns `ProtocolAssumptionNotEstablished{reason: strengthened}` with exact identities and paired paths, and replay confirms flush fairness and the violated `inc` constraint. | Test (TC-553) |
| FR-148-AC-11 | At each reachable product state whose incoming step rules passed, a refusal violation reports `RefusalNotPermitted{position, label}` with QSpec FR-379's canonical full `ObservationLabel`; a terminal-success mismatch reports `TerminalSuccessMismatch{position, direction}`. The first failing step rule takes precedence, then refusal precedes terminal success, and replay recomputes each recorded condition; model abstract subjects report neither kind. | Test (TC-553) |

## Dependencies

- ADR-018 FA-1 to FA-4; ADR-020 §1 RM-6, §4 AX-2, §6 RC-1 to RC-3, §7
  MC-1 and RS-14 (as amended by ADR-027); ADR-027 FO-3, PA-1 to PA-3,
  SE-1, TS-1, TS-3, PS-8, PR-2 and PR-3.
- [FR-137](FR-137-check-a-refinement-s-fairness-rows.md) supplies the checked
  concrete `assume` constraints.
- [QSpec FR-177](ix://agent-ix/quire-specification/FR-177) owns the protocol
  refinement relation; [QSpec FR-379](ix://agent-ix/quire-specification/FR-379)
  owns its typed failure and replay payloads.
- [FR-141](FR-141-decide-one-concrete-step-against-the-abstract-model.md),
  [FR-142](FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md),
  [FR-145](FR-145-replay-a-refinement-counterexample.md),
  [FR-147](FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md).

## References

- QSpec FR-177 (the relation over QSpec FR-377's step rules, and its
  further checks, QS-8); Linear STD-133.
