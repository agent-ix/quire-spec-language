---
id: FR-094
title: "Key model-owned, reference, population, clause-function and quantity type nodes"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-005
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: traces_to
---
# FR-094: Key model-owned, reference, population, clause-function and quantity type nodes

## Description

[FR-092](FR-092-key-type-parameter-and-declared-nodes.md) keys the `Value`
family's type, value, parameter and function nodes, and
[FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md) lowers
checked expressions to FR-322 application nodes. Four kinds of node those
requirements reference get their keys here:

1. the `model` and `relation` nodes of a domain package's declarations, which
   the `Reference<T>` type and the `field`, `operation`, `relationship_end`
   and `type_argument` members of FR-322 name;
2. the `Reference<T>` and `Population<T>[N]` type nodes of the `StateModel`
   family;
3. the clause functions `check` synthesizes from a domain package's
   operation clauses (`check::checked_dispatch`, FR-151 dispatch);
4. the type node of a quantity, for a declared unit and for a compound unit.

Every node here is identified by QSpec FR-322's structural node id
`{package, path, ordinal}` ("Structural node identity"), compared
componentwise by its derived `Ord` and never hashed. A node that stands for a
domain package's declaration carries QSpec's `ModelOwner` as its `owner`
relation, as ADR-013 O-04 and C-02 decide, and the owner is no part of the
node id. The producer (`check`) assigns every `ordinal`, which is `0` unless
two nodes share one `package` and `path`; a reader derives none. The node
shapes, the `model_population` and `compound_unit` shapes and the `clause`
binding are QSL proposals to QSpec (ADR-013 QC-25, QC-26). QSpec references (`ix://agent-ix/quire-specification`, cited by
reference, never copied): FR-322, `proposals/checked-package-v2/schema.json`,
`operation-catalog.json` and the positive fixtures.

## Inputs

- For each admitted domain package: its model selection's
  `DomainPackageRef` identity and its effective view, whose
  `type_identities` map each top-level `DeclarationKey{package, node}` to its
  `EffectiveId` (`qsl-semantics/src/model/normalize.rs`, ADR-013 O-03, O-05).
- The checked types: `ValueType::Reference(EffectiveId)`, the resolved type
  form of a `Population<T>[N]` parameter (its object type `T` and its maximum
  `N`), and `ValueType::Quantity(UnitId)` with the package's unit table.
- For each clause function: the `DeclarationKey` of the operation member
  whose clause it is, and its `DeclaredClauseKind`.

## Outputs

- Each node's structural key, and the model correspondence entries (node key,
  `DeclarationKey`).
- Or a typed refusal and no key.

## Behavior

### Model-owned nodes

A **model-owned node** is a model declaration node or a clause function
node, both defined below. `check` SHALL identify a model declaration node by
the FR-322 structural id with these members:

- `package`: the identity of the domain package that declares it, the
  selection whose identity equals the `DeclarationKey`'s `package`.
- `path`: `[node_tag, semantic_form, node]`, `node` being the
  `DeclarationKey`'s `node`, the IR node identity (for example
  `ix://acme/orders/Order`), as in the model root's `[model, model_root]`.
- `ordinal`: `0`, no two model declaration nodes sharing a `package` and
  `path`.

Its `owner` is QSpec's content-only `ModelOwner`, `{kind: "model", identity,
node}`, with no `version` member (QSpec FR-322-AC-28), the same `identity` and
`node`; it is a relation to the declaring domain package and no part of the id.
The node has no `declaration`: its name belongs to the domain package, not to
a QSL source declaration (FR-322 `declaration`).

A domain package selects one version of an identity per check (ADR-013 O-01,
QSpec FR-321), so within one check `package` and `node` name exactly one
declaration. A domain package's version is no part of the id, so a change of
the domain package that leaves a node's content unchanged yields the same
model-owned node ids. The v2 emission writes a model-owned node's `ModelOwner`
as its wire `owner` member (FR-093).

### Model declaration nodes

`check` SHALL build the node of a domain package declaration as follows:

| Declaration record | `node_tag` / `semantic_form` | `semantic_type` | Body |
|---|---|---|---|
| `ObjectTypeRecord` with no `interfaceFeatures` | `model` / `object_type` | itself | `aggregate{[]}` |
| `ObjectTypeRecord` with `interfaceFeatures` (FR-152 Interface) | `model` / `systems_interface` | itself | `aggregate{[]}` |
| `StateMachineRecord` | `model` / `state_machine` | itself | `aggregate{[]}` |
| `RelationshipRecord` | `relation` / `relationship` | itself | `aggregate{[]}` |
| `RecordValueTypeRecord` | `model` / `record_value_type` | itself | `aggregate{[]}` |
| `EventTypeRecord` | `model` / `event_type` | itself | `aggregate{[]}` |

A record value type or event type is never a `composite_type` with a source
owner (FR-359 item 3). Its model node's key is also the declaration key under
which the type environment declares it, so a value of the record names its
most-specific type by that key, whatever alias a unit gives the model
(FR-359 item 9).

A record kind that no checked node names has no node: a field member, an
operation member, a scalar type, a component, an endpoint, an allocation and
a population record. The `match` over record kinds gives each of them an arm
that is an internal fault (ADR-013 T-4). An `ObjectTypeRecord` whose
`interfaceFeatures` is present is `systems_interface` even when the list is
empty. A `RelationshipRecord` is `relation` / `relationship` whatever FR-152
role it also plays, because a checked node names it only through a
`relationship_end` member.

The body is empty because the owner already determines the declaration's
content: intake admits one selection of an identity per check (QSpec FR-321).
Across checks, the node id is the same for every version selected under one
`identity`. An empty body also keeps model declaration
nodes out of recursion groups, so two object types that reference each other
through `Reference` attributes key without a cycle.

`check` builds the node of each model declaration that a checked node or a
frame subject (ADR-013 O-08) names: the target of a `Reference<T>` type, the
declaring node of a `field`, `operation` or `relationship_end` member, a
`type_argument` member's type, and a `modifies`, `creates` or `deletes`
subject. A field, operation or relationship-end member has no node of
its own; an FR-322 member names it by its declaring node and its name
(ADR-013 O-06):

A selected `RelationshipRecord` is an unconditional exception to that
reachability rule. `check` roots every selected relationship declaration even
when no checked value, function, frame or relationship-end member names it,
and records one `generated` occurrence at the selected model alias's own
declaration-site/source span. The fallback `generated_location` is never used
for this root, and no relationship root may reach emission as an
`UnlocatedOccurrence`. For example, the `billedTo` relationship in the
unreferenced model fixture is emitted from the `M` alias declaration site
even when the fixture has no function that names it.

- the `field` member of an `Attribute` (`deref(r).f`) names the model node of
  `r`'s static object type `T`, the target of `r`'s `Reference<T>` type, and
  the attribute name `f`;
- the `operation` member of a `Dispatch` (`r.m(...)`) names the model node of
  the receiver's static object type `T` and the member name `m`. FR-151
  resolves `m` statically to the one effective operation `T` exposes, so the
  member names `T`'s node even when a supertype of `T` declares `m`
  (ADR-013 O-06);
- a `relationship_end` member of a `Navigate` (`r.name`, FR-363) names the
  relationship's node and the end's role;
- the `type_argument` member of `allInstances<T>` and `lookup<T>` names the
  model node of `T`.

A `Reference<T>` type carries `T` as an `EffectiveId` (ADR-013 O-05).
`check` maps it to the `DeclarationKey` that the effective view's
`type_identities` pairs with it, and keys the node by that `DeclarationKey`.
No node id member is computed from an `EffectiveId`, so a model declaration's
node id depends on its domain package and IR node identity, not on the
normalization rules' derivation facts. The object types of the checked type
environment are exactly the admitted effective views' `type_identities`
values, so every admitted `Reference<T>` maps to one `DeclarationKey`.

A model declaration node's body names no member, so the type of a `field` or
`operation` member is not in the graph. ADR-013 QC-25 asks QSpec how a
reader derives it for FR-322's `member` result form and `member_of`
constraint. The `Attribute` projection over a `deref` result is QC-24's.

### Model correspondence

`check` SHALL record, for each model declaration node it keys, the pair
(node key, `DeclarationKey`) in the checked package's model correspondence
(ADR-013 O-04, C-02). `check` is the correspondence's only writer, and the
correspondence holds exactly the pairs `check` keyed: each `DeclarationKey` it holds has one node key, and each node key one
`DeclarationKey`. Clause function nodes are not correspondence entries. The
v2 spelling of the correspondence is ADR-013 QC-25.

### Reference and Population type nodes

`check` SHALL build these `StateModel` type nodes. It builds a
`Population<T>[N]` node from the resolved type form, which names `T`, so
`Population<Order>[3]` and `Population<Invoice>[3]` are different nodes:

| Checked type | `node_tag` / `semantic_form` | `semantic_type` | Body |
|---|---|---|---|
| `Reference<T>` | `composite_type` / `reference` | itself | `aggregate{[reference(T's model node)]}` |
| `Population<T>[N]` | `bounded_domain` / `model_population` | the `Set<Reference<T>>` node | binding `max` = `N` |

`N` is an integer literal typed at the `Integer` node (FR-092 T2), spelled as
FR-092 spells an integer. Both nodes are anonymous derived nodes: they carry no
`declaration` and no `owner` (FR-092, ADR-013 OQ-G). Their `package` is the
checked package's, and their `path` is `[node_tag, semantic_form,
...operands]` (FR-322): `[composite_type, reference, T's model node id]` for
`Reference<T>`, and `[bounded_domain, model_population, T's model node id,
N]` for `Population<T>[N]`, `N` as its decimal text. The owner scope comes
through `T`'s model node id, so `Reference<T>` over object types of two domain
package identities gives two ids, and a version-only change of `T`'s domain
package keeps the id.

A type built from these follows FR-092's table: the `lookup<T>` result under
`absent empty` is the `Option<Reference<T>>` node, and the `allInstances<T>(p)`
result for a population of maximum `N` is the `collection_bounds` node of
`Set<Reference<T>>[0, N]` (FR-084).

A population the domain package declares and a state clause names as `M::P`
(FR-391) is not a `Population<T>[N]` type node. Its reference is a separate
node keyed by the population's declaration key alone, with no `N`, whose
id is the model node id of that key and whose
`owner` is the model owner of that key (QSpec FR-322): two populations of one
member type are two nodes, and the same population read twice is one node. QSpec FR-322 ("Population references")
defines that node, and FR-391 states how `check` emits it.

### Clause function nodes

A **clause function** is a function declaration that `check` synthesizes
from a domain package clause (`FunctionDeclaration::clause`), and no named
call reaches it. For one dispatch-eligible operation,
`check::checked_dispatch` builds:

- one function per authored precondition clause, shared by every candidate
  whose effective precondition includes it;
- one effective-precondition function for a candidate whose effective
  precondition has two or more terms, their disjunction with the candidate's
  own clause first; a candidate with one term uses its authored function;
- one body function per candidate.

`check` SHALL build a clause function's node as FR-092's function node, with
these differences:

- `owner`: the `ModelOwner` of the declaration whose clause it is: the
  authoring operation member for an authored precondition, the candidate
  operation member for an effective precondition and for a body, and the
  object type for an invariant.
- `declaration`: `null`, as for every model-owned node.
- body: FR-092's `parameters`, `body` and `decreases` bindings, followed by
  binding `clause` = the `DeclaredClauseKind` as a text literal typed at the
  text scalar node (FR-092 T3): `"precondition"` for an authored or
  effective precondition, `"body"` for a body, and `"invariant"` for an
  invariant. The `match` that spells it is over `DeclaredClauseKind`, which
  has no postcondition arm.

An operation's postconditions are lowered by the `ProtocolClause` family
with its checked operation header (ADR-012 §3, §4.3; #218), which gives each
its node, owner and clause binding. The `Pre` reads inside a postcondition
key by FR-093's `Pre` row. ADR-012 §1 assigns the precondition subnode to
`ProtocolClause` as well; #218 decides whether precondition clause
functions move from this requirement.

The name `check` gives a clause function (such as
`ix://acme/orders/Order/size.precondition`) is a lookup label and no part of
the node id, except as the owner's `node` where the two coincide: a body
function's label is its candidate's IR node identity. Renaming the label
leaves the node id unchanged.

The clause functions of one owner share one `package` and `path` (FR-322
"Structural node identity") and `check` assigns their `ordinal`s in the order
this section lists them: authored precondition `0`, effective precondition
`1`, body `2`; an invariant is `0`. No reader derives an ordinal.

A redefining candidate's own clauses are built both by its own dispatch
operation and as a candidate of the operation it redefines, and `check`
yields one node for each clause, with one node id each time.

A clause function carries `ModelOwner`, not `SourceOwner` or
`DefinitionOwner`, because the domain package declares its clause, not a QSL
source unit and not a definition document. The owner names one
member of one domain package identity, and the `clause` binding tells one
member's precondition and body apart. A `dispatch_call` reaches its
candidates' clause functions through the checked package's dispatch tables,
not through a graph reference; the v2 spelling of that link is QC-25.

### Quantity type nodes

`check` SHALL build the type node of a `Quantity(UnitId)` by the `UnitId`'s
domain:

- **Declared unit.** The type node is the unit's QSpec nominal node
  (`scalar_type` / `unit`, FR-092). It is the
  quantity's `semantic_type`, `result_type` and literal `type` directly. No
  other node stands for the quantity type. `check` builds it, and every
  dimension and unit node it names (its dimension, that dimension's base
  dimensions, its target units), from the admitted node the unit table
  holds for each, as QSpec's nominal fixtures spell them (a dimension or unit
  a domain package declares has the id `{package, [scalar_type, form, FCD
  identity], 0}`): the qualified
  name as `declaration`, with a `declaration` occurrence at the declared
  name, an empty `aggregate` body, a unit typed by its dimension and a
  dimension self-typed. A compound unit's root units are built the same
  way. No node carries a nominal identity preimage, and none is recomputed
  or compared.
- **Compound unit.** The type node is `scalar_type` / `compound_unit`, its own
  semantic type, with no `declaration` and no `owner`. Its body is an
  `aggregate` of one term per compound-unit term, in ascending root-unit node key
  (the derived `Ord` of the structural node id, QSpec FR-322). A
  term is `aggregate{[binding "unit" = reference(the root unit's node),
  binding "exponent" = the exponent as an integer literal typed at the
  `Integer` node]}`. The dimensionless unit's body is `aggregate{[]}`.
  The node's `path` is `[scalar_type, compound_unit, U_1, e_1, ..., U_n, e_n]`:
  each term's root-unit node id followed by its exponent as canonical decimal
  text, in ascending root-unit node key (FR-322); the dimensionless unit's
  path is `[scalar_type, compound_unit]`. `check` assigns the `ordinal`.

`check` reads a compound unit's terms from the `CompoundUnit` that the check
stage's unit scope holds for its `UnitId`: the package's unit table, or the
product, quotient and power units the checker formed while typing
(`UnitScope`). The scope keeps each formed unit until lowering has keyed its
node. A declared unit `u` and the
single-term compound unit `u^1` are different `UnitId`s (ADR-013 OQ-B), so
they have different type nodes.

A function body that divides by a quantity literal of nonzero magnitude
([FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md#quantity-literals))
is admitted: the literal is a nonzero fact for the division. A body that
divides by any other quantity refuses with
`undefined_expression`/`unproved-nonzero`: a checked quantity type carries no
value interval, so no guard proves a divisor that is not a literal nonzero
(QSpec FR-146; the `Quantity` divide arm of `check::facts`). A checked graph
therefore holds the compound unit that each product forms, such as `metre^2`, and the compound unit that each admitted quotient forms, whose terms are
the dividend's with the divisor's exponents subtracted. FR-094-AC-6 identifies the
quotient units from the units `value::quantity::result_unit` forms,
which is the forming `check` does while it types a quantity division, and
FR-094-AC-11 identifies the quotient of a division by a literal.

- **Dimension equivalence.** The unit table holds each dimension equivalence the
  unit declares (FR-091). `check` builds one
  `scalar_type`/`dimension_equivalence` node for each, in source order, as
  QSpec FR-322 spells it (FR-322-AC-118): self-typed, with no
  `declaration`, relating the two dimension nodes in ascending node-key order
  (the derived `Ord`), with the root ratio the unit table holds (FR-091
  step 6) as `root_scale`, stated for the node listed first, depending on
  exactly those two dimension nodes, and carrying one `anchor` occurrence whose
  ordinal is the declaration's place among the unit's equivalence declarations
  and whose region is the declaration's. It builds the node whether or not a
  quantity type names either dimension, together with the two dimension nodes
  it depends on as the declared-unit bullet builds a dimension node, and it
  changes no dimension node's key. The two spellings `A ~ B by x U = y V` and
  `B ~ A by y V = x U` build one node, and so do two statements of one ratio
  through different units.

Quantity parameters, the existing quantity operations and quantity literals
use these unit type nodes. A quantity literal in a value expression
([FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md#quantity-literals))
is a `value`/`literal` node of `value_kind` `quantity` whose `semantic_type`
and literal `type` are its declared unit's node, so it introduces no second
quantity type and no new type node: its unit's node is the one a
`Quantity<u>` parameter of that unit names (QSpec FR-322-AC-79).

### Refusals

`check` SHALL treat each of these as an internal fault (ADR-013 T-4), and
yield no key for the node that needs it:

- a `Reference<T>` whose `EffectiveId` is no `type_identities` value of an
  admitted effective view;
- a `DeclarationKey` whose `package` is no admitted domain package's
  identity, or whose `node` is empty;
- a compound `UnitId` that the check stage's unit scope does not hold;
- a declared unit, or a dimension or unit a held unit names, or a dimension
  equivalence, whose node the unit table does not hold;
- a domain package record kind that no checked node names.

Type admission refuses an unknown `Reference` target for a source-compiled
unit before `check` keys a node (FR-091's unresolved model reference, and
`TypeEnvironment`'s `type_refusal` with cause `TypeMismatch`), intake refuses
an unselected package or an empty IR identity, and the unit scope holds every
compound unit a checked quantity type names, so reaching one of these is a
fault in `check`, not in the input.

Building a `Reference`, `Population` or compound-unit type node walks the type
tree as FR-092 does, over an explicit heap stack within the check stage's
limits (`CheckingLimits`).

### Fixture

The acceptance criteria below use one domain package, `acme/orders`, at version
`1.0.0`; at version `2.0.0` every node id is the same (AC-1, AC-5). It declares
object type `ix://acme/orders/Order` with attribute `total: Int[0, 9]` and query
operations `size(): Integer`, `count(): Integer` and
`scaled(n: Integer): Integer`, object type `ix://acme/orders/Invoice`, object
type `ix://acme/orders/Sub` with supertype `Order`, which redefines `size`, and
redefines `scaled` as `scaled(m: Integer)`; and relationship
`ix://acme/orders/billedTo`. `M::Order`, `M::Invoice` and `M::Sub` name those
object types. A model declaration node's id is `{package: acme/orders, path:
[model, object_type, <node>], ordinal: 0}`, and `[relation, relationship,
<node>]` for the relationship. The root units of the compound units below are
QSpec's `unit-metre` and `unit-second` nominal nodes.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-094-CON-1 | The `match` that builds a model declaration node from a domain package record kind, with one arm per `DomainPackageRecord` variant, the `match` that builds a quantity type node from a `UnitId` domain, and the `match` that spells a `DeclaredClauseKind` have no `_` or catch-all arm (ADR-012 §5.1). | Design | Test (TC-417, TC-418, TC-419) |
| FR-094-CON-2 | A model-owned node carries `ModelOwner` as its `owner`, never `SourceOwner` or `DefinitionOwner`, and no node id member is computed from an `EffectiveId` (ADR-013 O-05). | Design | Test (TC-417, TC-418) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-094-AC-1 | With `acme/orders` `1.0.0` admitted, a parameter typed `Reference<M::Order>` has a `composite_type`/`reference` type node whose id is `{package of the checked package, path [composite_type, reference, <Order's node id>], ordinal 0}` and whose body references `Order`'s model node, whose id is `{acme/orders, [model, object_type, ix://acme/orders/Order], 0}`. With `acme/orders` `2.0.0` admitted instead, the same source gives the same two ids. `Invoice` has the id with `ix://acme/orders/Invoice` and the relationship `billedTo` the id `{acme/orders, [relation, relationship, ix://acme/orders/billedTo], 0}`. `Order`'s model node has `owner` `{kind: "model", identity: "acme/orders", node: "ix://acme/orders/Order"}`, with no `version` member, and `declaration` `null`, and the reference type node has no `owner`. | Test (TC-417) |
| FR-094-AC-2 | Checking `function g using v(r: Reference<M::Order>, s: Reference<M::Invoice>): Boolean pure { true }` records exactly two model correspondence entries, (`Order`'s node id, `ix://acme/orders/Order`'s `DeclarationKey`) and (`Invoice`'s node id, `ix://acme/orders/Invoice`'s), and no entry names a clause function or a type node. | Test (TC-417) |
| FR-094-AC-3 | In functions over the parameters `p: Population<M::Order>[3]` and `r: Reference<M::Order>`, in that order, `p`'s type node has id path `[bounded_domain, model_population, <Order's node id>, "3"]` and `semantic_type` the `Set<Reference<Order>>` node, and `r`'s type is the reference node of AC-1. `allInstances<M::Order>(p)` has `result_type` the `Set<Reference<Order>>[0, 3]` node, `lookup<M::Order>(p, r) absent undefined` has `result_type` the reference node, and `lookup<M::Order>(p, r) absent empty` has `result_type` the `Option<Reference<Order>>` node. `p: Population<M::Order>[5]` has a type node with a different id from the `[3]` one, and `Population<M::Invoice>[3]` has a different id from `Population<M::Order>[3]`. | Test (TC-417) |
| FR-094-AC-4 | With `r: Reference<M::Order>` at level 1, `deref(r).total` has a `field` member naming `Order`'s model node and `total`, and `r.size()` has an `operation` member naming `Order`'s model node and `size`. `deref(s).total` and `s.size()` over `s: Reference<M::Sub>` name `Sub`'s model node, not `Order`'s. | Test (TC-417) |
| FR-094-AC-5 | `checked_dispatch_operation` for `Order.size` with the authored precondition `true` and the body `7`, and receiver parameter `self: Reference<M::Order>`, builds two clause functions, one `precondition` and one `body`, each with `owner` `ModelOwner` of `ix://acme/orders/Order/size` and `declaration` `null`. Under `acme/orders` `2.0.0`, the same receiver parameter and the same clause functions are built, with the same ids. The same precondition authored on `Order.count` is a different node. When `Sub.size` redefines `Order.size` with the authored precondition `false` and receiver `self: Reference<M::Sub>`, its authored precondition and its effective precondition `false or true` are two nodes whose owner is `Sub.size`, while `Order.size`'s authored function is unchanged. Rebuilding with a different synthesized label gives the same ids, and building `Sub.size`'s clauses from both dispatch operations yields one node for each clause. | Test (TC-418) |
| FR-094-AC-6 | With `a` a quantity of QSpec's declared unit `metre` and `t` one of `second`, `a`'s parameter has semantic type the QSpec `unit-metre` node, and the graph holds no `compound_unit` node for it. In a function body, the result type of `a * a` is a `compound_unit` node of `metre^2`. The compound units that `value::quantity::result_unit` forms for `a / t`, `a / a` and `(a * a) / a`, held in the check stage's unit scope, are the `compound_unit` nodes of `metre^1 second^-1`, the dimensionless unit and `metre^1`. The first term of the first names metre, and the `metre^1` node differs from the `metre` unit node. | Test (TC-419) |
| FR-094-AC-7 | Keying a `Reference` whose `EffectiveId` is no `type_identities` value, a `DeclarationKey` whose `package` is no admitted identity, a `DeclarationKey` whose `node` is empty, a compound `UnitId` the unit scope does not hold, a declared unit for which the unit table holds no admitted node (a table built from the graph's units without `UnitTable::declared`), and a field member record reaching the record-kind `match` each refuses as an internal fault naming that value, and yields no key. | Test (TC-417, TC-419) |
| FR-094-AC-8 | Under a unit graph its own source declares, with base dimensions `Length` and `Time`, the derived dimension `Velocity = Length / Time`, root units `metre`, `second` and `mps` (of `Velocity`) and `km = 1000 × metre`, a record `Trip{d: km, v: mps}` is emitted with no node omitted. The `km` node depends on `Length` and `metre` and it targets `metre` with scale `1000`; the `Velocity` node depends on `Length` and `Time` and holds both terms; `mps` is typed by `Velocity`; and IR's checked-package/v2 reader admits the package. | Test (TC-419) |
| FR-094-AC-9 | For the unreferenced `billedTo` relationship selected through model alias `M`, `check` emits its `relation`/`relationship` node and records one `generated` occurrence at `M`'s declaration-site/source span, regardless of function or frame reachability; fallback `generated_location` and `UnlocatedOccurrence` are forbidden. | Test (TC-417) |
| FR-094-AC-11 | With FR-093-AC-36's units, the quotient `a / 3 ms` over `a: Quantity<m>` has one `scalar_type`/`compound_unit` node, with no `declaration` and no `owner`, whose body is an `aggregate` of two terms in ascending root-unit node-key order: `unit` the `m` node with `exponent` the integer literal `1`, and `unit` the `s` node with `exponent` the integer literal `-1`. The quotient `d / 3 ms` over `d: Quantity<ms>` has a `compound_unit` node whose body is `aggregate{[]}`, and `d / 3 s`, `d / rational(-3, 1) ms` and `d / -(3 ms)` have that same node, and `a / -(3 ms)` has the node of `a / 3 ms`. Neither node is the `m` unit node, and the node of `a / 3 ms` is the node of `a / 0.5 s`. | Test (TC-419) |
| FR-094-AC-12 | **Withdrawn.** The fixed QSpec vector key and `nominal_identity_preimage` oracle for the dimension-equivalence node was superseded by QSpec FR-322 structural node identity; the node's non-identity behavior is retained in FR-094-AC-13. | Withdrawn |
| FR-094-AC-13 | With a unit declaring `model R = "acme/rpc" …` and `model F = "acme/flow" …`, each package declaring base dimension `Time` and a root unit of it (`s`; flow's `ms` is `1/1000` of its root), and `dimension R::Time ~ F::Time by 1 R::s = 1000 F::ms;`, the checked graph holds one `scalar_type`/`dimension_equivalence` node with no `declaration`, self-typed, relating the two package `Time` nodes in ascending node-key order with `root_scale` `1/1`, whose `dependencies` are exactly those two nodes and whose one occurrence has role `anchor` and ordinal 0; `dimension F::Time ~ R::Time by 1000 F::ms = 1 R::s;` in its place gives the same node; with `acme/clock` as a third package (root `ms`), `dimension R::Time ~ G::Time by 1 R::s = 1000 G::ms;` gives the node with `root_scale` `1000/1` and `dimension G::Time ~ R::Time by 1000 G::ms = 1 R::s;` the same node; a second declaration `dimension R::Time ~ Time by 1 R::s = 1 s;` over a source `Time` gives a second node with ordinal 1; the package `Time` nodes are the nodes they are with no equivalence; the equivalence node is emitted though no quantity type names either dimension; and IR's checked-package/v2 reader admits the package and derives the classes the declarations give, refusing `invalid_semantic_graph` the same package with a `root_scale` of `0/1`, `-1/1`, `2000/2` or none, or with the ratios of its base pairs not multiplying to one. | Test (TC-419) |

A check with two admitted versions of one domain package
identity refuses as an internal fault.

## Dependencies

- [FR-092](FR-092-key-type-parameter-and-declared-nodes.md): the
  structural node id, the type and function node shapes and its type walk.
- [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md): the
  `Attribute`, `AllInstances`, `Lookup` and `Dispatch` application rows whose
  members and types this requirement keys.
- [FR-084](FR-084-admit-closed-populations-and-resolve-lookup.md): the
  `allInstances` and `lookup` result types.
- [FR-088](FR-088-clause-name-and-type-identity.md): the model correspondence
  and the two-domain `UnitId`.
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  O-01, O-03, O-04, O-05, O-06, O-08, C-02, OQ-B, OQ-G, T-4, QC-24 (the
  `Attribute` projection over `deref`), and QC-25 and QC-26, the QSpec
  requests this requirement makes.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §3 FB-13: only `check` mints a `NodeKey`.
- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md)
  §5.1: no catch-all arm.
- QSpec: `ModelOwner` (FR-322-AC-28; ADR-013 QC-3), the
  `reference` composite form and the `model_population` domain form of
  `schema.json`, and the model operation entries of `operation-catalog.json`.
  The `compound_unit` form, the
  `model_population` body and family, the `clause` binding and the v2
  correspondence spelling are QSL proposals (QC-25, QC-26).
