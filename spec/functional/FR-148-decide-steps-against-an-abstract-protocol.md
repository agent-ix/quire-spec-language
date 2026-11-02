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
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-383
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
- The selected abstract protocol clause's admitted `requirements` set and
  the exact `CheckedTemporalClause` each member identifies in its admitted
  package (FR-218, FR-337, FR-383). The authored part of specification
  assumption `S` comes only from the operation-class constraints in each
  directly required clause's `CheckedTemporalClause::fairness()`. Each source
  identity includes the required clause's `NodeKey` and the constraint's
  checked `FairnessOperation` identity, kind and granularity. It excludes
  `ensure` rows, clauses not directly required by the selected protocol, and
  protocol-target fairness rows, which FR-337 refuses to admit. If a
  requirement member cannot resolve to one exact admitted temporal clause,
  or an operation-class fairness row cannot resolve to one exact checked
  `FairnessOperation`, the relation is `relation-undetermined`; no source is
  inferred from a clause or target name. A resolved clause with no fairness
  rows contributes no authored constraint. With no directly required temporal
  clause, the authored part of `S` is empty.
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
For an authored specification constraint, it also carries the required
temporal clause's `NodeKey`; its target identity is the exact checked
`FairnessOperation` from that clause. Derived scheduler and memory identities
retain their existing source-specific identities and have no temporal clause
key.
`ProtocolAssumptionWitness` carries a `reason`, the exact implementation
identity and specification identity or identities, and either (a) for
`strengthened`, a concrete prefix and loop plus the corresponding abstract
protocol prefix and loop, or (b) for `relation-undetermined`, those paired
finite prefixes ending at the first paired product state whose fairness
enabledness is undetermined. The latter reason is reserved for an undecided
source-side fairness predicate (for example, `ContractUndetermined` while
determining whether the identified class is enabled); it is not used merely
because target identities differ. A product limit uses the limit-incomplete
outcome, not `relation-undetermined`. The abstract path SHALL be a member of
the product candidate relation at every paired position; loops SHALL close
at their respective entry states.

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
  implementation environment assumption `I`, and the specification
  environment assumption `S` as the union of (a) operation-class fairness
  constraints in `CheckedTemporalClause::fairness()` for only the temporal
  clauses directly named by the selected abstract protocol's checked
  `requires temporal` requirements, (b) the derived scheduler constraints,
  and (c) the derived memory fairness constraints.
  It SHALL NOT include `ensure` constraints in `I` or `S`: those are
  guarantees, not environment assumptions (FR-137; ADR-020 RM-6). The
  authored constraints SHALL retain their exact source clause `NodeKey` and
  checked `FairnessOperation` identity with kind and granularity. The
  checker SHALL NOT include protocol-target fairness rows, which FR-337
  refuses to admit, or any unrelated temporal clause's fairness rows. If a
  checked requirement cannot be resolved to exactly one admitted clause, or
  an operation-class fairness row cannot be resolved to one exact checked
  `FairnessOperation`, the authored relation SHALL be
  `relation-undetermined`; the checker SHALL NOT infer a source by clause or
  target display name. No directly required temporal clause means an empty
  authored part of `S`. The product SHALL prove the implication `S => I` over
  the paired behaviour
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
  operation/receiver/argument identity when that member form applies. An
  authored specification identity SHALL also include its required temporal
  clause `NodeKey`. A relation-undetermined result caused by resolving a
  requirement SHALL retain that exact requirement `NodeKey` and every
  available exact candidate identity; an absent source is represented as an
  empty candidate set, not guessed from a name. The identity SHALL preserve
  the source's typed checked identity; display names are not identity.

  When a reachable paired lasso satisfies every constraint in `S` and
  violates a named constraint in `I`, the result SHALL use reason
  `strengthened` and carry that exact implementation identity plus the
  paired concrete and abstract prefix/lasso. This witness is an explicit
  exception to the QSpec FR-379 liveness-counterexample rule requiring a
  lasso fair under concrete `F_C`: it necessarily violates the named member
  of `I`. Because QSpec FR-379 expressly does not define an assumption
  failure kind, this witness SHALL settle the QSL item `inconclusive`,
  `unsettled`, with the typed `ProtocolAssumptionNotEstablished` cause; it
  SHALL NOT settle `refuted` or `proved`. If the enabledness or taken-step
  predicate for an identified source fairness class is
  `ContractUndetermined` at a reachable paired product state and no
  violating paired lasso is available, the result SHALL use reason
  `relation-undetermined` and carry both exact typed identities plus the
  concrete and abstract finite prefixes to that state. Replay SHALL recompute
  the same undecided predicate at the final paired state. A configured
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
| FR-148-AC-6 | When the abstract protocol has one thread whose only enabled node maps to concrete operation `inc`, its derived weak whole scheduler constraint implies the matching concrete weak whole `assume inc` over every reachable paired lasso; the assumption-weakening check passes. Separately, a directly required temporal clause with `fair weak whole inc` contributes exactly its checked operation-class row, identified by the clause `NodeKey` and the row's `FairnessOperation`; an unrelated temporal clause's fairness row contributes nothing. | Test (TC-553) |
| FR-148-AC-7 | When the abstract protocol declares adversarial scheduling, has no memory constraint and has no directly required temporal clause, while the concrete model declares weak whole fairness for an operation that remains enabled on an infinite stutter loop, the loop satisfies empty `S` and violates `I`; the check returns `inconclusive`, `unsettled`, with `ProtocolAssumptionNotEstablished{reason: strengthened}` and the exact identity plus paired path, and replay reproduces the same predicates. An unrelated temporal clause's fairness row is not in `S`. It is not a QSpec `RefinementFailure`. | Test (TC-553) |
| FR-148-AC-8 | With concrete `assume` constraints on two distinct operation classes and an abstract protocol that directly requires only a temporal clause carrying `fair weak whole left`, a reachable lasso that satisfies this checked row but violates the `right` concrete assumption yields `ProtocolAssumptionNotEstablished{reason: strengthened}` with the exact required clause `NodeKey`, checked left `FairnessOperation`, and right implementation identity plus paired paths; an unrelated clause carrying `fair weak whole right` is excluded. When a product limit prevents deciding the relation, settlement is incomplete and never proved. | Test (TC-553) |
| FR-148-AC-9 | For a single finite operation-class domain, the first abstract protocol directly requires a temporal clause carrying `fair strong each inc`, whose exact source identity includes its clause `NodeKey` and checked `FairnessOperation`; this specification constraint implies a weak whole `assume inc` on the implementation side. In a separate protocol requiring a clause carrying `fair weak whole inc`, the specification constraint does not imply strong implementation fairness on a lasso where `inc` is enabled infinitely often but not continuously and is never taken. The first relation passes; the second returns the typed non-proof outcome with a replayable paired witness. | Test (TC-553) |
| FR-148-AC-10 | For an abstract `tso` protocol with `scheduling adversarial` and no directly required temporal fairness clause, the specification assumption set includes the derived `fair weak each flush(thread)` memory constraint. A reachable paired lasso that cycles a buffered `tick` write and its flush, never takes the continuously enabled `inc`, satisfies the memory fairness constraint but violates the concrete `assume fair weak whole inc`; the product returns `ProtocolAssumptionNotEstablished{reason: strengthened}` with exact identities and paired paths, and replay confirms flush fairness and the violated `inc` constraint. | Test (TC-553) |
| FR-148-AC-11 | At each reachable product state whose incoming step rules passed, a refusal violation reports `RefusalNotPermitted{position, label}` with QSpec FR-379's canonical full `ObservationLabel`; a terminal-success mismatch reports `TerminalSuccessMismatch{position, direction}`. The first failing step rule takes precedence, then refusal precedes terminal success, and replay recomputes each recorded condition; model abstract subjects report neither kind. | Test (TC-553) |
| FR-148-AC-12 | When an identified concrete or protocol fairness class has `ContractUndetermined` enabledness at a reachable paired state and no violating paired lasso exists, the product returns `ProtocolAssumptionNotEstablished{reason: relation-undetermined}` with the exact checked identities and both finite prefixes to the undecided state; FR-145 replay reproduces the same undecided predicate and the item settles `inconclusive`, `unsettled`, never `refuted` or `proved`. This is distinct from product-limit `incomplete`. | Test (TC-553) |
| FR-148-AC-13 | If a selected protocol's checked `requirements` member cannot resolve to exactly one admitted temporal-clause node, the product returns `ProtocolAssumptionNotEstablished{reason: relation-undetermined}` with the exact requirement `NodeKey`, every available candidate clause identity, and paired initial prefixes; it settles `inconclusive`, `unsettled`, never `refuted` or `proved`. Replay recomputes the same checked-node resolution and candidates. A clause with no fairness rows is a uniquely resolved source that contributes no authored constraint, and no clause or target is inferred by name. | Test (TC-553) |

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
  [FR-147](FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md),
  [FR-218](FR-218-check-every-protocol-control-construct-at-s3.md) binds
  `requires temporal` members, [FR-337](FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md)
  admits the checked clause's fairness rows, and [FR-383](FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md)
  carries the bound clause node keys in `requirements`.

## References

- QSpec FR-177 (the relation over QSpec FR-377's step rules, and its
  further checks, QS-8); Linear STD-133.
