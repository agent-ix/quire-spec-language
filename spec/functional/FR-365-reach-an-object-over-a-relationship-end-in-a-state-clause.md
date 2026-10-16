---
id: FR-365
title: "Check, lower and evaluate reachability over a relationship end in a state clause"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-363
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-364
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-047
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: references
  - target: ix://agent-ix/quire-specification/FR-152
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
---
# FR-365: Check, lower and evaluate reachability over a relationship end in a state clause

## Description

When a state clause writes `reaches(a, b, name)` and `name` is a relationship
end of the operands' object type, the `StateModel` family's `check` hook SHALL
type it as a Boolean reachability over the links of that relationship, S4 SHALL
lower it to one `quire.op.model.reaches` application, and the evaluator SHALL
decide it from the observation's extent of the relationship alone, under the
`graph` charges of `quire.value.accounting/v1`.

Reachability is one or more edges: an object does not reach itself without a
cycle or a self-loop. The form is the one FR-104 gives a field edge, with the
third argument naming a relationship end instead of a field. Reachability over
a field edge is unchanged.

## Inputs

- The form `reaches(a, b, name)` in a state clause (an `invariant`, `pre` or
  `post` clause, FR-104), where `a` and `b` have static type `Reference<S>` and
  `name` is one segment (a qualified `name` stays refused at the form's build,
  FR-102).
- The unit's admitted model effective view and `TypeEnvironment`, including
  each relationship's direction and both ends' type, role and multiplicity
  ([FR-085](FR-085-resolve-relationship-end-references.md)).
- For evaluation: the observation's extents, population manifest and
  `resolve` read ([FR-364](FR-364-carry-relationship-instances-in-an-observation-and-read-them-for-navigation.md)),
  and `ScalarLimitsV1`.

## Outputs

A checked reachability node over a relationship end, holding both operands, the
relationship's declaration key and the end's role, with static type `Boolean`;
or a located `StateModel` refusal. After lowering, one
`quire.op.model.reaches` expression node. After evaluation, `true` or `false`
with the charges below, or one refusal or incomplete outcome and no Boolean.

## Behavior

### Resolution of the third argument

`name` is resolved on the static type `S` of `a` by FR-363's rule of the near
end: the relationship's `sourceEnd.role` names the forward end for receivers
conforming to the source end's type, and the `targetEnd.role`, when declared,
names the inverse end for receivers conforming to the target end's type. The
candidates are the field `name` of `S`'s effective view and every relationship
end navigable from `S` under that role. The form resolves to the one
candidate; otherwise:

- no candidate: `missing_declaration`/`missing-name` at the third argument, the
  refusal a name that is no field already gets;
- two or more candidates (a field and an end, or two ends):
  `ambiguous_declaration`/`ambiguous-name` at the third argument, naming every
  candidate.

When the candidate is a field, FR-104's field-edge rules apply and this
requirement does not. The rules below apply when it is a relationship end.

### Checking

The hook SHALL check, stopping at the first failure, each refusal
`ill_typed`/`operator-ineligible` at the form unless stated:

1. **Context.** `reaches` appears in a state clause; outside one it refuses as
   FR-104 states, and a layer that does not admit `reaches` refuses it as
   FR-110 states.
2. **Operands.** Both operands have a `Reference<_>` static type whose object
   type conforms to the near end's type `E` (FR-082, reflexive). An operand of
   another type, or an object type that does not conform to `E`, refuses.
3. **Direction.** The traversal is the one FR-363 allows: along the forward end
   under `source-to-target`, `bidirectional` or `undirected`, along the inverse
   end under `target-to-source`, `bidirectional` or `undirected`.
4. **Destination multiplicity.** A destination end whose upper bound is
   `unbounded`, whose multiplicity is `ordered: true`, or that declares no
   multiplicity refuses `unsupported_construct`/`expression-form`.
5. **Destination type.** The destination end is single-valued (`[0,1]` or
   `[1,1]`) and its type is `E` itself. A finite multi-valued destination end,
   and a destination end of another type, refuse.

Direction is decided before multiplicity and multiplicity before type, the
order QSpec FR-322-AC-57 applies to the lowered application, so the lowered
node satisfies those checks by construction. Every refusal's family is
`StateModel`. The result type is `Boolean`. A set-valued or bag-valued
destination end has no reachability in this form; its navigation (FR-363) is
unchanged.

### The population a reachability walks

A reachability over a relationship end walks the objects of `E` in the
relationship's extent. Its requirements record's extent (FR-104 "Requirements")
adds the population domain of the one population with no maximum that covers
`E` by conformance, keyed as FR-104 keys the context population; when that
population is the context population it adds no second domain. When no
population covers `E` the record names no domain for it. When `E` is a member
type of two or more populations with no maximum, the checker SHALL refuse
`ambiguous_declaration`/`ambiguous-name` at the form, naming those populations
in ascending `DeclarationKey` order.

### Lowering

S4 SHALL lower a checked reachability over a relationship end to one
expression node, FR-093's application node with `semantic_form` `reachability`:

| Part | Value |
| --- | --- |
| `operator` | `reaches` |
| `operation.identity` | `quire.op.model.reaches` |
| `operation.member` | `{kind: "relationship_end", declaration: <the relationship's model node>, name: <the end's role>}` (FR-094) |
| `operation.laws`, `operation.leaves`, `operation.mode` | none, none, `null` |
| `arguments` | `[ref(a), ref(b)]`, no `deref` node |
| `result_type` and `semantic_type` | the `Boolean` scalar type node |
| `dependencies` | the two operand nodes and the relationship's model node |

The node carries the relationship (its declaration node) and the end's role, so
the traversal direction and the near end are those the role names. It carries no
population, no bound and no observation: the walked population is the
requirements record's domain, and the observation is the clause kind's (FR-104
"Observations of reads"). A reachability over a field edge keeps lowering to
`quire.op.model.reaches_field` (FR-105). The two operation identities are the
two forms of one predicate, and a clause holding either is carried by a v2
package.

### What a clause claim carries for a proof

A clause's claim is its `state_clause` node (FR-105) and the `claim`
occurrence of its declaration. The condition term of that node is a tree of
nodes, and a reachability is a node of that tree. A proof that binds the
reachability SHALL bind the pair of the `state_clause` node's identity and the
path of argument positions from the clause's condition term to the
reachability node. The node itself identifies the relationship and the role;
the clause's requirements record, keyed by the `claim` occurrence, identifies
the population domain and its bound; the clause kind identifies the
observation. The path is positions and not node identities because nodes are
keyed by content: one reachability written twice in a condition is one node on
two paths, and the same reachability in two clauses is one node under two
clause identities, so the clause identity alone does not separate the second
case and the path alone does not separate the first.

### Evaluation

The evaluator SHALL decide `reaches(a, b, name)` over the extent `R` of the
resolved relationship in the clause's observation, reading only
`complete(R)`, `targets(R, x)` for a forward role, `sources(R, x)` for an
inverse role, and `resolve(x)` (FR-364 "The read contract"). It SHALL stop at
the first of the outcomes below, in this order.

1. Evaluate `a` and `b`. Decide `resolve(a)` with no charge: `dangling` refuses
   `dangling_reference`/`absent-target-in-complete-population`, retaining `a`
   and `R`; `undecidable` gives `incomplete_population`/`incomplete-scope`,
   retaining `a` and `R`. `b` is compared by whole-key identity (FR-364) and
   is not required to be held.
2. Charge `graph.expand` for `a`, which is discovered but not reached, and
   enqueue it.
3. If `complete(R)` is `false` (the extent is unlisted or incomplete), give
   `incomplete_population`/`incomplete-scope`, retaining `R` and `a`, with no
   further charge. A known link is not used to answer `true` from an incomplete
   extent, so a Boolean is returned only from a complete extent.
4. Dequeue nodes in discovery order. For each dequeued node `n`, take its
   neighbours in the order the read returns them (ascending reference-key
   order, repeats of one reference adjacent), and for each neighbour `t`, in
   order: decide `resolve(t)` with no charge, exactly as step 1 decides `a`
   (`dangling` refuses, `undecidable` is incomplete, retaining `t` and `R`,
   even when `t` is `b`); charge one `graph.edge`; when `t` equals `b`, the
   result is `true` and exploration stops; otherwise when `t` is not yet
   discovered, charge `graph.expand` for `t` and enqueue it.
5. When the queue is empty the result is `false`.
6. Charge `graph.result-retain` and return the Boolean. A refusal or incomplete
   outcome makes no later charge and returns no Boolean.

A neighbour that is repeated by a non-unique relationship is one `graph.edge`
charge per occurrence and is discovered once. The first neighbour in order
that is not `present` decides the outcome, `dangling` and `undecidable` alike,
as FR-364 decides it for a navigation. The evaluator SHALL NOT compare the
number of neighbours of a node with the destination end's multiplicity: an
observation is evidence of a state, and a breach of a declared multiplicity is
not this form's refusal. The search is iterative over an explicit queue and
imposes no path-length bound and no population bound of its own.

### Bounds and budgets

The charges are the `graph` rows of `quire.value.accounting/v1`: the kth
`graph.expand` charges `value_occurrences = k` (the number of distinct objects
this evaluation has discovered, `a` included) and `work_units += 1`; each
`graph.edge` charges `work_units += 1`; `graph.result-retain` charges
`work_units += 1` and `result_units += 1`. A relationship read charges nothing
(FR-364), and `model.navigate` is not charged. A denied charge returns the
incomplete outcome of its charge point (`limit_kind`, `limit`, `consumed`,
`next_charge` and `charge_point`) with no Boolean, and a zero limit is a real
limit.

Every accounting vector of the form completes at exactly its required counters
and stops incomplete one below any nonzero required counter, as FR-008-AC-16
states for the native evaluator. Repeated runs start with fresh counters,
discovered sets and queue, and neither a success nor an incomplete outcome
changes the observation or a later run's result.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-365-AC-1 | `reaches(a, b, name)` over a relationship end whose destination is `[0,1]` or `[1,1]` of the near end's own type checks as `Boolean` in an invariant, a precondition and a postcondition; a receiver of a subtype of the near end's type, an inverse end under `bidirectional` and under `undirected`, and a subtype operand beside a base operand each check; the field-edge form `reaches(self, self, via)` still checks and lowers as before. | Test (TC-1046) |
| FR-365-AC-2 | A name matching a field and a relationship end, or two ends, refuses `ambiguous_declaration`/`ambiguous-name`; a name matching neither refuses `missing_declaration`/`missing-name`. | Test (TC-1046) |
| FR-365-AC-3 | An operand of a type that does not conform to the near end's type, a forward traversal against `target-to-source`, an inverse traversal against `source-to-target`, a finite multi-valued destination end, a destination end of another type and `reaches` in a function body each refuse `ill_typed`/`operator-ineligible`; an unbounded, ordered or multiplicity-less destination end refuses `unsupported_construct`/`expression-form`; direction is reported ahead of multiplicity and multiplicity ahead of destination type. | Test (TC-1046) |
| FR-365-AC-4 | A reachability over `E` whose type is a member type of two populations with no maximum refuses `ambiguous_declaration`/`ambiguous-name` at the form, naming both populations; with one covering population the requirements record adds that domain once and none when it is the context population. | Test (TC-1046) |
| FR-365-AC-5 | A checked reachability over a relationship end lowers to one expression node with `semantic_form` `reachability`, `operator` `reaches`, `operation.identity` `quire.op.model.reaches`, member `relationship_end` naming the relationship's model node and the role, no laws, leaves or mode, arguments `[ref(a), ref(b)]` and a `Boolean` `result_type`, with no `deref` node, and IR's v2 reader admits the emitted package. | Test (TC-1047) |
| FR-365-AC-6 | The lowered node carries no population, bound or observation. The same reachability in two clauses is one node under two `state_clause` identities; one reachability written twice in a condition is one node on two argument paths; two different roles or relationships give two nodes. | Test (TC-1047) |
| FR-365-AC-7 | Reachability is one or more edges over the extent: a chain reaches its far end and not its start, an isolated object does not reach itself, a self-loop and a cycle return to their start, a cycle that holds no target and an empty complete extent give `false`, and two distinct equal-valued objects are not the same target. | Test (TC-1048) |
| FR-365-AC-8 | Every graph on one to three objects, with each object holding no outgoing link or one, agrees with an independently authored transitive-closure oracle for every ordered pair, over a forward and an inverse role, and a non-unique relationship that repeats a link gives the same Boolean as one occurrence. | Test (TC-1048) |
| FR-365-AC-9 | The charges are the `graph.expand`, `graph.edge` and `graph.result-retain` schedule in discovery order with the stated `value_occurrences`, a repeated link charges one `graph.edge` per occurrence and is discovered once, and the exact required `work_units` and `value_occurrences` complete while one less is incomplete at the stated charge point. | Test (TC-1048) |
| FR-365-AC-10 | An unlisted extent and an incomplete extent each give `incomplete_population`/`incomplete-scope` after the start's `graph.expand` and no further charge, even when a known link would reach the target; a complete empty extent gives `false`. | Test (TC-1048) |
| FR-365-AC-11 | A dangling neighbour of a model-layer extent refuses `dangling_reference`/`absent-target-in-complete-population` before its `graph.edge`, including when it is the target `b`; an undecidable neighbour gives `incomplete_population`/`incomplete-scope` at the same point; the first neighbour in order that is not present decides; a dangling or undecidable start decides before any charge. | Test (TC-1048) |
| FR-365-AC-12 | A neighbour count above or below the destination end's bound refuses nothing; a reachability in a postcondition reads the post-observation extent and in a precondition the pre-observation extent; two evaluations of one reachability give equal results and charges and leave the observation equal to what it was. | Test (TC-1048) |

## Dependencies

- **Upstream:** [FR-104](FR-104-check-state-clauses.md) owns the state clause,
  the `reaches` field form, its context rule and the requirements record;
  [FR-363](FR-363-type-navigation-over-a-relationship-end.md) owns the
  near-end role resolution and the direction rule this form reuses;
  [FR-364](FR-364-carry-relationship-instances-in-an-observation-and-read-them-for-navigation.md)
  owns the extent reads and `resolve`; [FR-105](FR-105-emit-state-nodes.md),
  [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md) and
  [FR-094](FR-094-key-model-owned-reference-population-and-quantity-nodes.md)
  own the clause node and the application and model nodes it names;
  [FR-082](FR-082-resolve-conformance-subsetting-and-redefinition.md) supplies
  conformance. [FR-047](FR-047-evaluate-finite-object-reference-graphs.md) and
  [FR-008](FR-008-evaluate-state-reference.md) own the field-edge reachability
  semantics and the independent-oracle and exact-budget criteria this form
  repeats over relationship extents. QSpec FR-322-AC-57 owns the lowered
  application's checks and FR-152 the direction rule.
- **Downstream:** none.
