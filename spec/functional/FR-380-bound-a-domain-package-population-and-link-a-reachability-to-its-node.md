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
state-clause `reaches` over a relationship end walks a population that FR-104
records as unbounded, and a proof backend that needs a finite population
cannot settle the claim. This requirement lets the author state a finite
maximum for a population in source with a `bound` declaration, makes the
population a bounded domain of the requirements records that range over it,
and, for each reachability that walks a bounded population, emits the one
`bounded_domain`/`model_population` node a reader selects the population by,
which FR-365 links the reachability to.

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
- For each reachability over a relationship end that walks a bounded
  population, the population node and the `Type` occurrence the node holds
  for the bound (see "The population node").
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
are `bound` `[0, N]`, `collection` `set` and `count`. It applies to the
population a relationship-end `reaches` walks. A field-edge `reaches` walks
object fields and no population, and `allInstances` reads the snapshot listing,
so neither is bounded by this declaration.

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
authored bound has no node and no link. The reachability's link to the node is
FR-365 "The population link".

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-380-AC-1 | `bound HistoryBound on Config::config_history [8]` builds one form with label `HistoryBound`, alias `Config`, population `config_history` and `N` `8`, each with a span that slices to its source text; `bound` as an identifier parses in member position only (`b.bound`); `[0]` is accepted and `[-1]` and `[x]` refuse at S1. | Test (TC-1054) |
| FR-380-AC-2 | An alias naming no model and a population name naming no population of the package each refuse `missing_declaration`/`missing-name` at the name; `[18446744073709551616]` refuses as FR-333 refuses it; a second bound with a used label, and two bounds naming one population (equal or different `N`), each refuse `ambiguous_declaration`/`ambiguous-name` at both declarations. | Test (TC-1054) |
| FR-380-AC-3 | With a bound `N` on `config_history`, the requirements record of a clause over `ConfigVersion` holds the `config_history` domain as a `Population(Some(N))` domain with available finite bound `Cardinality{N}`, listed among no unbounded domain, for a context population and for a population a reachability walks; the clause extent is `Bounded` when no other domain is unbounded; two clause orders and two unit orders give equal records; without the bound the record is the unbounded one. | Test (TC-1054) |
| FR-380-AC-4 | A reachability over a relationship end of `Order` that walks a population bounded at `3` lowers with a `bounded_domain`/`model_population` node whose body is the one binding `max` of the integer literal `3` typed at the `Integer` node and whose `semantic_type` is the `Set<Reference<Order>>` node over the `Reference<Order>` node; those nodes are anonymous, the package holds exactly one `Type` occurrence of the population node with one `source_map` row, and IR's v2 reader admits the package and selects the population by the node and that occurrence. A bound of `0` emits the node with `max` `0`. | Test (TC-1054) |
| FR-380-AC-5 | When the walked population covers `Rush`, a subtype of `Order`, and the reachability's near-end type is `Order`, the node is that of `Population<Order>[N]`. A `Population<Order>[3]` parameter type in the same package and a bound of `3` walked by a reachability are one node holding two `Type` occurrences, ordinals 0 and 1, each with one `source_map` row. | Test (TC-1054) |
| FR-380-AC-6 | A bound that no reachability walks, and a population with no bound, emit no population node, no `Type` occurrence for it and no link; the reachability of the second keeps the node identity it had before FR-380. | Test (TC-1054) |
| FR-380-AC-7 | An observation whose selected population holds `N` objects evaluates a clause over it as for an unbounded population, and one holding `N + 1` refuses `cardinality_out_of_bound`/`above-maximum` naming `N` and `N + 1` (`bound` `[0, N]`, `collection` `set`, `count` `N + 1`), as a resource limit never does. | Test (TC-1054) |

## Dependencies

- [FR-104](FR-104-check-state-clauses.md) owns the requirements records and
  the population domain this bound makes finite;
  [FR-365](FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md)
  owns the reachability, the population it walks and the link to the node;
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
  key of a `quire.op.model.reaches` application).
- Linear QSL-819.
