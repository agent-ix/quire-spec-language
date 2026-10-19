---
id: FR-147
title: "Check a refinement whose abstract side is a protocol subject"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-018
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-144
    type: depends_on
---
# FR-147: Check a refinement whose abstract side is a protocol subject

## Description

The refinement checker (FR-135) SHALL admit a refinement declaration whose
abstract side names a checked protocol clause of the abstract model, which
makes the abstract subject a protocol subject (ADR-027 PS-1, PR-2; ADR-020
RM-9, MC-1). This is QSpec FR-177's protocol refinement. The mapping rows define
the abstract model's visible fields; the abstract protocol's rest points,
binders, queues and instances are hidden. Each concrete step's explicit row
states its observation, and the refinement profile's list of permitted
abstract internal steps (QSpec FR-432) names the abstract step classes that
are internal; every other abstract step class is visible. The relation's meaning, including
divergence-freedom, refusal sets, terminal success and assumption
weakening, is QSpec's (ADR-020 QS-8); this requirement covers QSL's S3
checks and requirement record. FR-148 decides the steps.

## Use case

A specification author has an abstract protocol that performs two
increments and then finishes, and a concrete compare-and-set model. They
map each commit to the abstract increment, each other concrete step to an
internal step, and list the abstract `finish` among the profile's internal
steps. QSL checks the
rows at compile time and requests one protocol refinement claim.

## Inputs

- The parsed refinement form, whose abstract side is written
  `abstract <A>::<protocol>`, with step rows whose right sides may be
  `node <A>::<protocol>::<node>(...)` (QSpec FR-432), and the refinement
  profile's list of abstract internal steps (ADR-020 RM-9, MC-1). The
  declaration carries no `internal` or `visible` row. The abstract-side
  spelling is illustrative until QSpec spells it (ADR-020 QS-11; FR-310).
- The abstract model's checked protocol clause and its step classes
  (ADR-027 ST-1 to ST-15).

## Outputs

- `CheckedRefinement` with `abstract_side: AbstractSide::Protocol{alias,
  clause: NodeKey}` and `abstract_internal: Vec<AbstractStepClass>`.
- Step rows whose `StepTarget::Abstract` may name an abstract protocol node
  as well as an abstract operation.
- A requirement record of kind `refinement` (QSpec FR-290's `protocol`
  family member) for the declaration.

## Behavior

### Abstract side

- When the abstract side names `<A>::<protocol>`, the checker SHALL resolve
  it to a checked protocol clause of the abstract model, refusing
  `missing_declaration`/`missing-name` when none matches.
- Population and object rows SHALL be checked by FR-135 against the
  abstract model's state; the protocol's state SHALL take no row, and a row
  naming a protocol binder, queue or node as a field SHALL refuse
  `invalid_model_binding`/`malformed-declaration`.

### Step rows and observations

- Step rows SHALL cover every concrete step class by FR-136. A right side
  SHALL be an abstract operation application, an abstract protocol node
  with one expression per field of its binder record, `stutter` or `any`.
- A right side naming an abstract node SHALL resolve to an `attempt`,
  `event`, `send`, `receive` or `finish` node of the abstract protocol,
  refusing `missing_declaration`/`missing-name` otherwise; its expressions
  SHALL be typed against the node's binder record, refusing
  `ill_typed`/`type-mismatch`.
- While the abstract side is a model subject, a right side naming a
  protocol node SHALL refuse `invalid_model_binding`/`malformed-declaration`.

### Internal abstract steps

- Each entry of the profile's internal list SHALL name an abstract protocol
  node, or a channel or memory step kind of the abstract protocol, refusing
  `missing_declaration`/`missing-name` otherwise. The resolved entries are
  `abstract_internal`.
- QSL adds a refusal beyond QSpec FR-432: an abstract class named both by
  the profile's internal list and as the right side of a step row SHALL
  refuse
  `invalid_model_binding`/`conflicting-binding`, naming the entry and the
  row.
- An abstract step class that the list does not name SHALL be visible with
  its observation label (QSpec FR-432); no class is refused for being named
  by neither the list nor a step row (ADR-020 MC-1).
- A non-empty internal list while the abstract side is a model subject SHALL
  refuse `invalid_model_binding`/`malformed-declaration`.

### Requirement record

- The `requirements` hook SHALL write one requirement record of kind
  `refinement` for the declaration, in place of FR-144's
  `temporal-satisfaction` record, binding the refinement node, the
  concrete subject and the abstract protocol subject in its obligation
  identity.
- Negotiation SHALL route it by its kind (FR-075); the explicit-state
  engine's provider manifest SHALL advertise `refinement` for a protocol
  abstract side.
- With `F_A` non-empty, the record's `F_A` liveness half SHALL settle
  `unsupported`, `unsupported-requested-capability`, naming liveness through
  hidden abstract fields, since the abstract control state is hidden
  (ADR-020 AX-5).

## Acceptance Criteria

Fixture `Twice`: abstract package `example/counter-spec` (ADR-020 §8) with
the protocol clause `Twice` over `c: Counter`, whose `run` is a sequence of
attempt node `a1` of `Spec::Counter::inc` by `c`, attempt node `a2` of the
same operation, and `finish`. Refinement `CasTwice`: `abstract Spec::Twice
concrete Impl` over ADR-020 §8's compare-and-set model with its `peek`
operation removed, `value = self.value`, `commitA` and `commitB` mapped
`-> Spec::Counter::inc(self)`, `beginA`, `beginB`, `retryA` and `retryB`
mapped `-> stutter`, and a profile internal list that names `Spec::Twice::finish`.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-147-AC-1 | `CasTwice` checks with `AbstractSide::Protocol` naming `Twice` and `abstract_internal = [finish]`, and writes one requirement record of kind `refinement` and no `temporal-satisfaction` record for the declaration. A row `step Impl::Counter::commitA -> node Spec::Twice::a1(self);` checks, naming node `a1`. | Test (TC-552) |
| FR-147-AC-2 | Refusals: `abstract Spec::Thrice` (`missing_declaration`/`missing-name`); a profile internal list naming `Spec::Twice::zz` (`missing_declaration`/`missing-name`); a list naming `Spec::Twice::a1` beside a row `-> node Spec::Twice::a1(self)` (`invalid_model_binding`/`conflicting-binding`, a QSL addition to FR-432); `-> node Spec::Twice::a1(self)` in `CasRefinesCounter`, whose abstract side is a model (`invalid_model_binding`/`malformed-declaration`); a list naming `Spec::Twice::finish` in `CasRefinesCounter` (`invalid_model_binding`/`malformed-declaration`). | Test (TC-552) |
| FR-147-AC-3 | `CasTwice` with `ensure fair weak Spec::Counter::inc` settles its liveness half `unsupported`, `unsupported-requested-capability`, naming hidden abstract fields, beside the safety result of FR-148. | Test (TC-552) |
| FR-147-AC-4 | `CasTwice` with `finish` removed from the profile's internal list checks, with `finish` visible and `abstract_internal` empty; with the list naming `finish` it checks with `finish` internal; with the list naming `finish` and a step row `-> node Spec::Twice::finish(self)` it refuses `invalid_model_binding`/`conflicting-binding` (a QSL addition to FR-432). | Test (TC-552) |

## Dependencies

- ADR-020 §1 RM-1 to RM-6, §4 AX-2 and AX-5, §7 MC-1 (as amended by ADR-027);
  ADR-027 PS-1, ST-1 to ST-15, PR-2 and PR-3; ADR-012 (requirement records).
- [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-135](FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md),
  [FR-136](FR-136-check-a-refinement-s-step-rows.md),
  [FR-144](FR-144-request-and-settle-a-refinement-item.md).
- FR-148 decides the steps of such a refinement.

## References

- The QSpec half: QSpec FR-177 (its relation over QSpec FR-377's step
  rules, QS-8) and FR-377 (Linear STD-133).
