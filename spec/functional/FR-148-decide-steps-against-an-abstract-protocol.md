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
  - target: ix://agent-ix/quire-spec-language/FR-145
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-147
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
- The abstract protocol subject's `ProtocolSystem` (ADR-027 TS-1), built
  with the derived universes (ADR-020 RM-2) and the request's bindings.
- FR-141's positions and steps.

## Outputs

- FR-141's `StepVerdict`, with `abstract_states` a set of abstract protocol
  states (model state with protocol state) sorted by ADR-027 PS-8 key.
- The failures FR-141 names, and for QSpec FR-177's further checks the
  `RefinementFailure` kinds QSpec gives them (ADR-020 QS-6, QS-8).

## Behavior

### Closure

- The **internal closure** of a set SHALL be the set together with every
  abstract protocol state reachable from a member by abstract steps whose
  class an `internal` row names. Each closure step computed SHALL count
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
  that node whose binder record equals the row's expressions.
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

- The product (FR-142) SHALL apply QSpec FR-177's divergence-freedom,
  refusal-set, terminal-success and assumption-weakening checks to the same
  product states and edges, as QSpec states them, and report a violation
  with the failure kind QSpec gives it, as a prefix or lasso over concrete
  steps.
- Replay (FR-145) SHALL recompute the abstract sets with `ProtocolSystem`
  and rerun the same check at the failing position or over the loop.

## Acceptance Criteria

Fixtures: `Twice` and `CasTwice` (FR-147).

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-148-AC-1 | `check_initial` on `CasTwice`'s concrete initial state gives one abstract state, at rest at `a1` with `value` 0. After `beginA`, `commitA` the set holds the state at rest at `a2` with `value` 1; after a second commit, the state with `value` 2 and the protocol finished, since the closure takes the internal `finish`. | Test (TC-553) |
| FR-148-AC-2 | `CasTwice` settles `proved`, `closed-scope` over universe `counters = {c}`. | Test (TC-553) |
| FR-148-AC-3 | The lost-update model (ADR-020 §8) with `peek` removed, against `Twice` with the same rows, settles `refuted` with `NoAbstractMatch` at the commit that leaves `value` unchanged, and the counterexample replays to `reproduced-with-evaluated-witness`. | Test (TC-553) |
| FR-148-AC-4 | `CasTwice` against the protocol `Once` (`a1` then `finish`) settles `refuted` with `NoAbstractMatch{position: 4}`, whose step is the second commit, and the counterexample replays. | Test (TC-553) |
| FR-148-AC-5 | `CasTwice` with `peek` kept in the concrete model and mapped `-> stutter` settles `refuted` by QSpec FR-177's divergence-freedom check, with a lasso whose loop is one `peek` step at the initial state, where the abstract `a1` is enabled, and with no `assume` or `ensure` row present. | Test (TC-553) |

## Dependencies

- ADR-020 §4 AX-2, §6 RC-1 to RC-3, §7 MC-1 (as amended by ADR-027); ADR-027
  FO-3, TS-1, TS-3, PS-8, PR-2 and PR-3.
- [FR-141](FR-141-decide-one-concrete-step-against-the-abstract-model.md),
  [FR-142](FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md),
  [FR-145](FR-145-replay-a-refinement-counterexample.md),
  [FR-147](FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md).

## References

- The QSpec half (QSpec FR-177's relation and its further checks, QS-8):
  Linear STD-133.
