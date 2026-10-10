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
exactly one declaration. The selected package's `sha256-jcs` digest is lock evidence in
`model_selections`, not owner content, so a change of the domain package
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
| `RelationshipRecord` | `relation` / `relationship` | itself | `aggregate{[]}` |

A record kind that no checked node names has no node: a field member, an
operation member, a scalar type, a component, an endpoint, an allocation and
a population record. The `match` over record kinds gives each of them an arm
that is an internal fault (ADR-013 T-4). An `ObjectTypeRecord` whose
`interfaceFeatures` is present is `systems_interface` even when the list is
empty. A `RelationshipRecord` is `relation` / `relationship` whatever FR-152
role it also plays, because a checked node names it only through a
`relationship_end` member.

The body is empty because the owner already determines the declaration's
content: the lock selects the domain package's bytes by identity and digest,
and intake admits one selection of an identity per check (QSpec FR-321), so
within one check `identity` names one set of bytes. Across checks, the node
id is the same for every version and digest selected under one `identity`,
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

- the `field` member of an `Attribute` (`deref(r).f`) names the model node of
  `r`'s static object type `T`, the target of `r`'s `Reference<T>` type, and
  the attribute name `f`;
- the `operation` member of a `Dispatch` (`r.m(...)`) names the model node of
  the receiver's static object type `T` and the member name `m`. FR-151
  resolves `m` statically to the one effective operation `T` exposes, so the
  member names `T`'s node even when a supertype of `T` declares `m`
  (ADR-013 O-06);
- a `relationship_end` member names the relationship's node and the end's
  name;
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
  the lock does not select.
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

A function body that divides by a quantity refuses with
`undefined_expression`/`unproved-nonzero`: a checked quantity type carries no
value interval, so no guard proves a quantity divisor nonzero (QSpec FR-146;
the `Quantity` divide arm of `check::facts`). A checked graph therefore
holds a compound unit that a product forms, such as `metre^2` (U1), and
holds no quotient unit. FR-094-AC-6 keys the quotient units U2 to U4 from
the units `value::quantity::result_unit` forms, which is the forming `check`
does while it types a quantity division.

### Refusals

`check` SHALL treat each of these as an internal fault (ADR-013 T-4), and
yield no key for the node that needs it:

- a `Reference<T>` whose `EffectiveId` is no `type_identities` value of an
  admitted effective view;
- a `DeclarationKey` whose `package` is no admitted domain package's
  identity, or whose `node` is empty;
- a compound `UnitId` that the check stage's unit scope does not hold;
- a declared unit, or a dimension or unit a held unit names, whose nominal
  preimage the unit table does not hold;
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

### Golden vectors

Each vector below is the exact preimage bytes and the key `check` SHALL mint
for the named node. The domain package is `acme/orders`, at version `1.0.0`;
at version `2.0.0` every vector is the same (AC-1, AC-5). It declares object type
`ix://acme/orders/Order` with attribute `total: Int[0, 9]` and query
operations `size(): Integer`, `count(): Integer` and
`scaled(n: Integer): Integer`, object type `ix://acme/orders/Invoice`, object
type `ix://acme/orders/Sub` with supertype `Order`, which redefines `size`, and
redefines `scaled` as `scaled(m: Integer)`; and relationship
`ix://acme/orders/billedTo`. `M::Order`, `M::Invoice` and
`M::Sub` name those object types. A
vector that names another vector's key uses the digest that vector gives, and
T1 to T4, L1 and L2 are FR-092's. The root units of U1 to U4 are QSpec's
`unit-metre` (`79637623…`) and `unit-second` (`d0f0c5d2…`) nominal vectors
from `node-identity-vectors.json`.

| Vector | Node | Key |
|---|---|---|
| M1 | object type `Order`, owner (`acme/orders`, `ix://acme/orders/Order`) | `6f006fa31e8ceeb4c7e6ffb71a68a7883643c931f90d487d35239d49390c596b` |
| M3 | object type `Invoice`, owner (`acme/orders`, `ix://acme/orders/Invoice`) | `4b07c3fd269e699bdd4bc1065acc80d2de047cc01e20fcd8bcf27fe404f79583` |
| M4 | relationship `billedTo`, owner (`acme/orders`, `ix://acme/orders/billedTo`) | `aa598d3142945d973a62e44cd87de545bd4e0046352b1669777b32abd074ed54` |
| M5 | object type `Sub` (supertype `Order`), owner (`acme/orders`, `ix://acme/orders/Sub`) | `c4df362db09cab29d75d4fda10da8c1ba4fe91e1a285721a78198dbfb1b5429d` |
| R1 | `Reference<Order>` (M1) | `249d3baf1f4728325f4b5194d54320ad6d7e11bc88de7a339504ea6582fe1fbb` |
| R3 | `Option<Reference<Order>>` | `473130ae8fc4a4088490090342954508c9f83d7c425ce9d40eb30609936ebecd` |
| S1 | `Set<Reference<Order>>` | `b8a0b4c1fcbcb8672f21a16c2373abed9ba6469cc9fff3288408349f288dbed5` |
| S2 | `Set<Reference<Order>>[0, 3]` | `38373b6eb15c08c8e4c980a6f432b200e4e476b7d383b2149df45ef9a6a7f365` |
| PO1 | `Population<Order>[3]` | `859d6b01a132a14711a8a4568e24a7ab54e3255b78bbcc62b14e4293dbf71f9a` |
| PO2 | `Population<Order>[5]` | `1ad3769488d608a16a67c8820f549452822f72f4bf9428e01257720fcbaeb1e2` |
| R4 | `Reference<Sub>` (M5) | `76c98302f8b0a4e2795c710719cb08010112a885a02812150f6c78729c4c87d2` |
| R5 | `Reference<Invoice>` (M3) | `c1d7d9c83f3f3d47136388b30219177d7b0eb47f245f9ee8237db8b0a4ce9396` |
| S3 | `Set<Reference<Invoice>>` | `f3f5a65e5ec9dd964b4f7cb7cdb345ec1b1443cafbfb589f1ec59ad72cf904ce` |
| PO3 | `Population<Invoice>[3]` | `73c35350eabf6d7c2ddcb20e40aa8bcd89056ccf6f320a281b2b9e64a2846502` |
| P5 | parameter `p: Population<M::Order>[3]`, level 0 | `6d40c507a2bfe6f3c3b70ae2b49b2a46f0fb82f9a1df2b2ef7fdd472342dd117` |
| P6 | parameter `r: Reference<M::Order>`, level 1 | `3f80443940b49c4f958abcb45bde0874211b549b90e9d9ca158c2f42ef1280f1` |
| P7 | parameter `self: Reference<M::Order>`, level 0 | `18cf5ed16dd555a52d0695ca3194de4835b4475cee06c2a6329f57bd147bc7e5` |
| E4 | `allInstances<M::Order>(p)` | `66fcc84e795b7804838c069947cdcdcaffad4db9a30adae8a3a26f1b57fde2b6` |
| E5 | `lookup<M::Order>(p, r) absent undefined` | `413ec04a0598f66d6220c88e6962616c480a6b41953d35743680b91159d935fa` |
| E6 | `lookup<M::Order>(p, r) absent empty` | `e777c7671eac7a9b82ef592d92ef027ad19653cebf77b1979137819299469587` |
| E7 | `deref(r)`, the operand of `deref(r).total` | `c91086a28d37344934b46fbb5385f2c36d27678a177e23e3405ac39a2b743292` |
| E8 | `deref(r).total` | `f99f1b7a05a43cbee4ce1cd1876024ca1b8951269db2b3f291188318f17092e0` |
| E9 | `r.size()` | `80c890a7c6681709f7d32337e9f922bad27926650038820d6a28377aad77abd7` |
| C1 | precondition clause `true` of `Order.size`, owner (`acme/orders`, `ix://acme/orders/Order/size`) | `71bcc8c0daec1eb4806ee51345cf795f91bbf8b8068c7a767600e2ea75a8151a` |
| C2 | body clause `7` of `Order.size`, same owner | `52186844818f4845b853b788f0783c83a4a97296794473d9d24a83028aa606a5` |
| C4 | precondition clause `true` of `Order.count` | `2f7f52726f381efbde6cbbd6c8ae2041d7a9135b19d5120e1e8bbaff49548ec9` |
| P8 | parameter `self: Reference<M::Sub>`, level 0 | `20f7c6bb72981e62027fa7f2448faef1b4f704284c5965de663fab0bcb143043` |
| L4 | literal `false` | `8bb6712f3a140ad8734b07cd2c08a750b1cc60b224bc57507c4618dfd2b0b7ef` |
| E10 | `false or true`, the effective precondition of `Sub.size` | `37f0058fabf962156a74b6a7047c8e0f7e73461f34b0ac425f027402a4e6bd8e` |
| C5 | precondition clause `false` authored on `Sub.size`, owner (`acme/orders`, `ix://acme/orders/Sub/size`) | `b605457229a7b3796363772b7bbf50b553566f619157c1c1c4f5c01f6e7f78af` |
| C6 | effective precondition `false or true` of candidate `Sub.size`, same owner as C5 | `297edd1460ab012b24e2e0ef864fe47f0c8b48bf2ed121407c01ad7708dfe514` |
| U1 | compound unit `metre^2` | `3509477149439eb6c2f909cae331701d3bef202ae5d7345ddde09c09b17993d7` |
| U2 | compound unit `metre^1 second^-1` | `bcf431d15781d66fcdd2514e251004373eb64a3ef21e37f7b2535f2a871821e7` |
| U3 | the dimensionless compound unit | `0d1d10569b375d6d91aca98ea001f3e20eb118abcd44ea8df6b1b44faa52e42d` |
| U4 | compound unit `metre^1` | `02df6b0ff98d087f2807cd502d84ac503dffe56d1a4af72067975a22f7be7023` |

**M1**: object type `Order`, owner (`acme/orders`, `ix://acme/orders/Order`)

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"model","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/Order"},"recursion":null,"semantic_form":"object_type","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `6f006fa31e8ceeb4c7e6ffb71a68a7883643c931f90d487d35239d49390c596b`

**M3**: object type `Invoice`, owner (`acme/orders`, `ix://acme/orders/Invoice`)

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"model","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/Invoice"},"recursion":null,"semantic_form":"object_type","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `4b07c3fd269e699bdd4bc1065acc80d2de047cc01e20fcd8bcf27fe404f79583`

**M4**: relationship `billedTo`, owner (`acme/orders`, `ix://acme/orders/billedTo`)

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"relation","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/billedTo"},"recursion":null,"semantic_form":"relationship","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `aa598d3142945d973a62e44cd87de545bd4e0046352b1669777b32abd074ed54`

**M5**: object type `Sub` (supertype `Order`), owner (`acme/orders`, `ix://acme/orders/Sub`)

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"model","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/Sub"},"recursion":null,"semantic_form":"object_type","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `c4df362db09cab29d75d4fda10da8c1ba4fe91e1a285721a78198dbfb1b5429d`

**R1**: `Reference<Order>` (M1)

```json
{"body":{"members":[{"target":{"digest":"6f006fa31e8ceeb4c7e6ffb71a68a7883643c931f90d487d35239d49390c596b","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":null,"semantic_form":"reference","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `249d3baf1f4728325f4b5194d54320ad6d7e11bc88de7a339504ea6582fe1fbb`

**R3**: `Option<Reference<Order>>`

```json
{"body":{"members":[{"target":{"digest":"249d3baf1f4728325f4b5194d54320ad6d7e11bc88de7a339504ea6582fe1fbb","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":null,"semantic_form":"option","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `473130ae8fc4a4088490090342954508c9f83d7c425ce9d40eb30609936ebecd`

**S1**: `Set<Reference<Order>>`

```json
{"body":{"members":[{"target":{"digest":"249d3baf1f4728325f4b5194d54320ad6d7e11bc88de7a339504ea6582fe1fbb","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":null,"semantic_form":"set","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `b8a0b4c1fcbcb8672f21a16c2373abed9ba6469cc9fff3288408349f288dbed5`

**S2**: `Set<Reference<Order>>[0, 3]`

```json
{"body":{"members":[{"name":"min","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}},{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"3","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"collection_bounds","semantic_type":{"digest":"b8a0b4c1fcbcb8672f21a16c2373abed9ba6469cc9fff3288408349f288dbed5","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `38373b6eb15c08c8e4c980a6f432b200e4e476b7d383b2149df45ef9a6a7f365`

**PO1**: `Population<Order>[3]`

```json
{"body":{"members":[{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"3","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"model_population","semantic_type":{"digest":"b8a0b4c1fcbcb8672f21a16c2373abed9ba6469cc9fff3288408349f288dbed5","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `859d6b01a132a14711a8a4568e24a7ab54e3255b78bbcc62b14e4293dbf71f9a`

**PO2**: `Population<Order>[5]`

```json
{"body":{"members":[{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"5","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"model_population","semantic_type":{"digest":"b8a0b4c1fcbcb8672f21a16c2373abed9ba6469cc9fff3288408349f288dbed5","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `1ad3769488d608a16a67c8820f549452822f72f4bf9428e01257720fcbaeb1e2`

**R4**: `Reference<Sub>` (M5)

```json
{"body":{"members":[{"target":{"digest":"c4df362db09cab29d75d4fda10da8c1ba4fe91e1a285721a78198dbfb1b5429d","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":null,"semantic_form":"reference","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `76c98302f8b0a4e2795c710719cb08010112a885a02812150f6c78729c4c87d2`

**R5**: `Reference<Invoice>` (M3)

```json
{"body":{"members":[{"target":{"digest":"4b07c3fd269e699bdd4bc1065acc80d2de047cc01e20fcd8bcf27fe404f79583","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":null,"semantic_form":"reference","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `c1d7d9c83f3f3d47136388b30219177d7b0eb47f245f9ee8237db8b0a4ce9396`

**S3**: `Set<Reference<Invoice>>`

```json
{"body":{"members":[{"target":{"digest":"c1d7d9c83f3f3d47136388b30219177d7b0eb47f245f9ee8237db8b0a4ce9396","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":null,"semantic_form":"set","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `f3f5a65e5ec9dd964b4f7cb7cdb345ec1b1443cafbfb589f1ec59ad72cf904ce`

**PO3**: `Population<Invoice>[3]`

```json
{"body":{"members":[{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"3","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"model_population","semantic_type":{"digest":"f3f5a65e5ec9dd964b4f7cb7cdb345ec1b1443cafbfb589f1ec59ad72cf904ce","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `73c35350eabf6d7c2ddcb20e40aa8bcd89056ccf6f320a281b2b9e64a2846502`

**P5**: parameter `p: Population<M::Order>[3]`, level 0

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"p","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"859d6b01a132a14711a8a4568e24a7ab54e3255b78bbcc62b14e4293dbf71f9a","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `6d40c507a2bfe6f3c3b70ae2b49b2a46f0fb82f9a1df2b2ef7fdd472342dd117`

**P6**: parameter `r: Reference<M::Order>`, level 1

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"r","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"249d3baf1f4728325f4b5194d54320ad6d7e11bc88de7a339504ea6582fe1fbb","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `3f80443940b49c4f958abcb45bde0874211b549b90e9d9ca158c2f42ef1280f1`

**P7**: parameter `self: Reference<M::Order>`, level 0

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"self","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"249d3baf1f4728325f4b5194d54320ad6d7e11bc88de7a339504ea6582fe1fbb","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `18cf5ed16dd555a52d0695ca3194de4835b4475cee06c2a6329f57bd147bc7e5`

**E4**: `allInstances<M::Order>(p)`

```json
{"body":{"arguments":[{"target":{"digest":"6d40c507a2bfe6f3c3b70ae2b49b2a46f0fb82f9a1df2b2ef7fdd472342dd117","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.model.all_instances","laws":[],"leaves":[],"member":{"declaration":{"digest":"6f006fa31e8ceeb4c7e6ffb71a68a7883643c931f90d487d35239d49390c596b","domain":"quire.checked-semantic-node/v1"},"kind":"type_argument"},"mode":null},"operator":"query","result_type":{"digest":"38373b6eb15c08c8e4c980a6f432b200e4e476b7d383b2149df45ef9a6a7f365","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"query","semantic_type":{"digest":"38373b6eb15c08c8e4c980a6f432b200e4e476b7d383b2149df45ef9a6a7f365","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `66fcc84e795b7804838c069947cdcdcaffad4db9a30adae8a3a26f1b57fde2b6`

**E5**: `lookup<M::Order>(p, r) absent undefined`

```json
{"body":{"arguments":[{"target":{"digest":"6d40c507a2bfe6f3c3b70ae2b49b2a46f0fb82f9a1df2b2ef7fdd472342dd117","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"3f80443940b49c4f958abcb45bde0874211b549b90e9d9ca158c2f42ef1280f1","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.model.lookup","laws":[],"leaves":[],"member":{"declaration":{"digest":"6f006fa31e8ceeb4c7e6ffb71a68a7883643c931f90d487d35239d49390c596b","domain":"quire.checked-semantic-node/v1"},"kind":"type_argument"},"mode":{"kind":"absence","value":"undefined"}},"operator":"query","result_type":{"digest":"249d3baf1f4728325f4b5194d54320ad6d7e11bc88de7a339504ea6582fe1fbb","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"query","semantic_type":{"digest":"249d3baf1f4728325f4b5194d54320ad6d7e11bc88de7a339504ea6582fe1fbb","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `413ec04a0598f66d6220c88e6962616c480a6b41953d35743680b91159d935fa`

**E6**: `lookup<M::Order>(p, r) absent empty`

```json
{"body":{"arguments":[{"target":{"digest":"6d40c507a2bfe6f3c3b70ae2b49b2a46f0fb82f9a1df2b2ef7fdd472342dd117","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"3f80443940b49c4f958abcb45bde0874211b549b90e9d9ca158c2f42ef1280f1","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.model.lookup","laws":[],"leaves":[],"member":{"declaration":{"digest":"6f006fa31e8ceeb4c7e6ffb71a68a7883643c931f90d487d35239d49390c596b","domain":"quire.checked-semantic-node/v1"},"kind":"type_argument"},"mode":{"kind":"absence","value":"empty"}},"operator":"query","result_type":{"digest":"473130ae8fc4a4088490090342954508c9f83d7c425ce9d40eb30609936ebecd","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"query","semantic_type":{"digest":"473130ae8fc4a4088490090342954508c9f83d7c425ce9d40eb30609936ebecd","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `e777c7671eac7a9b82ef592d92ef027ad19653cebf77b1979137819299469587`

**E7**: `deref(r)`, the operand of `deref(r).total`

```json
{"body":{"arguments":[{"target":{"digest":"3f80443940b49c4f958abcb45bde0874211b549b90e9d9ca158c2f42ef1280f1","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.model.deref","laws":[],"leaves":[],"member":null,"mode":null},"operator":"deref","result_type":{"digest":"6f006fa31e8ceeb4c7e6ffb71a68a7883643c931f90d487d35239d49390c596b","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"deref","semantic_type":{"digest":"6f006fa31e8ceeb4c7e6ffb71a68a7883643c931f90d487d35239d49390c596b","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `c91086a28d37344934b46fbb5385f2c36d27678a177e23e3405ac39a2b743292`

**E8**: `deref(r).total`

```json
{"body":{"arguments":[{"target":{"digest":"c91086a28d37344934b46fbb5385f2c36d27678a177e23e3405ac39a2b743292","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.record.project","laws":[],"leaves":[],"member":{"declaration":{"digest":"6f006fa31e8ceeb4c7e6ffb71a68a7883643c931f90d487d35239d49390c596b","domain":"quire.checked-semantic-node/v1"},"kind":"field","name":"total"},"mode":null},"operator":"query","result_type":{"digest":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"query","semantic_type":{"digest":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `f99f1b7a05a43cbee4ce1cd1876024ca1b8951269db2b3f291188318f17092e0`

**E9**: `r.size()`

```json
{"body":{"arguments":[{"target":{"digest":"3f80443940b49c4f958abcb45bde0874211b549b90e9d9ca158c2f42ef1280f1","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.model.dispatch_call","laws":[],"leaves":[],"member":{"declaration":{"digest":"6f006fa31e8ceeb4c7e6ffb71a68a7883643c931f90d487d35239d49390c596b","domain":"quire.checked-semantic-node/v1"},"kind":"operation","name":"size"},"mode":null},"operator":"call","result_type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"call","semantic_type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `80c890a7c6681709f7d32337e9f922bad27926650038820d6a28377aad77abd7`

**C1**: precondition clause `true` of `Order.size`, owner (`acme/orders`, `ix://acme/orders/Order/size`)

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"18cf5ed16dd555a52d0695ca3194de4835b4475cee06c2a6329f57bd147bc7e5","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"target":{"digest":"03a8898fec9612e8a5faac4927eef10568cd62179e71acc17c86eddb12a792e9","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"clause","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"precondition","value_kind":"text"}}],"term":"aggregate"},"declaration":null,"node_tag":"function","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/Order/size"},"recursion":null,"semantic_form":"pure_function","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `71bcc8c0daec1eb4806ee51345cf795f91bbf8b8068c7a767600e2ea75a8151a`

**C2**: body clause `7` of `Order.size`, same owner

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"18cf5ed16dd555a52d0695ca3194de4835b4475cee06c2a6329f57bd147bc7e5","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"target":{"digest":"9f42fd255e4db2821dffdd2da201755df67dec54e8b0814f8160cab40a44b29b","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"clause","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"body","value_kind":"text"}}],"term":"aggregate"},"declaration":null,"node_tag":"function","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/Order/size"},"recursion":null,"semantic_form":"pure_function","semantic_type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `52186844818f4845b853b788f0783c83a4a97296794473d9d24a83028aa606a5`

**C4**: precondition clause `true` of `Order.count`

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"18cf5ed16dd555a52d0695ca3194de4835b4475cee06c2a6329f57bd147bc7e5","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"target":{"digest":"03a8898fec9612e8a5faac4927eef10568cd62179e71acc17c86eddb12a792e9","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"clause","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"precondition","value_kind":"text"}}],"term":"aggregate"},"declaration":null,"node_tag":"function","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/Order/count"},"recursion":null,"semantic_form":"pure_function","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `2f7f52726f381efbde6cbbd6c8ae2041d7a9135b19d5120e1e8bbaff49548ec9`

**P8**: parameter `self: Reference<M::Sub>`, level 0

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"self","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"76c98302f8b0a4e2795c710719cb08010112a885a02812150f6c78729c4c87d2","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `20f7c6bb72981e62027fa7f2448faef1b4f704284c5965de663fab0bcb143043`

**L4**: literal `false`

```json
{"body":{"term":"literal","type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"value":false,"value_kind":"boolean"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"literal","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `8bb6712f3a140ad8734b07cd2c08a750b1cc60b224bc57507c4618dfd2b0b7ef`

**E10**: `false or true`, the effective precondition of `Sub.size`

```json
{"body":{"arguments":[{"target":{"digest":"8bb6712f3a140ad8734b07cd2c08a750b1cc60b224bc57507c4618dfd2b0b7ef","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"03a8898fec9612e8a5faac4927eef10568cd62179e71acc17c86eddb12a792e9","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.boolean.or","laws":[],"leaves":[],"member":null,"mode":null},"operator":"binary","result_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"binary","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `37f0058fabf962156a74b6a7047c8e0f7e73461f34b0ac425f027402a4e6bd8e`

**C5**: precondition clause `false` authored on `Sub.size`, owner (`acme/orders`, `ix://acme/orders/Sub/size`)

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"20f7c6bb72981e62027fa7f2448faef1b4f704284c5965de663fab0bcb143043","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"target":{"digest":"8bb6712f3a140ad8734b07cd2c08a750b1cc60b224bc57507c4618dfd2b0b7ef","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"clause","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"precondition","value_kind":"text"}}],"term":"aggregate"},"declaration":null,"node_tag":"function","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/Sub/size"},"recursion":null,"semantic_form":"pure_function","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `b605457229a7b3796363772b7bbf50b553566f619157c1c1c4f5c01f6e7f78af`

**C6**: effective precondition `false or true` of candidate `Sub.size`, same owner as C5

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"20f7c6bb72981e62027fa7f2448faef1b4f704284c5965de663fab0bcb143043","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"target":{"digest":"37f0058fabf962156a74b6a7047c8e0f7e73461f34b0ac425f027402a4e6bd8e","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"clause","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"precondition","value_kind":"text"}}],"term":"aggregate"},"declaration":null,"node_tag":"function","owner":{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/Sub/size"},"recursion":null,"semantic_form":"pure_function","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `297edd1460ab012b24e2e0ef864fe47f0c8b48bf2ed121407c01ad7708dfe514`

**U1**: compound unit `metre^2`

```json
{"body":{"members":[{"members":[{"name":"unit","term":"binding","value":{"target":{"digest":"79637623a46d29e884b62c6fa292aeb29d41e4ecc4e800b4d7ee910a3eaf23a4","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"exponent","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"2","value_kind":"integer"}}],"term":"aggregate"}],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"compound_unit","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `3509477149439eb6c2f909cae331701d3bef202ae5d7345ddde09c09b17993d7`

**U2**: compound unit `metre^1 second^-1`

```json
{"body":{"members":[{"members":[{"name":"unit","term":"binding","value":{"target":{"digest":"79637623a46d29e884b62c6fa292aeb29d41e4ecc4e800b4d7ee910a3eaf23a4","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"exponent","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"}}],"term":"aggregate"},{"members":[{"name":"unit","term":"binding","value":{"target":{"digest":"d0f0c5d24d6accb2f1d9afff7c6cdda3143df382e50af4bda0704d48c67ed998","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"exponent","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"-1","value_kind":"integer"}}],"term":"aggregate"}],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"compound_unit","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `bcf431d15781d66fcdd2514e251004373eb64a3ef21e37f7b2535f2a871821e7`

**U3**: the dimensionless compound unit

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"compound_unit","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `0d1d10569b375d6d91aca98ea001f3e20eb118abcd44ea8df6b1b44faa52e42d`

**U4**: compound unit `metre^1`

```json
{"body":{"members":[{"members":[{"name":"unit","term":"binding","value":{"target":{"digest":"79637623a46d29e884b62c6fa292aeb29d41e4ecc4e800b4d7ee910a3eaf23a4","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"exponent","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"}}],"term":"aggregate"}],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"compound_unit","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `02df6b0ff98d087f2807cd502d84ac503dffe56d1a4af72067975a22f7be7023`

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
| FR-094-AC-6 | With `a` a quantity of QSpec's declared unit `metre` and `t` one of `second`, `a`'s parameter has semantic type `79637623a46d29e884b62c6fa292aeb29d41e4ecc4e800b4d7ee910a3eaf23a4`, QSpec's `unit-metre` key, and the graph holds no `compound_unit` node for it. In a function body, the result type of `a * a` keys to U1. The compound units that `value::quantity::result_unit` forms for `a / t`, `a / a` and `(a * a) / a`, held in the check stage's unit scope, key to U2, U3 and U4. U2's first term names metre, and U4 differs from the `metre` unit key. | Test (TC-419) |
| FR-094-AC-7 | Keying a `Reference` whose `EffectiveId` is no `type_identities` value, a `DeclarationKey` whose `package` is no admitted identity, a `DeclarationKey` whose `node` is empty, a compound `UnitId` the unit scope does not hold, a declared unit for which the unit table holds no admitted node (a table built from the graph's units without `UnitTable::declared`), and a field member record reaching the record-kind `match` each refuses as an internal fault naming that value, and yields no key. | Test (TC-417, TC-419) |
| FR-094-AC-8 | Under a unit graph its own source declares, with base dimensions `Length` and `Time`, the derived dimension `Velocity = Length / Time`, root units `metre`, `second` and `mps` (of `Velocity`) and `km = 1000 × metre`, a record `Trip{d: km, v: mps}` is emitted with no node omitted. The `km` node depends on `Length` and `metre` and its preimage targets `metre` with scale `1000`; the `Velocity` node depends on `Length` and `Time` and its preimage holds both terms; `mps` is typed by `Velocity`; and IR's checked-package/v2 reader admits the package. | Test (TC-419) |

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

