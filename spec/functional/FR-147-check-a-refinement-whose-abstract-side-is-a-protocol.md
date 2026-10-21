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
states its observation, and the refinement's internal list (QSpec FR-432)
names the abstract step classes that are internal; every other abstract step
class is visible. The relation's meaning, including
divergence-freedom, refusal sets, terminal success and assumption
weakening, is QSpec's (ADR-020 QS-8); this requirement covers QSL's S3
checks and requirement record. FR-148 decides the steps.

## Use case

A specification author has an abstract protocol that performs two
increments and then finishes, and a concrete compare-and-set model. They
map each commit to the abstract increment, each other concrete step to an
internal step, and list the abstract `finish` in the refinement's internal
list. QSL checks the
rows at compile time and requests one protocol refinement claim.

## Inputs

- The parsed refinement form, whose abstract side is written
  `abstract <A>::<protocol>` (QSpec FR-375's `abstract-subject`), with step
  rows whose right sides may be `node <A>::<protocol>::<node>(...)` (QSpec
  FR-432). The declaration carries no `internal` or `visible` row; the
  internal list is not source (ADR-020 RM-9, MC-1).
- The compilation's optional `refinement_internals` input: one value for the
  unit and one for each supplied library (QSpec FR-375), carried by the
  compile input as supplied and unchecked (FR-099). Its shape and
  spellings are described in "The refinement's internal list". An absent
  value is the empty list.
- The abstract model's checked protocol clause and its step classes
  (ADR-027 ST-1 to ST-15).

## Outputs

- `CheckedRefinement` with `abstract_side: AbstractSide::Protocol{alias,
  clause: NodeKey}` and `abstract_internal: Vec<AbstractStepClass>`, the
  resolved classes in ascending UTF-8 byte order of their alias-free
  spellings. The refinement's node carries them as those alias-free
  spellings, and carries no `abstract_internal` entry for an empty list
  (QSpec FR-375).
- Step rows whose `StepTarget::Abstract` may name an abstract protocol node
  as well as an abstract operation.
- A requirement record of kind `refinement` (QSpec FR-290's `protocol`
  family member) for the declaration.

## Behavior

### Abstract side

- When the abstract side names `<A>::<protocol>`, the checker SHALL resolve
  it to a checked protocol clause of the abstract model, refusing
  `missing_declaration`/`missing-name` when none matches.
- The checker SHALL resolve the clause against the checked clause: an
  abstract protocol declared in the declaring package is checked before the
  refinement that names it is resolved, the order FR-136 states for a
  concrete protocol; a dependency's clause is already checked.
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

### The refinement's internal list

A profile selection carries an identity and nothing else (FR-110; QSpec's
shared grammar), and QSpec's refinement grammar has no `internal` row, so the
list is not source. QSpec owns it: the list is the compilation's optional
`refinement_internals` input, supplied beside the source as a supplied
library is, and its shape, class spellings, resolution, refusals, effect on
the refinement node, lock and replay are QSpec FR-375's and FR-432's. This
requirement states how S3 applies them. The names `AbstractStepClass` and
`abstract_internal` are QSL's own Rust-side names for what QSpec describes.

**Shape.** A `refinement_internals` value is a JSON array of objects
`{"refinement": "<name>", "internal": ["<class>", ...]}`, closed to those two
keys. `refinement` names a refinement declared in the compiled source (the
unit, or one supplied library; a library's list is its own and names only its
own refinements). `internal` is an array of class spellings.

**Class spellings.** A class spelling is the left side of the class's row
(QSpec FR-432), without `step`, with the refinement's abstract alias `<A>` in
place of `<C>`. It has one space after a leading keyword, no other
whitespace and no other characters; it is its own canonical spelling.
`<A>::<protocol>` must be the refinement's abstract alias and selected
abstract protocol. QSpec's alias-free spelling, which the refinement node
carries, is the same text with `<A>::` removed.

| Abstract step class | Spelling | Alias-free (the spelling with `<A>::` removed) |
| --- | --- | --- |
| `attempt`, `event`, `send`, `receive`, `timeout`, `finish` or `fence` of one node; `cattempt` of one template | `<A>::<protocol>::<node>`, or `<A>::<protocol>::<template>` | `<protocol>::<node>`, or `<protocol>::<template>` |
| `fork`, `join` of one `parallel` node | `fork <A>::<protocol>::<node>`, `join <A>::<protocol>::<node>` | `fork <protocol>::<node>`, `join <protocol>::<node>` |
| `cend` of one template | `cend <A>::<protocol>::<template>` | `cend <protocol>::<template>` |
| `duplicate`, `lose` on one channel | `duplicate <A>::<protocol>::<channel>`, `lose <A>::<protocol>::<channel>` | `duplicate <protocol>::<channel>`, `lose <protocol>::<channel>` |
| `spawn`, `retire` of one role | `spawn <A>::<protocol>::<role>`, `retire <A>::<protocol>::<role>` | `spawn <protocol>::<role>`, `retire <protocol>::<role>` |
| `activate` of the clause's trigger | `activate <A>::<protocol>` | `activate <protocol>` |
| one memory step kind | `memory <kind>` | `memory <kind>` |

A bare node path names the one class of an attempt, event, send, receive,
timeout, finish or fence node or of a template's `cattempt`. A `parallel`
node has the two classes `fork` and `join`, so its bare path is not a class.
`<A>`, `<protocol>`, `<node>`, `<template>`, `<channel>`, `<role>` and
`<kind>` are identifiers.

**Resolution.** S3 SHALL resolve one refinement's entries against its selected
abstract protocol clause (and, for `memory`, the abstract model's memory
model). The resolved classes, in ascending order of the UTF-8 bytes of their
alias-free spellings, are `abstract_internal`. The order of objects in the
array and of entries within one `internal` array is free and does not change
the result. Because the alias is out of the spelling, renaming the abstract
alias, with the value's spellings renamed to match, changes neither
`abstract_internal` nor the refinement's node identity (QSpec FR-375-AC-13).

**Absent is empty.** A refinement with no object in the value, and an absent
value, has an empty list: every abstract step class is visible. Absence is
not a refusal (QSpec FR-432).

**Defects.** S3 SHALL refuse each defect below with no `CheckedRefinement`, located by a pair, the **input** (the
unit's `refinement_internals` value, or the value of one named supplied
library) and the RFC 6901 JSON Pointer of the offending value inside it (such
as `/0/internal/1`); a JSON value has no source span and no IR node. The pair
names no member of the compile request that carries the value. A refusal
naming two values carries both pairs. A refusal from a supplied library's
value refuses the whole compilation, the unit included, whether or not the
unit imports that library.

| Defect | Refusal |
| --- | --- |
| the value is not an array of objects; an object lacks `refinement` or `internal`, has another key, a non-string `refinement`, a non-array `internal` or a non-string entry | `invalid_model_binding`/`malformed-declaration` |
| an entry that is not one of the spellings above (an empty string, `frob x`, `Spec::Twice`, extra or leading whitespace, a non-identifier part) | `invalid_model_binding`/`malformed-declaration` |
| `refinement` names no refinement of the compiled source; an entry spelled correctly whose alias or protocol is not the refinement's abstract alias and selected protocol, or whose node, template, channel, role or memory kind does not exist, or whose bare path names a `parallel` node | `missing_declaration`/`missing-name` |
| two objects naming one refinement; one canonical spelling twice in one `internal` array | `invalid_model_binding`/`conflicting-binding` |
| a non-empty list for a refinement whose abstract subject is a model (once the feature is built; see below) | `invalid_model_binding`/`malformed-declaration` at the object's pointer |

Every code and cause above is in QSpec FR-272's closed lists. A defect
attributable to no refinement (a value that is not an array of objects, `{}`,
a `refinement` that is not a string or names no refinement) is reported by
this table alone. Each refusal yields no `CheckedRefinement`.

**One class, one decision.** A class that is in the list and is also the
abstract node a step row's right side names (`node <A>::<protocol>::<node>(...)`)
refuses `invalid_model_binding`/`conflicting-binding`, naming the row and the
list entry; neither wins. The conflict exists only for a class a right side
can name.

**Precedence and the unbuilt feature.** S3 SHALL resolve the list first and
SHALL report every defect above whatever the build supports, except the
model-subject row. Only a **non-empty resolved** list then reaches the
feature check: where the build's support set omits `refinement-internal-list`
(FR-135), the refinement check SHALL refuse it
`unsupported_construct`/`not-yet-implemented` (FR-341-AC-5), once per
refinement, at the first token of the name written after `abstract` (which
both subject forms have), naming the feature, the refinement's name and the
exact selected profile identity, with no `CheckedRefinement`. An empty list
never refuses for the feature, and a list with a resolution defect reports
that defect and no not-yet-built refusal. **A model-subject refinement.** A model subject selects no protocol, so for
its list S3 SHALL check the value's shape, the spelling of each entry, the
repetition of an object or of a spelling, and that `refinement` names a
refinement of the compiled source; it SHALL NOT resolve an entry's alias,
protocol, node, template, channel, role or memory kind, since there is
nothing to resolve them against. A well-formed non-empty list is then refused
only as not yet built, at the name after `abstract`, in a build that omits
`refinement-internal-list`; in a build that has it, the list is refused
`invalid_model_binding`/`malformed-declaration` at the object's pointer
(QSpec FR-432). 
**Both features.** A protocol abstract side the build has not
built gives its own refusal beside it (FR-135-AC-5), so a non-empty list
under `abstract Spec::Twice` in a build omitting both features gives two.

### Internal abstract steps

- An abstract step class the list names SHALL be internal, and one it does not
  name SHALL be visible with its observation label (QSpec FR-432). No class is
  refused for being named by neither the list nor a step row (ADR-020 MC-1).
- A class named both by the list and as the right side of a step row is
  refused as "One class, one decision" states.
- A non-empty internal list on any abstract side, a model subject or a
  protocol subject, is refused by the one rule of "Precedence and the
  unbuilt feature", and not by a binding refusal, until the build has the
  feature.

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
mapped `-> stutter`, and the unit's `refinement_internals` value
`[{"refinement": "CasTwice", "internal": ["Spec::Twice::finish"]}]`. A second
refinement `CasRefinesCounter` (ADR-020 §8, model abstract side) is declared
in the unit where a vector names it. Fixture `Full`: a protocol clause that
declares one node of each kind (attempt, event, send, receive, timeout,
finish, fence), a `cattempt` template, a `parallel` node, a channel, a role, a
trigger, and a memory model with a step kind `flush`; a refinement `CasFull`
selects it.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-147-AC-1 | In a build whose support set holds `protocol-abstract-side` and `refinement-internal-list`, `CasTwice` checks with `AbstractSide::Protocol` naming `Twice` and `abstract_internal = [Twice::finish]`, and writes one requirement record of kind `refinement` and no `temporal-satisfaction` record for the declaration. A row `step Impl::Counter::commitA -> node Spec::Twice::a1(self);` checks, naming node `a1`. | Test (TC-552) |
| FR-147-AC-2 | Refusals: `abstract Spec::Thrice` (`missing_declaration`/`missing-name`); an internal list naming `Spec::Twice::zz` (`missing_declaration`/`missing-name`); an internal list naming `Spec::Twice::a1` beside a row `-> node Spec::Twice::a1(self)` (`invalid_model_binding`/`conflicting-binding`, naming the row and the entry); `-> node Spec::Twice::a1(self)` in `CasRefinesCounter`, whose abstract side is a model (`invalid_model_binding`/`malformed-declaration`). In a build whose support set omits `refinement-internal-list`, a non-empty list on `CasRefinesCounter` is AC-7's, not a binding refusal; in a build that has it, AC-12's. A producer that has built a feature is checked by that feature's own criteria. The first three vectors need a build that holds `protocol-abstract-side` and `refinement-internal-list`; the `CasRefinesCounter` node-row vector runs in any build. | Test (TC-552) |
| FR-147-AC-3 | In a build that holds both features, `CasTwice` with `ensure fair weak Spec::Counter::inc` settles its liveness half `unsupported`, `unsupported-requested-capability`, naming hidden abstract fields, beside the safety result of FR-148. | Test (TC-552) |
| FR-147-AC-4 | In a build that holds both features, `CasTwice` with `finish` removed from its `refinement_internals` list checks, with `finish` visible and `abstract_internal` empty; with the list naming `finish` it checks with `finish` internal; with the list naming `finish` and a step row `-> node Spec::Twice::finish(self)` it refuses `invalid_model_binding`/`conflicting-binding`. | Test (TC-552) |
| FR-147-AC-5 | Resolution of the shape, in any build (the resolver only). For `CasTwice`, the value above resolves to `abstract_internal = [Twice::finish]`. The value absent, `[]`, and `[{"refinement": "CasRefinesCounter", "internal": []}]` each resolve `CasTwice` to the empty list. A value naming `["Spec::Twice::finish", "lose Spec::Twice::ch"]` and its permutation, and the same two inside two arrays swapped, resolve to the same list, in the byte order of `Twice::finish` then `lose Twice::ch`. For a refinement with the lower-case alias `spec` over `Twice`, the value `["spec::Twice::finish", "lose spec::Twice::ch"]` and its permutation resolve to `[Twice::finish, lose Twice::ch]`, the order of the alias-free spellings, which the alias-including spellings (`lose spec::Twice::ch` first) would reverse. | Test (TC-552) |
| FR-147-AC-6 | Defects, in any build, each refused with no list, located at (the unit's value, pointer): `{"refinement": "CasTwice"}` at `/0`, `"internal": "Spec::Twice::finish"` at `/0/internal`, a non-string entry at `/0/internal/0`, `"refinement": 7` at `/0/refinement`, and an object key `visible` at `/0` (each `invalid_model_binding`/`malformed-declaration`); the entries `""`, `frob x`, `Spec::Twice` and ` Spec::Twice::finish` (each `malformed-declaration` at its pointer); `{"refinement": "Nope", ...}` at `/0/refinement`, the entries `Other::Twice::finish`, `Spec::Twice::zz`, `spawn Spec::Twice::r` (`Twice` declares no role `r`), `lose Spec::Twice::nochan` and `Spec::Twice::par` (the bare path of a `parallel` node) (each `missing_declaration`/`missing-name` at its pointer); two objects for `CasTwice` at `/1` and `/0`, and `["Spec::Twice::finish", "Spec::Twice::finish"]` at `/0/internal/1` and `/0/internal/0` (each `invalid_model_binding`/`conflicting-binding`). | Test (TC-552) |
| FR-147-AC-7 | In a build whose support set omits the features named, the refinement check given a non-empty resolved list for `CasRefinesCounter` (model side), in a build whose support set omits `refinement-internal-list`, refuses exactly once, `unsupported_construct`/`not-yet-implemented` at the first token of the name after `abstract` (its model alias), naming `refinement-internal-list`, the refinement and the profile identity, classifies `unsupported` (FR-341-AC-5), produces no `CheckedRefinement` and no `invalid_model_binding`. Given a non-empty resolved list for `CasTwice` (protocol side), in a build that also omits `protocol-abstract-side`, it refuses exactly twice, each once: `protocol-abstract-side` at `Twice` and `refinement-internal-list` at `Spec` (the first token of `Spec::Twice`), both classified `unsupported`, with no `CheckedRefinement`. Given an empty list it does not refuse for the list. | Test (TC-552) |
| FR-147-AC-8 | In any build, each class spells and resolves over `Full`: a node path for an attempt, event, send, receive, timeout, finish and fence node and a template path for a `cattempt`, and `fork`, `join`, `cend`, `duplicate`, `lose`, `spawn`, `retire`, `activate` and `memory flush` entries, each making exactly its own class internal; the resolved list for all of them is in the byte order of their alias-free spellings. | Test (TC-552) |
| FR-147-AC-9 | In a build that holds both features, `abstract_internal` is alias-free: for `CasTwice`, `Twice::finish`. Renaming the abstract alias `Spec` to `S`, with the value's spellings renamed to match, gives the same `abstract_internal` and the same refinement node identity; the empty list and `[Twice::finish]` give different node identities. | Test (TC-552) |
| FR-147-AC-10 | In any build, the locus of a list defect is the pair (input, pointer): a defect in the unit's value is located at (the unit's value, its pointer); the same defect in a supplied library's value is located at (that library's value, its pointer) and refuses the whole compilation, the unit included, whether or not the unit imports that library (a unit with no import of it is refused all the same). No refusal names a compile-request member. | Test (TC-552) |
| FR-147-AC-11 | Precedence, in a build whose support set omits `refinement-internal-list`: a list on `CasTwice` (protocol subject) with an unresolved entry (`Spec::Twice::zz`) reports `missing_declaration`/`missing-name` and no `not-yet-implemented` naming `refinement-internal-list`, and, because the build also omits `protocol-abstract-side`, the independent `not-yet-implemented` refusal for that feature at `Twice`; a value attributable to no refinement (`{}`, `"refinement": 7`) reports its defect alone; a non-empty resolved list on `CasRefinesCounter` reports only AC-7's refusal, and a malformed entry, a repeated spelling or a `refinement` naming no refinement on it is still reported while an entry such as `Other::Twice::finish` or `Spec::Twice::zz` is not resolved and gives no `missing-name`. | Test (TC-552) |
| FR-147-AC-12 | In a build that holds `refinement-internal-list`, a non-empty list for `CasRefinesCounter` (model side) refuses `invalid_model_binding`/`malformed-declaration` at (the unit's value, `/0`), with no `CheckedRefinement` and no `not-yet-implemented`; an empty list does not refuse. | Test (TC-552) |

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
