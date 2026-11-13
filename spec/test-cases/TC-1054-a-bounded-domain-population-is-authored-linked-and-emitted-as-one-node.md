---
id: TC-1054
title: "A bounded domain-package population is authored, made finite and linked from its reachability as one node"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-380
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-365
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: verifies
---
# TC-1054: A bounded domain-package population is authored, made finite and linked from its reachability as one node

## Description

Verify the `bound` declaration, the finite domain it gives the requirements
records, the population node a reachability over a relationship end walks and
the link from the reachability, over a relationship end or a field edge, to
that node. Scope: FR-380-AC-1 to FR-380-AC-21,
FR-365-AC-6, FR-365-AC-13, FR-104-AC-11.

Catches a bound that is parsed and then ignored by the requirements record, a
population node built over the population's canonical member type instead of
the near end's type, a link written as `null` or a second node for the same
`(E, N)`, a link for an unbounded population (which re-keys every existing
reachability node), a bound that emits a node no reachability walks, a
bound converted into a resource outcome, a field-edge reachability emitted
without its link, and a link derived from a sequence's `[l, u]` or from the
operand's subtype instead of the edge's type, a walked population taken
from the operand's type, and a field-edge walked population left out of the
record or the above-maximum check.

## Test Procedure

Over TC-1046's fixture **R** (populations `orders` of `Order`, with `Rush` a
subtype, and `people` of `Person`, both with no maximum), model alias `M` and
profile alias `V`, compile one package per row and emit it, then read it with
IR's v2 reader. `Order/next` is the relationship of TC-1046's table. Rows 15
to 28 add to `Order` the fields `chief: Reference<Order>`,
`chain: Sequence<Reference<Order>>[0, 3]` and
`pair: Sequence<Reference<Order>>[2, 3]` beside R's
`trail: Option<Reference<Order>>`, and to `Person` the fields
`favourite: Reference<Order>` and `favRush: Reference<Rush>`. Their bound is
`4`, unequal to every sequence `l` and `u`. **FavLoop** is
`invariant FavLoop using V on M::Person at current { not reaches(self.favourite, self.favourite, trail) }`.

| # | Input | Expected |
| --- | --- | --- |
| 1 | the unit `bound OrdersBound on M::orders [8]` | one form, label `OrdersBound`, alias `M`, population `orders`, `N` `8`, each span slicing to its source text; `bound` used as an identifier in member position (`b.bound`) parses; `[0]` builds; `[-1]` and `[x]` refuse at S1 |
| 2 | `bound X on Nope::orders [8]`, `bound X on M::nothing [8]`, `bound X on M::orders [18446744073709551616]` | `missing-name` at `Nope`, `missing-name` at `nothing`, the FR-333 refusal of an `N` outside `u64` |
| 3 | `bound X ...` twice with equal labels; two bounds on `orders` with `[8]` and `[8]`; with `[8]` and `[9]`; a bound labelled like an existing clause | `ambiguous_declaration`/`ambiguous-name` at both declarations each time |
| 4 | `bound OrdersBound on M::orders [3]` with `invariant NoLoop using V on M::Order at current { not reaches(self, self, next) }` | the clause's requirements record holds the `orders` domain as `Population(Some(3))` with available finite bound `Cardinality{3}` and no unbounded domain; extent `Bounded`; the same unit without the bound gives the unbounded record of TC-1046 and extent `Unbounded`; the unit's declarations in reverse order give equal records |
| 5 | row 4 | the reachability application's `operation.member` holds `kind` `relationship_end`, `declaration` the `Order/next` node, `name` `next` and `population` the id of one `bounded_domain`/`model_population` node; that node is anonymous, its body is the one binding `max` of the integer literal `3` typed at the `Integer` node, its `semantic_type` is the `Set<Reference<Order>>` node over the `Reference<Order>` node, its `dependencies` are exactly the `Set` node (FR-322's dependency rule: a `bounded_domain` node lists its `semantic_type`), the `Set` node lists exactly the `Reference<Order>` node and that node lists exactly the `Order` node, the application's `dependencies` hold the node, and the package holds exactly one `Type` occurrence of it with one `source_map` row at the `[3]` span; IR's v2 reader admits the package and its population selection returns that node and the `Type` occurrence |
| 6 | row 4 with `[0]` | the node with `max` `0`; admitted and selected |
| 7 | row 4 without the `bound` | the reachability's member holds exactly `kind`, `declaration` and `name`, with no `population` key; the package holds no `model_population` node; the application's node id equals the one TC-1047 row 1 records |
| 8 | row 4 with a second invariant `reaches(self, self, outgoing)` over `Order/link` | both applications carry the same `population` id; the package holds one population node |
| 9 | `bound OrdersBound on M::orders [3]`, `bound PeopleBound on M::people [2]` and the invariant of row 4 | the reachability carries the `orders` link only; no node is emitted for `people`, which no reachability walks, and no `Type` occurrence for its `[2]` |
| 10 | `bound OrdersBound on M::orders [3]` and `invariant NoLoopRush using V on M::Rush at current { not reaches(self, self, next) }` | the node is the `Population<Order>[3]` node of row 5, not a `Population<Rush>[3]` node: the near end's type `Order`, not the clause's context type, names it |
| 11 | row 4 plus `function f using V(p: Population<M::Order>[3]): Integer pure { 0 }` | one `model_population` node holding two `Type` occurrences, ordinals 0 and 1 in ascending source order, one `source_map` row each; the link names the node |
| 12 | row 4 plus a bound `[3]` on a second population `staff` with member type `Order` | the clause refuses `ambiguous_declaration`/`ambiguous-name` at its `on` naming `orders` and `staff` (FR-104-AC-5); a reachability-only claim over `E` `Order` refuses the same at the form |
| 13 | a bound `[2]` on `orders`; evaluate the clause of row 4 over an observation whose `orders` population holds 2 objects, and over one holding 3 | the first is decided as for an unbounded population; the second refuses `cardinality_out_of_bound`/`above-maximum` naming 2 and 3 (`bound` `[0, 2]`, `collection` `set`, `count` `3`), not an incomplete or resource outcome; a clause over the bounded context population alone, and one whose `reaches` a short-circuit skips, refuse the same |
| 14 | `bound OrdersBound on M::orders [3]`, `bound PeopleBound on M::people [2]`, the invariant of row 4 and `invariant NoFriendLoop using V on M::Person at current { not reaches(self, self, friend) }` | the two reachability applications carry different `population` ids; the package holds two population nodes, each linked once, with one `Type` occurrence each, at `[3]` and `[2]` |
| 15 | `bound OrdersBound on M::orders [4]` with `invariant ChiefLoop using V on M::Order at current { not reaches(self, self, chief) }` | a `quire.op.model.reaches_field` application whose `field` member holds `kind` `field`, `declaration` the `Order` node, `name` `chief` and `population` the id of the `Population<Order>[4]` node, which its `dependencies` hold |
| 16 | row 15 with `trail` in place of `chief` | the same, with `name` `trail`, the same `population` id and the node in `dependencies` |
| 17 | row 15 with `chain` | the same, with `name` `chain`, the same `population` id and the node in `dependencies` |
| 18 | row 15 with `pair` | the same, with `name` `pair`, the same `population` id and the node in `dependencies`; no `model_population` node of maximum `2` or `3` |
| 19 | the packages of rows 15 to 18 and 23, read by IR's strict v2 reader | each admitted; the population selection of the `reaches_field` application returns the `Population<Order>[4]` node and its least-ordinal `Type` occurrence, in row 23 with the member `declaration` the `Rush` node |
| 20 | rows 15 to 18 without the `bound` | each member holds exactly `kind`, `declaration` and `name`; no `model_population` node; row 16 without the bound, over R without the added fields, gives the application node id TC-1047 row 3 records |
| 21 | **FavLoop** with a second population `staff` of member type `Order` | `ambiguous_declaration`/`ambiguous-name` at the `reaches`, family not `StateModel`, naming `orders` and `staff` in ascending `DeclarationKey` order |
| 22 | row 16 plus `invariant NoLoop using V on M::Order at current { not reaches(self, self, next) }` in one package | both applications carry the same `population` id; the package holds one population node with one `Type` occurrence at the `[4]` span |
| 23 | `bound OrdersBound on M::orders [4]` under `quire.model.complete/v1` with `invariant RushTrail using V on M::Rush at current { not reaches(self, self, trail) }` | the member `declaration` is the `Rush` node; the link names the `Population<Order>[4]` node, not a `Population<Rush>[4]` node |
| 24 | R with `orders` replaced by `rushes` of member type `Rush`, `bound RushesBound on M::rushes [4]`, and row 23's invariant | no `population` key; no `Population<Rush>[4]` or `Population<Order>[4]` node |
| 25 | row 24, and row 24 without the bound | each clause admitted; the record's only population domain is `rushes`, the context domain; no refusal |
| 26 | `bound OrdersBound on M::orders [4]` and **FavLoop** | the record holds `people` as an unbounded domain and `orders` as `Population(Some(4))` |
| 27 | row 26, evaluated over an observation whose `orders` population holds 4 objects, and over one holding 5 | the first is decided as for an unbounded population; the second refuses `cardinality_out_of_bound`/`above-maximum` naming 4 and 5 (`bound` `[0, 4]`, `collection` `set`, `count` `5`) |
| 28 | R plus `rushes` of member type `Rush`, under `quire.model.complete/v1`, with `invariant RushFav using V on M::Person at current { not reaches(self.favRush, self.favRush, trail) }` | admitted; the record holds `people` and `orders` and no `rushes`; no `ambiguous-name` refusal |

## Expected Results

An authored bound makes the population a finite domain of the records that name
it, and nothing else changes for an unbounded population. A reachability, over
a relationship end or a field edge, over a bounded population links to one node of `(E, N)`, the node every writer of that
`(E, N)` shares, with the occurrence rule of FR-365 "The population link".
