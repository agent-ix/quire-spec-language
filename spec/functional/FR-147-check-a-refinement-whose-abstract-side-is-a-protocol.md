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
- A `refinement_internals` value, the data shape of the profile's internal
  list (see "The profile's internal list"). No document of the build hosts
  one yet, so it is absent.
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
where it lives or what it looks like. QSL decides what it can decide alone:
the **data shape** of the list and how S3 **resolves** it. It does not decide
where the list is hosted; that waits on QSpec (Remaining work, STD-199).

**Shape.** A `refinement_internals` value is a JSON array of objects
`{"refinement": "<name>", "internal": ["<class>", ...]}`, where `refinement`
is the name of a refinement declared in the unit and `internal` is an array of
class spellings. A class spelling is exactly one of

| Class | Spelling | Resolves to |
| --- | --- | --- |
| a node | `<A>::<protocol>::<node>` | an attempt, event, send, receive or finish node of the abstract protocol |
| a channel duplicate | `duplicate <A>::<protocol>::<channel>` | the `duplicate` step of that channel |
| a channel loss | `lose <A>::<protocol>::<channel>` | the `lose` step of that channel |
| a memory step | `memory <kind>` | a memory step kind of the abstract model's memory model |

`<A>` is the abstract alias of the refinement, `<protocol>`, `<node>`,
`<channel>` and `<kind>` are identifiers. The spelling has one space after a
keyword, no other whitespace and no other characters; it is its own
**canonical spelling**. The classes of FR-432 that have no spelling here
(`fork`, `join`, `cend`, `spawn`, `retire`, `activate`) cannot be named
internal, so each stays visible (Remaining work).

**Resolution.** S3 resolves one refinement's entries against its abstract
protocol clause. The resolved classes, in ascending order of the UTF-8 bytes
of their canonical spellings, are `abstract_internal`. The order of objects in
the array and of entries within one `internal` array is free and does not
change the result.

**Absent is empty.** A refinement with no object in the value, and an absent
value, has an empty list: every abstract step class is visible. Absence is
not a refusal (QSpec FR-432).

**Defects.** Each refusal below carries as its locus the RFC 6901 JSON Pointer
of the offending value inside the `refinement_internals` value (such as
`/0/internal/1`), since a JSON value has no source span and no IR node; the host
adds its own prefix when QSpec decides it. A refusal naming two values carries
both pointers.

| Defect | Refusal |
| --- | --- |
| the value is not an array of objects; an object lacks `refinement` or `internal`, has another key, or has a non-string `refinement`, a non-array `internal` or a non-string entry | `invalid_model_binding`/`malformed-declaration` |
| an entry that is not one of the four spellings above (an empty string, `frob x`, `spawn Spec::Twice::r`, `Spec::Twice`, extra or leading whitespace, a non-identifier part) | `invalid_model_binding`/`malformed-declaration` |
| `refinement` names no refinement of the unit; an entry spelled correctly whose alias is not the refinement's abstract alias or whose node, channel or kind does not exist, or whose node is not an attempt, event, send, receive or finish node | `missing_declaration`/`missing-name` |
| two objects naming one refinement; one canonical spelling twice in one `internal` array | `invalid_model_binding`/`conflicting-binding` |

Every code and cause above is in QSpec FR-272's closed lists.

**Until QSpec decides the host.** The build supplies no list: every refinement
has an empty list, and every abstract step class is visible. S3's refinement
check takes the resolved list as an input, and a non-empty list reaching it in
any form, on a model or a protocol abstract side, is refused
`unsupported_construct`/`not-yet-implemented` (FR-341-AC-5), once per
refinement, at the first token of the name written after `abstract` (the
abstract-side name, which both forms have), naming the feature
`refinement-internal-list`, the refinement's name and the exact selected
profile identity, with no `CheckedRefinement`. Whether `abstract_internal` is a
member of the refinement node, whether it enters the `package_id`, the lock
and replay, and how a verdict reproduces the list on replay are not decided
here.

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
- A non-empty internal list on any abstract side, a model subject or a
  protocol subject, is refused by the one rule of "The profile's internal
  list", `unsupported_construct`/`not-yet-implemented`, and not by a binding
  refusal.

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
the protocol clause `Twice` over `c: Counter`, which declares a channel `ch`
and whose `run` is a sequence of attempt node `a1` of `Spec::Counter::inc` by
`c`, attempt node `a2` of the same operation, a `parallel` node `par` of two
empty branches, and `finish`. Refinement `CasTwice`: `abstract Spec::Twice
concrete Impl` over ADR-020 §8's compare-and-set model with its `peek`
operation removed, `value = self.value`, `commitA` and `commitB` mapped
`-> Spec::Counter::inc(self)`, `beginA`, `beginB`, `retryA` and `retryB`
mapped `-> stutter`, and the `refinement_internals` value `[{"refinement":
"CasTwice", "internal": ["Spec::Twice::finish"]}]`. A second refinement
`CasRefinesCounter` (ADR-020 §8, model abstract side) is declared in the unit
where a vector names it.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-147-AC-1 | (Waits on STD-199 and STD-182.) `CasTwice` checks with `AbstractSide::Protocol` naming `Twice` and `abstract_internal = [finish]`, and writes one requirement record of kind `refinement` and no `temporal-satisfaction` record for the declaration. A row `step Impl::Counter::commitA -> node Spec::Twice::a1(self);` checks, naming node `a1`. | Test (TC-552) |
| FR-147-AC-2 | Refusals: `abstract Spec::Thrice` (`missing_declaration`/`missing-name`); an internal list naming `Spec::Twice::zz` (`missing_declaration`/`missing-name`); an internal list naming `Spec::Twice::a1` beside a row `-> node Spec::Twice::a1(self)` (`invalid_model_binding`/`conflicting-binding`, a QSL addition to FR-432); `-> node Spec::Twice::a1(self)` in `CasRefinesCounter`, whose abstract side is a model (`invalid_model_binding`/`malformed-declaration`). A non-empty list on `CasRefinesCounter` is AC-7's, not a binding refusal. The first three vectors wait on STD-199 and STD-182 (Remaining work). | Test (TC-552) |
| FR-147-AC-3 | (Waits on STD-199 and STD-182.) `CasTwice` with `ensure fair weak Spec::Counter::inc` settles its liveness half `unsupported`, `unsupported-requested-capability`, naming hidden abstract fields, beside the safety result of FR-148. | Test (TC-552) |
| FR-147-AC-4 | (Waits on STD-199 and STD-182.) `CasTwice` with `finish` removed from its `refinement_internals` list checks, with `finish` visible and `abstract_internal` empty; with the list naming `finish` it checks with `finish` internal; with the list naming `finish` and a step row `-> node Spec::Twice::finish(self)` it refuses `invalid_model_binding`/`conflicting-binding` (a QSL addition to FR-432). | Test (TC-552) |
| FR-147-AC-5 | Resolution of the shape. For `CasTwice`, the value above resolves to `abstract_internal = [finish]`. The value absent, `[]`, and `[{"refinement": "CasRefinesCounter", "internal": []}]` each resolve `CasTwice` to the empty list. A value naming `["Spec::Twice::finish", "lose Spec::Twice::ch"]` and its permutation, and the same two inside two arrays swapped, resolve to the same list, in the byte order of `Spec::Twice::finish` then `lose Spec::Twice::ch`. | Test (TC-552) |
| FR-147-AC-6 | Defects, each refused with no list: `{"refinement": "CasTwice"}` at `/0`, `"internal": "Spec::Twice::finish"` at `/0/internal`, a non-string entry at `/0/internal/0`, and an object key `visible` at `/0` (each `invalid_model_binding`/`malformed-declaration`); the entries `""`, `frob x`, `spawn Spec::Twice::r`, `Spec::Twice` and ` Spec::Twice::finish` (each `malformed-declaration` at its pointer); `{"refinement": "Nope", ...}` at `/0/refinement`, the entries `Other::Twice::finish`, `Spec::Twice::zz`, `lose Spec::Twice::nochan` and a `parallel` node (each `missing_declaration`/`missing-name` at its pointer); two objects for `CasTwice` at `/1` and `/0`, and `["Spec::Twice::finish", "Spec::Twice::finish"]` at `/0/internal/1` and `/0/internal/0` (each `invalid_model_binding`/`conflicting-binding`). | Test (TC-552) |
| FR-147-AC-7 | The refinement check given a non-empty resolved list for `CasRefinesCounter` (model side) refuses exactly once, `unsupported_construct`/`not-yet-implemented` at the first token of the name after `abstract` (its model alias), naming `refinement-internal-list`, the refinement and the profile identity, classifies `unsupported` (FR-341-AC-5), produces no `CheckedRefinement` and no `invalid_model_binding`. Given a non-empty resolved list for `CasTwice` (protocol side), in a build that has not built a protocol abstract side, it refuses exactly twice, each once: `protocol-abstract-side` at `Twice` and `refinement-internal-list` at `Spec` (the first token of `Spec::Twice`), both classified `unsupported`, with no `CheckedRefinement`. Given an empty list it does not refuse for the list. |

## Remaining work

QSL-proposed pending QSpec, Linear STD-199 (QSpec FR-432 says "the
refinement profile's explicit list" and FR-375 gives the declaration no
row). The data shape, the class spellings, the resolution and ordering and
the defect causes above are QSL's own decisions and stand until QSpec
spells them differently. QSpec has not decided, and this requirement does
not:

- the **host** of the `refinement_internals` value: the source unit, the
  compile request, the lock or another document;
- a **node member** for `abstract_internal` in FR-375's refinement node (its
  body list is closed), and so whether the list is part of the `package_id`
  and obligation identities;
- its inclusion in the lock sources and replay (ADR-015; QSpec FR-323-AC-7),
  without which a replay recompiles with an empty list;
- spellings for the classes of FR-432 that cannot be named here (`fork`,
  `join`, `cend`, `spawn`, `retire`, `activate`).

Until STD-199 decides, no list is supplied and a non-empty one is refused
(above). FR-147-AC-1, AC-3 and AC-4 need a protocol abstract side (Linear
STD-182) and a non-empty list, so they wait on both. In AC-2, `abstract
Spec::Thrice`, the list naming `zz` and the list-beside-row conflict wait on
both too: FR-135 refuses a protocol abstract side and AC-7 refuses a
non-empty list, so none can run; only the `CasRefinesCounter` node-row vector
runs now. AC-5 to AC-7 are testable now over the resolver and the refusal.

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
