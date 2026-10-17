---
id: TC-1047
title: "A checked reaches over a relationship end lowers to one quire.op.model.reaches node a proof can bind"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-365
    type: verifies
---
# TC-1047: A checked reaches over a relationship end lowers to one quire.op.model.reaches node a proof can bind

## Description

Verify the lowered node of a reachability over a relationship end and what a
proof binds to reach it. Scope: FR-365-AC-5, FR-365-AC-6 (a reachability over an unbounded population; the link of a bounded one is TC-1054's).

Catches a lowering that emits `quire.op.model.reaches_field` for an end, names
a field member, routes an operand through `deref`, carries a population or an
observation in the node, or keys the node by its clause.

## Test Procedure

Over TC-1046's fixture **R**, check, lower and emit the units below, and read
each emitted package with IR's v2 reader.

| # | Input | Expected |
| --- | --- | --- |
| 1 | `invariant NoLoop using V on M::Order at current { not reaches(self, self, next) }` | the `state_clause` node's condition holds a `not` application over one expression node with `semantic_form` `reachability`, `operator` `reaches`, `operation.identity` `quire.op.model.reaches`, `laws` `[]`, `leaves` `[]`, `mode` `null`, member `{kind: "relationship_end", declaration: <the `Order/next` model node>, name: "next"}`, arguments `[reference(self's parameter node), reference(self's parameter node)]`, `result_type` and `semantic_type` the `Boolean` scalar type node; no `quire.op.model.deref` node; `dependencies` list the parameter node and the relationship node; the reader admits the package |
| 2 | row 1 with `incoming` over `Order/link` and with `mateOf` over `Order/mate` | member `declaration` the `Order/link` node and `name` `incoming`; the `Order/mate` node and `mateOf`: the role, not the relationship alone, is named, so the traversal direction is the role's |
| 3 | `reaches(self, self, trail)` | a `quire.op.model.reaches_field` application whose member is `{kind: "field", declaration: <Order node>, name: "trail"}`, unchanged |
| 4 | invariants `A` as `not reaches(self, self, next)` and `B` as `reaches(self, self, next)` on `M::Order` | one reachability node, referenced by two `state_clause` nodes whose conditions differ; each has its own identity and its own `claim` occurrence |
| 4b | invariants `A` and `B` on `M::Order`, each `not reaches(self, self, next)` | one `state_clause` node (kind, parameters, anchor and condition are equal; the clause name is no part of it) holding one reachability node, with two `claim` occurrences, ordinals 0 and 1 in source order |
| 5 | one invariant `reaches(self, self, next) and reaches(self, self, next)` | one reachability node on two argument paths of the condition (positions 0 and 1 of the `and` application) |
| 6 | invariants `reaches(self, self, next)` and `reaches(self, self, outgoing)` | two reachability nodes with different identities |
| 7 | the node of row 1, over the unbounded `orders` population | its body names no population, no bound and no observation, and its `operation.member` holds exactly `kind`, `declaration` and `name`; the clause's requirements record, keyed by the `claim` occurrence, holds the `orders` population domain, and the `state_clause` node's anchor and kind name the observation. The bounded population is TC-1054's |
| 8 | row 1 with `kids` over `Order/kids` (target end `{0,3}` unique) and with `litter` over `Order/litter` (target end `{0,3}`, `unique: false`) | the same node shape as row 1: one `quire.op.model.reaches` application with member `relationship_end` naming the relationship's node and the role, arguments `[ref(a), ref(b)]`, a `Boolean` result type, no `deref`; IR's v2 reader admits the package (QSpec FR-322-AC-57) |

## Expected Results

Each reachability over an end is one `quire.op.model.reaches` application with
the member, arguments and result type stated, and the package reads. A proof
identifies the reachability by the `state_clause` identity and the argument-
position path from its condition term; the node supplies the relationship and
the role, the requirements record the population domain.
