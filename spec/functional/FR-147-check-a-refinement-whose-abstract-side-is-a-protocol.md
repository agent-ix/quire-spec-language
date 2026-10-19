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
- The unit's `refinement_internals` member, the one place the profile's
  internal list is declared (see "The profile's internal list").
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

### The profile's internal list

A profile selection carries an identity and nothing else (FR-110; QSpec's
shared grammar), and QSpec's refinement grammar has no `internal` row
(QSpec FR-375), so QSpec names the list (QSpec FR-432) without declaring
where it lives. QSL declares it in the unit's own admission input, as the
**`refinement_internals` member** beside `dependency_selections`. The
profile document, the admitted abstract package and the refinement
declaration are not sources: the first has no member to hold a list, the
second belongs to another author who cannot know which of its steps a
particular refinement treats as internal, and the third would add a
row QSpec's grammar does not have. The member is QSL-proposed pending
QSpec's ruling (Remaining work).

- **Shape.** `refinement_internals` is a JSON array of objects
  `{"refinement": "<name>", "internal": ["<class>", ...]}`. `refinement`
  is the name of a refinement declared in the unit; `internal` is an array
  of abstract step class spellings, each a string in the form of FR-432's
  left sides with the abstract alias: `<A>::<protocol>::<node>` for a node
  class, `duplicate <A>::<protocol>::<channel>` or
  `lose <A>::<protocol>::<channel>` for a channel class, and
  `memory <kind>` for a memory step kind.
- **Reading.** S3 reads the member once, when it checks the refinement
  named by `refinement`, resolving each entry against the abstract
  protocol clause. The resolved classes, ordered by their canonical class
  keys and without repeats, are the refinement's `abstract_internal`, and
  they enter the refinement node and so the `package_id` and every
  obligation identity that cites it (FR-135 "Package and identity"): the
  verdict is a function of the checked package, not of the member read
  again.
- **Absent is empty.** A unit with no `refinement_internals` member, or none
  whose `refinement` names this declaration, has an empty list for it: every
  abstract step class is visible. Absence is not a refusal (QSpec FR-432).
- **Defects.** A member that is not an array of objects with a string
  `refinement` and an array of string `internal` entries, or an object with
  an unknown key, SHALL refuse `invalid_model_binding`/`malformed-declaration`
  naming the member path. An entry whose `refinement` names no refinement of
  the unit SHALL refuse `missing_declaration`/`missing-name` naming it. Two
  objects naming one refinement, or one class named twice in one `internal`
  array, SHALL refuse `invalid_model_binding`/`conflicting-binding` naming
  both.
- Every cause above is in QSpec FR-272's closed lists. Only the member is
  QSL-proposed.

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
mapped `-> stutter`, and the unit member `refinement_internals =
[{"refinement": "CasTwice", "internal": ["Spec::Twice::finish"]}]`.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-147-AC-1 | `CasTwice` checks with `AbstractSide::Protocol` naming `Twice` and `abstract_internal = [finish]`, and writes one requirement record of kind `refinement` and no `temporal-satisfaction` record for the declaration. A row `step Impl::Counter::commitA -> node Spec::Twice::a1(self);` checks, naming node `a1`. | Test (TC-552) |
| FR-147-AC-2 | Refusals: `abstract Spec::Thrice` (`missing_declaration`/`missing-name`); an internal list naming `Spec::Twice::zz` (`missing_declaration`/`missing-name`); an internal list naming `Spec::Twice::a1` beside a row `-> node Spec::Twice::a1(self)` (`invalid_model_binding`/`conflicting-binding`, a QSL addition to FR-432); `-> node Spec::Twice::a1(self)` in `CasRefinesCounter`, whose abstract side is a model (`invalid_model_binding`/`malformed-declaration`); a `refinement_internals` object for `CasRefinesCounter` naming `Spec::Twice::finish` (`invalid_model_binding`/`malformed-declaration`). | Test (TC-552) |
| FR-147-AC-3 | `CasTwice` with `ensure fair weak Spec::Counter::inc` settles its liveness half `unsupported`, `unsupported-requested-capability`, naming hidden abstract fields, beside the safety result of FR-148. | Test (TC-552) |
| FR-147-AC-4 | `CasTwice` with `finish` removed from its `refinement_internals` list checks, with `finish` visible and `abstract_internal` empty; with the list naming `finish` it checks with `finish` internal; with the list naming `finish` and a step row `-> node Spec::Twice::finish(self)` it refuses `invalid_model_binding`/`conflicting-binding` (a QSL addition to FR-432). | Test (TC-552) |
| FR-147-AC-5 | The list is read from the unit's `refinement_internals` member only. `CasTwice` with the member above checks with `abstract_internal = [finish]`. The same unit with the member absent, with `[]`, and with an object for another refinement checks with `abstract_internal` empty and `finish` visible. Two units that differ only in this member have different `package_id`s. | Test (TC-552) |
| FR-147-AC-6 | Member defects: `{"refinement": "CasTwice"}` with no `internal`, `"internal": "Spec::Twice::finish"` (not an array), a non-string entry, and an object key `visible` each refuse `invalid_model_binding`/`malformed-declaration` naming the member path; `{"refinement": "Nope", ...}` refuses `missing_declaration`/`missing-name` naming `Nope`; two objects for `CasTwice`, and `["Spec::Twice::finish", "Spec::Twice::finish"]`, each refuse `invalid_model_binding`/`conflicting-binding` naming both. No refusal produces a `CheckedRefinement`. | Test (TC-552) |

## Remaining work

- The `refinement_internals` member and its shape, the class spellings and
  AC-5 and AC-6 are QSL-proposed. QSpec FR-432 says "the refinement profile's
  explicit list" and FR-375 gives the declaration no row; the QSpec
  follow-up is Linear STD-199. If QSpec declares the list in another place
  or shape, that replaces this member, with no second source kept.

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
