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

Every node here whose body holds no application is keyed by FR-092's
`quire.structural-node/v1` preimage. A node that stands for a domain
package's declaration carries QSpec's `ModelOwner`, as ADR-013 O-04 and C-02
decide. The node shapes, the `ModelOwner` use on the structural preimage, the
`model_population` and `compound_unit` shapes and the `clause` binding are
QSL proposals to QSpec (ADR-013 QC-25, QC-26), as `quire.structural-node/v1`
is (QC-24). QSpec references (`ix://agent-ix/quire-specification`, cited by
reference, never copied): FR-322, `proposals/checked-package-v2/schema.json`,
`node-identity-preimage.schema.json`, `node-identity-vectors.json`,
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

- Each node's RFC 8785 preimage bytes and its `quire.checked-semantic-node/v1`
  key, and the model correspondence entries (node key, `DeclarationKey`).
- Or a typed refusal and no key.

## Behavior

### Model-owned nodes

A **model-owned node** is a model declaration node or a clause function
node, both defined below. `check` SHALL key a model-owned node by the
`quire.structural-node/v1` preimage (FR-092 rule 3) with these members fixed:

- `owner`: QSpec's content-only `ModelOwner`, `{kind: "model", identity,
  node}`, with no `version` member (QSpec FR-322-AC-28). `identity` is that
  of the `DomainPackageRef` of the domain package that declares it, the
  selection whose identity equals the
  `DeclarationKey`'s `package`. `node` is the `DeclarationKey`'s `node`, the
  IR node identity (for example `ix://acme/orders/Order`).
- `declaration`: `null`. A model-owned node's name belongs to the domain
  package, not to a QSL source declaration, and it has no `declaration`
  source occurrence (FR-322 `declaration`).

A domain package selects one version of an identity per check (ADR-013 O-01,
QSpec FR-321), so within one check the owner's (`identity`, `node`) names
exactly one declaration. A domain package's version is not owner content, so a change of the domain package
that leaves a node's content unchanged keys the same model-owned nodes, and a node id depends only
on content (QSpec FR-322-AC-28). The v2 emission writes a model-owned node's
`ModelOwner` as its wire `owner` member (FR-093), so a reader keys the node
from the node itself.

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
content: the selection names the domain package's bytes by identity,
and intake admits one selection of an identity per check (QSpec FR-321), so
within one check `identity` names one set of bytes. Across checks, the node
id is the same for every version selected under one `identity`,
as it is for every QSpec `ModelOwner` nominal node. An empty body also keeps model declaration
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
No key is computed from an `EffectiveId`'s bytes, so a model declaration's
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
FR-092 spells an integer. Both nodes are anonymous: they carry no
`declaration` and no `owner` (FR-092, ADR-013 OQ-G). The owner scope comes
through `T`'s model node, so `Reference<T>` over object types of two domain
package identities gives two ids, and a version-only change of `T`'s domain
package keeps the id.

A type built from these follows FR-092's table: the `lookup<T>` result under
`absent empty` is the `Option<Reference<T>>` node, and the `allInstances<T>(p)`
result for a population of maximum `N` is the `collection_bounds` node of
`Set<Reference<T>>[0, N]` (FR-084).

A population the domain package declares and a state clause names as `M::P`
(FR-391) is not a `Population<T>[N]` type node. Its reference is a separate
node keyed by the population's declaration key alone, with no `N`, whose
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
`ix://acme/orders/Order/size.precondition`) is a lookup label and appears in
no preimage member except as the owner's `node` where the two coincide: a
body function's label is its candidate's IR node identity. Renaming the
label leaves the key unchanged.

Two clause functions with equal preimages are one node (FR-093's content
addressing). A redefining candidate's own clauses are built both by its own
dispatch operation and as a candidate of the operation it redefines, and
they key to one node each time.

A clause function carries `ModelOwner`, not `SourceOwner` or
`DefinitionOwner`, because the domain package declares its clause, not a QSL
source unit and not a definition document. The owner scopes the key to one
member of one domain package identity, and the `clause` binding tells one
member's precondition and body apart. A `dispatch_call` reaches its
candidates' clause functions through the checked package's dispatch tables,
not through a graph reference; the v2 spelling of that link is QC-25.

### Quantity type nodes

`check` SHALL build the type node of a `Quantity(UnitId)` by the `UnitId`'s
domain:

- **Declared unit.** The type node is the unit's QSpec nominal node
  (`scalar_type` / `unit`, `quire.unit-node/v1`, FR-092 rule 1). It is the
  quantity's `semantic_type`, `result_type` and literal `type` directly. No
  other node stands for the quantity type. `check` builds it, and every
  dimension and unit node it names (its dimension, that dimension's base
  dimensions, its target units), from the admitted node the unit table
  holds for each, keyed by its admitted preimage bytes, as QSpec's nominal
  fixtures spell them: the preimage's qualified name as `declaration`, with
  a `declaration` occurrence at the declared name, an empty `aggregate`
  body, a unit typed by its dimension and a dimension self-typed. A
  compound unit's root units are built the same way. The node carries its
  nominal preimage when that preimage, rebuilt under the checked unit's own
  `SourceOwner`, has the admitted bytes; a node another owner declares
  carries none, and the emission omits it as a nominal node whose owner
  the package does not select.
- **Compound unit.** The type node is `scalar_type` / `compound_unit`, its own
  semantic type, with no `declaration` and no `owner`. Its body is an
  `aggregate` of one term per compound-unit term, in the order of the unit's
  `quire.value.compound-unit/v1` preimage (ascending root-unit node key). A
  term is `aggregate{[binding "unit" = reference(the root unit's node),
  binding "exponent" = the exponent as an integer literal typed at the
  `Integer` node]}`. The dimensionless unit's body is `aggregate{[]}`.

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
therefore holds the compound unit that each product forms, such as `metre^2`
(U1), and the compound unit that each admitted quotient forms, whose terms are
the dividend's with the divisor's exponents subtracted. FR-094-AC-6 keys the
quotient units U2 to U4 from the units `value::quantity::result_unit` forms,
which is the forming `check` does while it types a quantity division, and
FR-094-AC-11 keys the quotient of a division by a literal.

- **Dimension equivalence.** The unit table holds the admitted preimage of each
  dimension equivalence the unit declares (FR-091). `check` builds one
  `scalar_type`/`dimension_equivalence` node for each, in source order, from
  that preimage as QSpec FR-322 spells it (FR-322-AC-118): self-typed, with no
  `declaration`, its `nominal_identity_preimage`
  `{version: quire.dimension-equivalence-node/v1, owner, dimension_node_ids,
  root_scale}` with the two related dimension keys in ascending key order
  and the root ratio the unit table holds (FR-091 step 6) as `root_scale`, stated
  for the node listed first, depending on
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
  equivalence, whose nominal preimage the unit table does not hold;
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

### Vector index

Each vector below names a node whose structural key `check` SHALL mint. A
structural key is the node's preimage value itself (`owner`, `node_tag`,
`semantic_form`, `body` and the other members stated in Behavior above, with
each child node referenced by its own key), compared by equality. The domain
package is `acme/orders`, at version `1.0.0`; at version `2.0.0` every vector is
the same (AC-1, AC-5). It declares object type
`ix://acme/orders/Order` with attribute `total: Int[0, 9]` and query
operations `size(): Integer`, `count(): Integer` and
`scaled(n: Integer): Integer`, object type `ix://acme/orders/Invoice`, object
type `ix://acme/orders/Sub` with supertype `Order`, which redefines `size`, and
redefines `scaled` as `scaled(m: Integer)`; and relationship
`ix://acme/orders/billedTo`. `M::Order`, `M::Invoice` and
`M::Sub` name those object types. A
vector that names another vector's key uses the key that vector gives, and
T1 to T4, L1 and L2 are FR-092's. The root units of U1 to U4 are QSpec's
`unit-metre` and `unit-second` nominal keys.

| Vector | Node | Key |
|---|---|---|
| M1 | object type `Order`, owner (`acme/orders`, `ix://acme/orders/Order`) | its structural key |
| M3 | object type `Invoice`, owner (`acme/orders`, `ix://acme/orders/Invoice`) | its structural key |
| M4 | relationship `billedTo`, owner (`acme/orders`, `ix://acme/orders/billedTo`) | its structural key |
| M5 | object type `Sub` (supertype `Order`), owner (`acme/orders`, `ix://acme/orders/Sub`) | its structural key |
| R1 | `Reference<Order>` (M1) | its structural key |
| R3 | `Option<Reference<Order>>` | its structural key |
| S1 | `Set<Reference<Order>>` | its structural key |
| S2 | `Set<Reference<Order>>[0, 3]` | its structural key |
| PO1 | `Population<Order>[3]` | its structural key |
| PO2 | `Population<Order>[5]` | its structural key |
| R4 | `Reference<Sub>` (M5) | its structural key |
| R5 | `Reference<Invoice>` (M3) | its structural key |
| S3 | `Set<Reference<Invoice>>` | its structural key |
| PO3 | `Population<Invoice>[3]` | its structural key |
| P5 | parameter `p: Population<M::Order>[3]`, level 0 | its structural key |
| P6 | parameter `r: Reference<M::Order>`, level 1 | its structural key |
| P7 | parameter `self: Reference<M::Order>`, level 0 | its structural key |
| E4 | `allInstances<M::Order>(p)` | its structural key |
| E5 | `lookup<M::Order>(p, r) absent undefined` | its structural key |
| E6 | `lookup<M::Order>(p, r) absent empty` | its structural key |
| E7 | `deref(r)`, the operand of `deref(r).total` | its structural key |
| E8 | `deref(r).total` | its structural key |
| E9 | `r.size()` | its structural key |
| C1 | precondition clause `true` of `Order.size`, owner (`acme/orders`, `ix://acme/orders/Order/size`) | its structural key |
| C2 | body clause `7` of `Order.size`, same owner | its structural key |
| C4 | precondition clause `true` of `Order.count` | its structural key |
| P8 | parameter `self: Reference<M::Sub>`, level 0 | its structural key |
| L4 | literal `false` | its structural key |
| E10 | `false or true`, the effective precondition of `Sub.size` | its structural key |
| C5 | precondition clause `false` authored on `Sub.size`, owner (`acme/orders`, `ix://acme/orders/Sub/size`) | its structural key |
| C6 | effective precondition `false or true` of candidate `Sub.size`, same owner as C5 | its structural key |
| U1 | compound unit `metre^2` | its structural key |
| U2 | compound unit `metre^1 second^-1` | its structural key |
| U3 | the dimensionless compound unit | its structural key |
| U4 | compound unit `metre^1` | its structural key |

The `digest` and `domain` members of a node-id object in the vectors below are QSpec FR-322 wire members that QSL carries as opaque strings (BLOCKED-BY STD-583 (owner: planner titan:QSL:plan)).

**M1**: object type `Order`, owner (`acme/orders`, `ix://acme/orders/Order`)

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"model","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/Order"},"recursion":null,"semantic_form":"object_type","semantic_type":null,"version":"quire.structural-node/v1"}
```

**M3**: object type `Invoice`, owner (`acme/orders`, `ix://acme/orders/Invoice`)

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"model","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/Invoice"},"recursion":null,"semantic_form":"object_type","semantic_type":null,"version":"quire.structural-node/v1"}
```

**M4**: relationship `billedTo`, owner (`acme/orders`, `ix://acme/orders/billedTo`)

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"relation","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/billedTo"},"recursion":null,"semantic_form":"relationship","semantic_type":null,"version":"quire.structural-node/v1"}
```

**M5**: object type `Sub` (supertype `Order`), owner (`acme/orders`, `ix://acme/orders/Sub`)

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"model","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/Sub"},"recursion":null,"semantic_form":"object_type","semantic_type":null,"version":"quire.structural-node/v1"}
```

**R1**: `Reference<Order>` (M1)

```json
{"body":{"members":[{"target":{"digest":"<digest of M1>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":null,"semantic_form":"reference","semantic_type":null,"version":"quire.structural-node/v1"}
```

**R3**: `Option<Reference<Order>>`

```json
{"body":{"members":[{"target":{"digest":"<digest of R1>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":null,"semantic_form":"option","semantic_type":null,"version":"quire.structural-node/v1"}
```

**S1**: `Set<Reference<Order>>`

```json
{"body":{"members":[{"target":{"digest":"<digest of R1>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":null,"semantic_form":"set","semantic_type":null,"version":"quire.structural-node/v1"}
```

**S2**: `Set<Reference<Order>>[0, 3]`

```json
{"body":{"members":[{"name":"min","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T2>","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}},{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T2>","domain":"quire.checked-semantic-node/v1"},"value":"3","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"collection_bounds","semantic_type":{"digest":"<digest of S1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

**PO1**: `Population<Order>[3]`

```json
{"body":{"members":[{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T2>","domain":"quire.checked-semantic-node/v1"},"value":"3","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"model_population","semantic_type":{"digest":"<digest of S1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

**PO2**: `Population<Order>[5]`

```json
{"body":{"members":[{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T2>","domain":"quire.checked-semantic-node/v1"},"value":"5","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"model_population","semantic_type":{"digest":"<digest of S1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

**R4**: `Reference<Sub>` (M5)

```json
{"body":{"members":[{"target":{"digest":"<digest of M5>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":null,"semantic_form":"reference","semantic_type":null,"version":"quire.structural-node/v1"}
```

**R5**: `Reference<Invoice>` (M3)

```json
{"body":{"members":[{"target":{"digest":"<digest of M3>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":null,"semantic_form":"reference","semantic_type":null,"version":"quire.structural-node/v1"}
```

**S3**: `Set<Reference<Invoice>>`

```json
{"body":{"members":[{"target":{"digest":"<digest of R5>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":null,"semantic_form":"set","semantic_type":null,"version":"quire.structural-node/v1"}
```

**PO3**: `Population<Invoice>[3]`

```json
{"body":{"members":[{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T2>","domain":"quire.checked-semantic-node/v1"},"value":"3","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"model_population","semantic_type":{"digest":"<digest of S3>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

**P5**: parameter `p: Population<M::Order>[3]`, level 0

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T3>","domain":"quire.checked-semantic-node/v1"},"value":"p","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T2>","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"<digest of PO1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

**P6**: parameter `r: Reference<M::Order>`, level 1

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T3>","domain":"quire.checked-semantic-node/v1"},"value":"r","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T2>","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"<digest of R1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

**P7**: parameter `self: Reference<M::Order>`, level 0

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T3>","domain":"quire.checked-semantic-node/v1"},"value":"self","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T2>","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"<digest of R1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

**E4**: `allInstances<M::Order>(p)`

```json
{"body":{"arguments":[{"target":{"digest":"<digest of P5>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.model.all_instances","laws":[],"leaves":[],"member":{"declaration":{"digest":"<digest of M1>","domain":"quire.checked-semantic-node/v1"},"kind":"type_argument"},"mode":null},"operator":"query","result_type":{"digest":"<digest of S2>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"query","semantic_type":{"digest":"<digest of S2>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

**E5**: `lookup<M::Order>(p, r) absent undefined`

```json
{"body":{"arguments":[{"target":{"digest":"<digest of P5>","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"<digest of P6>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.model.lookup","laws":[],"leaves":[],"member":{"declaration":{"digest":"<digest of M1>","domain":"quire.checked-semantic-node/v1"},"kind":"type_argument"},"mode":{"kind":"absence","value":"undefined"}},"operator":"query","result_type":{"digest":"<digest of R1>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"query","semantic_type":{"digest":"<digest of R1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

**E6**: `lookup<M::Order>(p, r) absent empty`

```json
{"body":{"arguments":[{"target":{"digest":"<digest of P5>","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"<digest of P6>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.model.lookup","laws":[],"leaves":[],"member":{"declaration":{"digest":"<digest of M1>","domain":"quire.checked-semantic-node/v1"},"kind":"type_argument"},"mode":{"kind":"absence","value":"empty"}},"operator":"query","result_type":{"digest":"<digest of R3>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"query","semantic_type":{"digest":"<digest of R3>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

**E7**: `deref(r)`, the operand of `deref(r).total`

```json
{"body":{"arguments":[{"target":{"digest":"<digest of P6>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.model.deref","laws":[],"leaves":[],"member":null,"mode":null},"operator":"deref","result_type":{"digest":"<digest of M1>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"deref","semantic_type":{"digest":"<digest of M1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

**E8**: `deref(r).total`

```json
{"body":{"arguments":[{"target":{"digest":"<digest of E7>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.record.project","laws":[],"leaves":[],"member":{"declaration":{"digest":"<digest of M1>","domain":"quire.checked-semantic-node/v1"},"kind":"field","name":"total"},"mode":null},"operator":"query","result_type":{"digest":"<digest of FR-092 T4>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"query","semantic_type":{"digest":"<digest of FR-092 T4>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

**E9**: `r.size()`

```json
{"body":{"arguments":[{"target":{"digest":"<digest of P6>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.model.dispatch_call","laws":[],"leaves":[],"member":{"declaration":{"digest":"<digest of M1>","domain":"quire.checked-semantic-node/v1"},"kind":"operation","name":"size"},"mode":null},"operator":"call","result_type":{"digest":"<digest of FR-092 T2>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"call","semantic_type":{"digest":"<digest of FR-092 T2>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

**C1**: precondition clause `true` of `Order.size`, owner (`acme/orders`, `ix://acme/orders/Order/size`)

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"<digest of P7>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"target":{"digest":"<digest of FR-092 L1>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"clause","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T3>","domain":"quire.checked-semantic-node/v1"},"value":"precondition","value_kind":"text"}}],"term":"aggregate"},"declaration":null,"node_tag":"function","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/Order/size"},"recursion":null,"semantic_form":"pure_function","semantic_type":{"digest":"<digest of FR-092 T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

**C2**: body clause `7` of `Order.size`, same owner

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"<digest of P7>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"target":{"digest":"<digest of FR-092 L2>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"clause","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T3>","domain":"quire.checked-semantic-node/v1"},"value":"body","value_kind":"text"}}],"term":"aggregate"},"declaration":null,"node_tag":"function","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/Order/size"},"recursion":null,"semantic_form":"pure_function","semantic_type":{"digest":"<digest of FR-092 T2>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

**C4**: precondition clause `true` of `Order.count`

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"<digest of P7>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"target":{"digest":"<digest of FR-092 L1>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"clause","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T3>","domain":"quire.checked-semantic-node/v1"},"value":"precondition","value_kind":"text"}}],"term":"aggregate"},"declaration":null,"node_tag":"function","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/Order/count"},"recursion":null,"semantic_form":"pure_function","semantic_type":{"digest":"<digest of FR-092 T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

**P8**: parameter `self: Reference<M::Sub>`, level 0

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T3>","domain":"quire.checked-semantic-node/v1"},"value":"self","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T2>","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"<digest of R4>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

**L4**: literal `false`

```json
{"body":{"term":"literal","type":{"digest":"<digest of FR-092 T1>","domain":"quire.checked-semantic-node/v1"},"value":false,"value_kind":"boolean"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"literal","semantic_type":{"digest":"<digest of FR-092 T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

**E10**: `false or true`, the effective precondition of `Sub.size`

```json
{"body":{"arguments":[{"target":{"digest":"<digest of L4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"<digest of FR-092 L1>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.boolean.or","laws":[],"leaves":[],"member":null,"mode":null},"operator":"binary","result_type":{"digest":"<digest of FR-092 T1>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"binary","semantic_type":{"digest":"<digest of FR-092 T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

**C5**: precondition clause `false` authored on `Sub.size`, owner (`acme/orders`, `ix://acme/orders/Sub/size`)

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"<digest of P8>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"target":{"digest":"<digest of L4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"clause","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T3>","domain":"quire.checked-semantic-node/v1"},"value":"precondition","value_kind":"text"}}],"term":"aggregate"},"declaration":null,"node_tag":"function","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/Sub/size"},"recursion":null,"semantic_form":"pure_function","semantic_type":{"digest":"<digest of FR-092 T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

**C6**: effective precondition `false or true` of candidate `Sub.size`, same owner as C5

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"<digest of P8>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"target":{"digest":"<digest of E10>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"clause","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T3>","domain":"quire.checked-semantic-node/v1"},"value":"precondition","value_kind":"text"}}],"term":"aggregate"},"declaration":null,"node_tag":"function","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/Sub/size"},"recursion":null,"semantic_form":"pure_function","semantic_type":{"digest":"<digest of FR-092 T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

**U1**: compound unit `metre^2`

```json
{"body":{"members":[{"members":[{"name":"unit","term":"binding","value":{"target":{"digest":"<QSpec unit-metre key>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"exponent","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T2>","domain":"quire.checked-semantic-node/v1"},"value":"2","value_kind":"integer"}}],"term":"aggregate"}],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"compound_unit","semantic_type":null,"version":"quire.structural-node/v1"}
```

**U2**: compound unit `metre^1 second^-1`

```json
{"body":{"members":[{"members":[{"name":"unit","term":"binding","value":{"target":{"digest":"<QSpec unit-metre key>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"exponent","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T2>","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"}}],"term":"aggregate"},{"members":[{"name":"unit","term":"binding","value":{"target":{"digest":"<QSpec unit-second key>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"exponent","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T2>","domain":"quire.checked-semantic-node/v1"},"value":"-1","value_kind":"integer"}}],"term":"aggregate"}],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"compound_unit","semantic_type":null,"version":"quire.structural-node/v1"}
```

**U3**: the dimensionless compound unit

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"compound_unit","semantic_type":null,"version":"quire.structural-node/v1"}
```

**U4**: compound unit `metre^1`

```json
{"body":{"members":[{"members":[{"name":"unit","term":"binding","value":{"target":{"digest":"<QSpec unit-metre key>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"exponent","term":"binding","value":{"term":"literal","type":{"digest":"<digest of FR-092 T2>","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"}}],"term":"aggregate"}],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"compound_unit","semantic_type":null,"version":"quire.structural-node/v1"}
```

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-094-CON-1 | The `match` that builds a model declaration node from a domain package record kind, with one arm per `DomainPackageRecord` variant, the `match` that builds a quantity type node from a `UnitId` domain, and the `match` that spells a `DeclaredClauseKind` have no `_` or catch-all arm (ADR-012 §5.1). | Design | Test (TC-417, TC-418, TC-419) |
| FR-094-CON-2 | A model-owned node's preimage carries `ModelOwner`, never `SourceOwner` or `DefinitionOwner`, and no preimage member is computed from an `EffectiveId` (ADR-013 O-05). | Design | Test (TC-417, TC-418) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-094-AC-1 | With `acme/orders` `1.0.0` admitted, a parameter typed `Reference<M::Order>` has type node R1, whose body references `Order`'s model node M1. With `acme/orders` `2.0.0` admitted instead, the same source gives the same M1 and R1. `Invoice` keys to M3 and the relationship `billedTo` to M4. M1's preimage has `owner` `{kind: "model", identity: "acme/orders", node: "ix://acme/orders/Order"}`, with no `version` member, and `declaration` `null`, and R1's preimage has no `owner`. | Test (TC-417) |
| FR-094-AC-2 | Checking `function g using v(r: Reference<M::Order>, s: Reference<M::Invoice>): Boolean pure { true }` records exactly two model correspondence entries, (M1, `ix://acme/orders/Order`'s `DeclarationKey`) and (M3, `ix://acme/orders/Invoice`'s), and no entry names a clause function or a type node. | Test (TC-417) |
| FR-094-AC-3 | In functions over the parameters `p: Population<M::Order>[3]` and `r: Reference<M::Order>`, in that order, `p`'s parameter node keys to P5 over type node PO1, whose `semantic_type` is S1, and `r`'s to P6. `allInstances<M::Order>(p)` keys to E4 with `result_type` S2, `lookup<M::Order>(p, r) absent undefined` to E5 with `result_type` R1, and `lookup<M::Order>(p, r) absent empty` to E6 with `result_type` R3. `p: Population<M::Order>[5]` has type node PO2, and `Population<M::Invoice>[3]` has PO3, which differs from PO1. | Test (TC-417) |
| FR-094-AC-4 | With `r: Reference<M::Order>` at level 1, `deref(r).total` lowers to E7 and E8, whose `field` member names M1 and `total`, and `r.size()` keys to E9, whose `operation` member names M1 and `size`. `deref(s).total` and `s.size()` over `s: Reference<M::Sub>` (R4) name `Sub`'s model node M5, not M1. | Test (TC-417) |
| FR-094-AC-5 | `checked_dispatch_operation` for `Order.size` with the authored precondition `true` and the body `7`, and receiver parameter `self: Reference<M::Order>` (P7), builds clause functions keyed C1 and C2. Under `acme/orders` `2.0.0`, the receiver parameter keys to the same P7 and the clause functions to the same C1 and C2. The same precondition authored on `Order.count` keys to C4. When `Sub.size` redefines `Order.size` with the authored precondition `false` and receiver `self: Reference<M::Sub>` (P8), its authored precondition keys to C5 and its effective precondition `false or true` (E10) to C6, whose owner is `Sub.size`, while `Order.size`'s authored function keeps C1. Each preimage carries `ModelOwner` and `declaration` `null`. Rebuilding with a different synthesized label gives the same keys, and building `Sub.size`'s clauses from both dispatch operations gives one node each. | Test (TC-418) |
| FR-094-AC-6 | With `a` a quantity of QSpec's declared unit `metre` and `t` one of `second`, `a`'s parameter has semantic type `<QSpec unit-metre key>`, QSpec's `unit-metre` key, and the graph holds no `compound_unit` node for it. In a function body, the result type of `a * a` keys to U1. The compound units that `value::quantity::result_unit` forms for `a / t`, `a / a` and `(a * a) / a`, held in the check stage's unit scope, key to U2, U3 and U4. U2's first term names metre, and U4 differs from the `metre` unit key. | Test (TC-419) |
| FR-094-AC-7 | Keying a `Reference` whose `EffectiveId` is no `type_identities` value, a `DeclarationKey` whose `package` is no admitted identity, a `DeclarationKey` whose `node` is empty, a compound `UnitId` the unit scope does not hold, a declared unit for which the unit table holds no admitted node (a table built from the graph's units without `UnitTable::declared`), and a field member record reaching the record-kind `match` each refuses as an internal fault naming that value, and yields no key. | Test (TC-417, TC-419) |
| FR-094-AC-8 | Under a unit graph its own source declares, with base dimensions `Length` and `Time`, the derived dimension `Velocity = Length / Time`, root units `metre`, `second` and `mps` (of `Velocity`) and `km = 1000 × metre`, a record `Trip{d: km, v: mps}` is emitted with no node omitted. The `km` node depends on `Length` and `metre` and its preimage targets `metre` with scale `1000`; the `Velocity` node depends on `Length` and `Time` and its preimage holds both terms; `mps` is typed by `Velocity`; and IR's checked-package/v2 reader admits the package. | Test (TC-419) |
| FR-094-AC-9 | For the unreferenced `billedTo` relationship selected through model alias `M`, `check` emits its `relation`/`relationship` node and records one `generated` occurrence at `M`'s declaration-site/source span, regardless of function or frame reachability; fallback `generated_location` and `UnlocatedOccurrence` are forbidden. | Test (TC-417) |
| FR-094-AC-11 | With FR-093-AC-36's units, the quotient `a / 3 ms` over `a: Quantity<m>` has one `scalar_type`/`compound_unit` node, with no `declaration` and no `owner`, whose body is an `aggregate` of two terms in ascending root-unit node-key order: `unit` the `m` node with `exponent` the integer literal `1`, and `unit` the `s` node with `exponent` the integer literal `-1`. The quotient `d / 3 ms` over `d: Quantity<ms>` has a `compound_unit` node whose body is `aggregate{[]}`, and `d / 3 s`, `d / rational(-3, 1) ms` and `d / -(3 ms)` have that same node, and `a / -(3 ms)` has the node of `a / 3 ms`. Neither node is the `m` unit node, and the node of `a / 3 ms` is the node of `a / 0.5 s`. | Test (TC-419) |
| FR-094-AC-12 | With a unit declaring `model R = "acme/rpc" …` and `model F = "acme/flow" …`, each package declaring base dimension `Time` and a root unit of it (`s`; flow's `ms` is `1/1000` of its root), and `dimension R::Time ~ F::Time by 1 R::s = 1000 F::ms;`, the checked graph holds one `scalar_type`/`dimension_equivalence` node with no `declaration`, self-typed, whose preimage is `{version: quire.dimension-equivalence-node/v1, owner: SourceOwner, dimension_node_ids: [the two package Time keys in ascending key order], root_scale: 1/1}` (FR-091-AC-61's the QSpec vector key under QSpec's vector owner), whose `dependencies` are exactly those two keys and whose one occurrence has role `anchor` and ordinal 0; `dimension F::Time ~ R::Time by 1000 F::ms = 1 R::s;` in its place gives the same key; with `acme/clock` as a third package (root `ms`), `dimension R::Time ~ G::Time by 1 R::s = 1000 G::ms;` gives the node with `root_scale` `1000/1` and `dimension G::Time ~ R::Time by 1000 G::ms = 1 R::s;` the same key; a second declaration `dimension R::Time ~ Time by 1 R::s = 1 s;` over a source `Time` gives a second node with ordinal 1; the package `Time` nodes keep the keys they have with no equivalence; the equivalence node is emitted though no quantity type names either dimension; and IR's checked-package/v2 reader admits the package and derives the classes the declarations give, refusing `invalid_semantic_graph` the same package with a `root_scale` of `0/1`, `-1/1`, `2000/2` or none, or with the ratios of its base pairs not multiplying to one. | Test (TC-419) |

A check with two admitted versions of one domain package
identity refuses as an internal fault.

## Dependencies

- [FR-092](FR-092-key-type-parameter-and-declared-nodes.md): the
  `quire.structural-node/v1` preimage, the type and function node shapes, its
  type walk and vectors T1 to T4, L1 and L2.
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
- QSpec: `ModelOwner` and its vectors (FR-322-AC-28; ADR-013 QC-3), the
  `reference` composite form and the `model_population` domain form of
  `schema.json`, and the model operation entries of `operation-catalog.json`.
  The `ModelOwner` structural preimage, the `compound_unit` form, the
  `model_population` body and family, the `clause` binding and the v2
  correspondence spelling are QSL proposals (QC-25, QC-26).
