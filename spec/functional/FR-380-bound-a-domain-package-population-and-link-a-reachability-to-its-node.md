---
id: FR-380
title: "Author a finite bound on a domain-package population and emit its population node for a reachability"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-333
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-365
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
---
# FR-380: Author a finite bound on a domain-package population and emit its population node for a reachability

## Description

A domain-package population declaration has no maximum (QSpec FR-153), so a
state-clause `reaches`, over a relationship end or a field edge, walks a
population that FR-104
records as unbounded, and a proof backend that needs a finite population
cannot settle the claim. This requirement lets the author state a finite
maximum for a population in source with a `bound` declaration, makes the
population a bounded domain of the requirements records that range over it,
and, for each reachability that walks a bounded population, emits the one
`bounded_domain`/`model_population` node a reader selects the population by,
which FR-365 links a relationship-end reachability to and "Field-edge
reachability" links a field-edge one to.

The bound is an authored semantic bound in ADR-014's sense (B-1): part of the
meaning of the population, never a resource limit. A population the package
does not bound stays unbounded in every respect.

## Use case

An author writes `bound HistoryBound on Config::config_history [8]` next to
`invariant NoCycle using v on Config::ConfigVersion at current { not
reaches(self, self, previous) }`. The `config_history` population is then a
population of at most eight objects for every clause of the package, and the
lowered `reaches` of `NoCycle` names the node that says so.

## Inputs

- A unit's `bound` declaration: a label, a model alias `M`, the name `P` of a
  population declaration of `M`'s domain package, and an unsigned integer `N`
  in the spelling FR-333 reads (`0` or a nonzero digit followed by digits;
  `0` is a valid maximum).
- The assembled `PackageDeclarations` with the populations of the admitted
  domain package (FR-084; QSpec FR-153).

## Outputs

- A checked population bound: the population and its maximum `N`, recorded in
  the `CheckedGraph` under the label.
- The population's domain in each requirements record that names it
  (FR-104, FR-365) as a `Population(Some(N))` domain.
- For each reachability, over a relationship end or a field edge, that walks
  a bounded population, the population node and the `Type` occurrence the
  node holds for the bound (see "The population node"), and for a field edge
  the link to it ("Field-edge reachability").
- Or a located refusal with a catalog code.

## Behavior

### The declaration

S1 SHALL parse `bound <label> on <model-alias>::<population> [<N>]` as a
declaration. `bound` is a declaration-head spelling only, as `invariant`, `pre`,
`post` and `body` are (FR-102): as an expression identifier it parses in member position only (`b.bound`), because
`bound` is also a reserved word (the verification step `bound N`). S2 SHALL
build one `StateModel` form for it through the closed leading-token-kind entry
table (ADR-012 §3, §4.3), keeping the label, the alias, the population name,
the digits of `N` and the span of each, and SHALL NOT resolve the alias or the
population.

The assembler SHALL resolve the alias to a `model` declaration of the unit and
`P` to a population declaration of that model's domain package, refusing
`missing_declaration`/`missing-name` at the unresolved name. It SHALL refuse a
bound whose `N` is outside `u64` as FR-333 refuses a `Population<T>[N]` whose
`N` is. It SHALL refuse a label that another bound, clause or function of the
package already uses, and two bounds that name one population (equal `N` or
not), `ambiguous_declaration`/`ambiguous-name` at both declarations. A bound
applies to the whole package: the order of units and of declarations SHALL NOT
change what a clause sees.

### Meaning

A population with an authored bound `N` is a `Population(Some(N))` domain
(ADR-014 N-3) in every requirements record that names it, the context
population of a clause and the population a reachability walks alike (FR-104
"Requirements"). It is not listed among a claim's unbounded domains; its
available finite bound is `Cardinality{N}`; a claim whose every domain is
bounded has extent `Bounded` (FR-097). The bound does not change which
population covers a type: FR-104's and FR-365's rules for the one covering
population, and for refusing two, are the same for a bounded population.

When the evaluator reads a population that has an authored bound, an
observation whose selected population holds more than `N` objects SHALL refuse
`cardinality_out_of_bound`/`above-maximum`, naming `N` and the count, as FR-084
states for `allInstances` over a population with a declared maximum; the bound
is never converted into a resource outcome (ADR-014 B-1). The refusal's fields
are `bound` `[0, N]`, `collection` `set` and `count`. It applies to every bounded population the clause's requirement record names
(its context population and each population a `reaches` walks, over a
relationship end or a field edge), before any charge and whether or not a
`reaches` of the body is evaluated. `allInstances` reads the snapshot listing
and adds no bounded population by itself.

### The population node

For each lowered reachability over a relationship end whose walked population
has an authored bound `N` (FR-365), S4 SHALL emit, once, the nodes FR-094 builds
for `Population<E>[N]` with `T` the near end's type `E` of FR-365 (not the
population's canonical member type, which a supertype of `E` can be): the
`Reference<E>` node, the `Set<Reference<E>>` node, the `Integer` scalar node,
and the `bounded_domain`/`model_population` node whose body is the one binding
`max` of the integer literal `N` typed at the `Integer` node and whose
`semantic_type` is the `Set<Reference<E>>` node. The whole chain is emitted
whenever the population node is emitted. All of these are anonymous nodes
(FR-094) and are content-addressed: the node of `(E, N)` is the same node
whichever way it is written, a `Population<E>[N]` parameter type included, and
its key does not depend on the occurrences it holds.

The population node holds one `Type` occurrence for the `[N]` of each bound that
produces it, with exactly one `source_map` row `(node, Type, ordinal)` for the
bound's `[N]` span. Ordinals are assigned in ascending source order over the
node's `Type` occurrences, whatever else writes the node. A bound that no
reachability walks emits no node and no occurrence. A population without an
authored bound has no node and no link. The link of a reachability over a
relationship end to the node is FR-365 "The population link"; the link of a
field-edge reachability is "Field-edge reachability" below.

### Field-edge reachability

A field-edge `reaches(a, b, edge)` (FR-104) whose `edge` is a field of type
`Reference<T>`, `Option<Reference<T>>` or `Sequence<Reference<T>>[l, u]`
walks the objects of `T`, the field's owner and reference target type (QSpec
FR-322 "Reaches over a field", step 2), not the member's `declaration` `D`,
the operand 0 type of step 1. The checker SHALL take the population a
field-edge `reaches` walks as the one population that covers `T` by
conformance (FR-104's and FR-365's covering rule), whatever the operands'
static types: under `quire.model.complete/v1` an operand of a proper subtype
of `T` walks the population that covers `T`, and a population that covers
the operand's type but not `T` is not walked. The checker SHALL add that
population's domain to the clause's requirements record as FR-365 states for
a relationship end (once, and none when it is the context population); the
domain is `Population(Some(N))` exactly when the package authors a bound `N`
on that population.

If two or more populations cover `T`, then the checker SHALL refuse
`ambiguous_declaration`/`ambiguous-name` at the `reaches`, naming `T` and
those populations in ascending `DeclarationKey` order. The refusal's family
is that of FR-104's field-edge checks, not `StateModel`: unlike FR-365's
refusal over a relationship end, the field form's refusal is unchanged.

When the walked population has an authored bound `N`, S4 SHALL emit the
population node of `Population<T>[N]` as "The population node" states with
`E` = `T`, and SHALL write `population: <id>` in the `field` member of the
`quire.op.model.reaches_field` application (FR-105), the id of that node,
with that node among the application's `dependencies`. The member is then
`{kind: "field", declaration, name, population}`; `declaration` (`D`) and
`name` are unchanged, and the population node is over `T` even when `D` is a
proper subtype of `T`. The rule is the same for the three field forms. Only
the authored `bound` supplies `N`: the field's optionality, the sequence's
`[l, u]` and a `Population<T>[N]` parameter type are not population bounds
and supply no link.

When the walked population has no authored bound, or no population covers
`T`, S4 SHALL write no `population` key (absent, not `null`) and emit no
population node for the reachability, and the application SHALL keep the
member and node identity it had before this rule. A walked population with
no authored bound stays a `Population(None)` domain of the record; with no
covering population the record names no domain for the reachability; neither
is a refusal.

The node is content-addressed by `(T, N)`, so a field-edge reachability and a
relationship-end reachability that walk one bounded population of `T` name
one node, which holds one `Type` occurrence per bound that produces it, not
one per reachability.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-380-AC-1 | `bound HistoryBound on Config::config_history [8]` builds one form with label `HistoryBound`, alias `Config`, population `config_history` and `N` `8`, each with a span that slices to its source text; `bound` as an identifier parses in member position only (`b.bound`); `[0]` is accepted and `[-1]` and `[x]` refuse at S1. | Test (TC-1054) |
| FR-380-AC-2 | An alias naming no model and a population name naming no population of the package each refuse `missing_declaration`/`missing-name` at the name; `[18446744073709551616]` refuses as FR-333 refuses it; a second bound with a used label, and two bounds naming one population (equal or different `N`), each refuse `ambiguous_declaration`/`ambiguous-name` at both declarations. | Test (TC-1054) |
| FR-380-AC-3 | With a bound `N` on `config_history`, the requirements record of a clause over `ConfigVersion` holds the `config_history` domain as a `Population(Some(N))` domain with available finite bound `Cardinality{N}`, listed among no unbounded domain, for a context population and for a population a reachability walks; the clause extent is `Bounded` when no other domain is unbounded; two clause orders and two unit orders give equal records; without the bound the record is the unbounded one. | Test (TC-1054) |
| FR-380-AC-4 | A reachability over a relationship end of `Order` that walks a population bounded at `3` lowers with a `bounded_domain`/`model_population` node whose body is the one binding `max` of the integer literal `3` typed at the `Integer` node and whose `semantic_type` is the `Set<Reference<Order>>` node over the `Reference<Order>` node; those nodes are anonymous, the package holds exactly one `Type` occurrence of the population node with one `source_map` row, and IR's v2 reader admits the package and selects the population by the node and that occurrence. A bound of `0` emits the node with `max` `0`. | Test (TC-1054) |
| FR-380-AC-5 | When the walked population covers `Rush`, a subtype of `Order`, and the reachability's near-end type is `Order`, the node is that of `Population<Order>[N]`. A `Population<Order>[3]` parameter type in the same package and a bound of `3` walked by a reachability are one node holding two `Type` occurrences, ordinals 0 and 1 in ascending source order, each with one `source_map` row. | Test (TC-1054) |
| FR-380-AC-6 | A bound that no reachability walks, and a population with no bound, emit no population node, no `Type` occurrence for it and no link; the reachability of the second keeps the node identity it had before FR-380. | Test (TC-1054) |
| FR-380-AC-7 | An observation whose selected population holds `N` objects evaluates a clause over it as for an unbounded population, and one holding `N + 1` refuses `cardinality_out_of_bound`/`above-maximum` naming `N` and `N + 1` (`bound` `[0, N]`, `collection` `set`, `count` `N + 1`), as a resource limit never does. | Test (TC-1054) |
| FR-380-AC-8 | When the package bounds `orders` at `4` and a clause on `Order` holds `reaches(self, self, chief)` over the field `chief: Reference<Order>`, the emitter lowers it to a `quire.op.model.reaches_field` application whose `field` member holds `population`, the id of the `Population<Order>[4]` `bounded_domain`/`model_population` node, and whose `dependencies` hold that node. | Test (TC-1054) |
| FR-380-AC-9 | When the package bounds `orders` at `4` and a clause on `Order` holds `reaches(self, self, trail)` over the field `trail: Option<Reference<Order>>`, the emitter lowers it to a `quire.op.model.reaches_field` application whose `field` member holds `population`, the id of the `Population<Order>[4]` node, and whose `dependencies` hold that node. | Test (TC-1054) |
| FR-380-AC-10 | When the package bounds `orders` at `4` and a clause on `Order` holds `reaches(self, self, chain)` over the field `chain: Sequence<Reference<Order>>[0, 3]`, the emitter lowers it to a `quire.op.model.reaches_field` application whose `field` member holds `population`, the id of the `Population<Order>[4]` node, and whose `dependencies` hold that node. | Test (TC-1054) |
| FR-380-AC-11 | When the package bounds `orders` at `4` and a clause on `Order` holds `reaches(self, self, pair)` over the field `pair: Sequence<Reference<Order>>[2, 3]`, the emitter lowers it to a `quire.op.model.reaches_field` application whose `field` member holds `population`, the id of the `Population<Order>[4]` node, and whose `dependencies` hold that node; the package holds no `model_population` node of maximum `2` or `3`. | Test (TC-1054) |
| FR-380-AC-12 | When IR's strict v2 reader reads a package emitted under FR-380-AC-8, FR-380-AC-9, FR-380-AC-10, FR-380-AC-11 or FR-380-AC-15, the reader admits it and selects the `Population<Order>[4]` node and its least-ordinal `Type` occurrence as the population of the `reaches_field` application, including in the FR-380-AC-15 package, whose member `declaration` is the `Rush` node. | Test (TC-1054) |
| FR-380-AC-13 | When no `bound` names the population covering the edge's type, the emitter lowers a field-edge reachability with a `field` member holding exactly `kind`, `declaration` and `name`, no `population` key and no `model_population` node; over fixture R the `trail` application keeps the node id TC-1047 row 3 records. | Test (TC-1054) |
| FR-380-AC-14 | When populations `orders` and `staff` both have member type `Order` and the clause `FavLoop` of FR-380-AC-18 holds `reaches(self.favourite, self.favourite, trail)`, the checker refuses `ambiguous_declaration`/`ambiguous-name` at the `reaches`, outside the `StateModel` family, naming `orders` and `staff` in ascending `DeclarationKey` order. | Test (TC-1054) |
| FR-380-AC-15 | When a clause on `Rush` under `quire.model.complete/v1` holds `reaches(self, self, trail)` over the inherited `trail` and `orders` is bounded at `4`, the emitter links the application, whose member `declaration` is the `Rush` node, to the `Population<Order>[4]` node, the edge type's, not a `Population<Rush>[4]` node. | Test (TC-1054) |
| FR-380-AC-16 | When the package declares only the populations `rushes` (member type `Rush`, bounded at `4`) and `people`, and a clause on `Rush` under `quire.model.complete/v1` holds `reaches(self, self, trail)` over `trail: Option<Reference<Order>>`, the emitter lowers the application with no `population` key and emits no `Population<Rush>[4]` or `Population<Order>[4]` node: no population covers `Order`, and `rushes`, which covers the operand's type, is not walked. | Test (TC-1054) |
| FR-380-AC-17 | When the package declares only the populations `rushes` (member type `Rush`, bounded at `4` or unbounded) and `people`, and a clause on `Rush` under `quire.model.complete/v1` holds `reaches(self, self, trail)`, the checker admits the clause with no refusal and gives it a record whose only population domain is the context domain `rushes`. | Test (TC-1054) |
| FR-380-AC-18 | When the package bounds `orders` at `4` and `invariant FavLoop using V on M::Person at current { not reaches(self.favourite, self.favourite, trail) }` reads `favourite: Reference<Order>`, the checker gives the clause a record holding the unbounded context domain `people` and the walked domain `orders` as `Population(Some(4))`. | Test (TC-1054) |
| FR-380-AC-19 | When the clause of FR-380-AC-18 is evaluated over an observation whose `orders` population holds 5 objects, the evaluator refuses `cardinality_out_of_bound`/`above-maximum` naming `4` and `5` (`bound` `[0, 4]`, `collection` `set`, `count` `5`), and over one holding 4 it decides the clause as for an unbounded population. | Test (TC-1054) |
| FR-380-AC-20 | When one package holds `reaches(self, self, trail)` and `reaches(self, self, next)` over `Order/next` with `orders` bounded at `4`, the emitter writes the same `population` id in both applications and emits one population node holding one `Type` occurrence for the bound. | Test (TC-1054) |
| FR-380-AC-21 | When the package declares the populations `orders` (member type `Order`), `rushes` (member type `Rush`) and `people`, and `invariant RushFav using V on M::Person at current { not reaches(self.favRush, self.favRush, trail) }` under `quire.model.complete/v1` reads `favRush: Reference<Rush>`, the checker admits the clause with a record holding the domains `people` and `orders` and no `rushes` domain, with no `ambiguous-name` refusal. | Test (TC-1054) |

## Dependencies

- [FR-104](FR-104-check-state-clauses.md) owns the requirements records and
  the population domain this bound makes finite;
  [FR-365](FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md)
  owns the reachability over a relationship end, the population it walks and
  its link to the node; FR-104 and [FR-105](FR-105-emit-state-nodes.md) own
  the field-edge reachability and its `reaches_field` application, whose
  link "Field-edge reachability" adds;
  [FR-094](FR-094-key-model-owned-reference-population-and-quantity-nodes.md)
  owns the `Population<T>[N]`, `Reference<T>` node shapes this node reuses;
  [FR-102](FR-102-build-state-clause-forms.md)
  owns the leading-token table the `bound` spelling joins;
  [FR-333](FR-333-read-an-optional-collection-bound-and-population-maximum.md)
  owns the spelling of `N`;
  [FR-084](FR-084-admit-closed-populations-and-resolve-lookup.md) owns the
  above-maximum refusal; [FR-097](FR-097-classify-claim-extent-and-write-bounded-requests.md)
  owns the extent classification.
- QSpec FR-153 (the population declaration), FR-322 (the `population` member
  key of a `quire.op.model.reaches` application). PENDING: QSpec FR-322 today
  admits `population` on a `relationship_end` member only; admitting it on
  the `field` member of `quire.op.model.reaches_field`, checked against the
  field owner `T` and not the member's `declaration`, is the open amendment
  STD-663, and the IR reader change is IR-1533. FR-380-AC-12 reads through
  both.
- Linear QSL-819, QSL-1904.
