---
id: FR-569
title: "Declare object types, populations and operations in QSL source"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-001
    type: implements
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-531
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-150
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-151
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-342
    type: depends_on
---
# FR-569: Declare object types, populations and operations in QSL source

## Description

When a complete-V1 unit declares an object type in its own source (its
fields, its object identity, its supertypes, the populations of its objects
and its operations with their signatures and frames), S1, S2, the assembler,
S3 and S4 SHALL admit, check and emit it as below. A
source-declared object type SHALL check, lower and evaluate exactly as an
object type that a selected domain package declares (FR-056, FR-094), except
that it is owned by the unit's source rather than by a domain package. Its
operations take the unit's existing `pre`, `post` and `body` declarations
(FR-104, QSpec FR-151 `operation-body`), and its `state`/`operation_anchor`
nodes are the source-declared anchors that FR-390 "Source-declared operation
calls from the reader" evaluates.

Today a unit names object types and operations only through a model alias
(`M::Node`, `M::Node::step`); the grammar has no object or operation
declaration (`qsl-cst/src/grammar.rs`, `source_and_types`), and QSpec's
shared grammar states "A native declaration cannot declare a domain object"
(`proposals/quire-v1/shared-grammar.md`). This requirement adds the
declaration, and ADR-013 QC-30 carries the QSpec changes it needs.

Out of scope: a source object type declares no relationship, component,
endpoint, interface feature, state machine or process; a source field
declares no `subsets` or field redefinition; a source object type does not
specialize a model-owned type and a model-owned type does not specialize a
source-declared one. Each of those stays a domain package construct.

## Inputs

- A complete-V1 source unit (FR-091's E1 and E2 input) and its selected
  domain packages, if any.
- The unit's source owner (authority, identity) and its checked package name.

## Outputs

- S2: one `DeclarationForm::Object(ObjectForm)` per object declaration,
  carrying its span and the spans of its parts.
- Assembly: the unit's source model, the object-type, field, operation and
  population records below.
- S4: the `model`/`object_type` nodes, the `state`/`operation_anchor` and
  `state`/`frame` nodes of its named operations, the population reference
  nodes of its named populations, and one source-object identity row per
  emitted source-declared object type.
- Or a located refusal and no checked package.

## Behavior

### Surface syntax

S1 SHALL parse these productions as declarations of a complete-V1 unit,
written in the shared-grammar notation; `field` and `parameter` are the
existing productions:

```ebnf
object-decl       = [ 'abstract' ], 'object', ident,
                    [ 'extends', ident, { ',', ident } ],
                    [ 'identity', '(', ident, { ',', ident }, ')' ],
                    '{', { object-member }, '}' ;
object-member     = field | population-member | operation-member ;
population-member = [ 'symmetric' ], 'population', ident, ';' ;
operation-member  = 'operation', ident,
                    '(', [ parameter, { ',', parameter } ], ')',
                    [ ':', type-ref ],
                    [ 'redefines', ident, '::', member-name ],
                    { frame-clause }, ';' ;
frame-clause      = 'modifies', field-target, { ',', field-target }
                  | 'creates', ident, { ',', ident }
                  | 'deletes', delete-target, { ',', delete-target } ;
field-target      = 'self', '.', ident | ident, [ '.', ident ] ;
delete-target     = 'self' | ident ;

type-name         = model-name | ident ;
operation-name    = type-name, '::', member-name ;
population-name   = model-name | ident ;
```

Every word of these productions is a reserved spelling, as every grammar
word of a complete-V1 unit is (`base_reserved_spellings`): `object`,
`operation`, `population`, `abstract` and `identity` already are, and
`extends`, `symmetric`, `redefines`, `modifies`, `creates` and `deletes`
become reserved and are refused as identifiers. The last three productions replace
`TypeName = ModelName` and `OperationName = TypeName :: member` and extend the
population positions (`PopulationBinder`, `PopulationBound`, `PopulationRow`
and FR-391's population positions) to a single identifier. A one-segment
type name, operation type or population name names a source declaration; a
two-segment model name names a model declaration, as before. Everything
that takes a `TypeName`, `OperationName` or population name today takes the
source form with no other change: a state clause `on Node` or `on
Node::step`, an operation body `body b using P on Node::step(x) { ... }`, a
protocol `over (view: Node)`, `role R on Node` and `by R on Node::step`, a
population bound `bound b on nodes [3]`, a quantifier domain `x in nodes`,
`Reference<Node>`, `Population<Node>[3]`, `allInstances<Node>(nodes)` and
`lookup<Node>(nodes, r)`.

The parts of an object declaration mean:

- `abstract`: the type has no direct instances (QSpec FR-151, D05).
- `extends`: the type's direct supertypes, in source order.
- `identity`: the type's identity fields, own or inherited, in source order;
  without it the type declares none and inherits its supertypes'.
- a `field`: a field member; `?` makes its presence optional. Its type is the
  written type, a collection type for a many-valued field, exactly as a
  source record field.
- a `population-member`: a closed population whose member type is the
  declaring type; `symmetric` marks it symmetric (QSpec FR-150).
- an `operation-member`: an operation with its parameters in order, its
  result (none when `:` is absent), the inherited operation it redefines and
  its frame. `modifies self.f` is a receiver-scoped field entry, `modifies
  f` an every-object entry of the declaring type's field `f` (own or
  inherited) and `modifies T.f` one of the field `f` of the source object
  type `T`,
  `creates T` and `deletes T` type-wide entries over a source object type,
  `deletes self` the receiver deletion and `deletes p` the deletion of the
  reference parameter `p` (FR-154). A frame names only the unit's source
  object types, their fields and the operation's own parameters; the
  grammar admits no model name in it. An operation with no frame clause
  writes nothing (a query).

S2 SHALL build one `ObjectForm` per `object-decl`, its members in source
order, without reordering, merging or deduplicating them; S3 and the
assembler check them. An `object-decl` that does not parse is the
`invalid_syntax` diagnostic at the offending token, as for any declaration.

### The source model

The assembler SHALL build, from the unit's object declarations, the same
normalized records that domain intake builds from an admitted domain package
(FR-056, `qsl-semantics/src/model/domain_package.rs`): one
`ObjectTypeRecord` per type (its `abstract_type`, its `supertypes`, no
`interface_features`), one `FieldMemberRecord` per field (exactly-one
multiplicity, the declared presence, no subsets), one `OperationMemberRecord`
per operation (its parameters, its result, its `OperationEffect` from the
frame clauses, its `redefines`, its `requires` and `ensures` as below, and
`has_body` set from the unit's `body` declarations as it is for a model
operation) and one `PopulationRecord` per population (member type the
declaring type, extent `closed`, the `symmetric` marker). Their declaration
keys belong to the unit's source owner, never to a domain package, and
no ModelSelection, domain package bytes, FR-154 admission or `ModelOwner`
takes part. The source model is one closed declaration set: FR-081's
effective view, FR-103's operations and frames, QSpec FR-150 and FR-151
(conformance, redefinition, abstract types and closed dispatch), FR-104's
clause resolution and FR-106 and FR-107's admission and evaluation SHALL run
over it with the same code paths and the same rules as over a model's
effective view.

The `pre` and `post` state clauses that name a source-declared operation are
that operation's contract clauses, its `requires` and `ensures` in source
order: FR-104 checks them as clauses, and QSpec FR-151's redefinition rule
(a redefinition cannot weaken an inherited contract) and its dispatch read
them as a domain operation's `pre` and `post` entries. A `body` declaration
naming a source-declared operation supplies its query body as for a model
operation.

### Name resolution and collisions

- The unit's source namespace holds its records, tuples, unions, enums,
  aliases, functions, predicates, state clauses, object types and
  populations. A source object type or population whose name equals another
  declaration of that namespace, a model alias, a profile alias or an import
  alias of the unit SHALL be refused `ambiguous_declaration`/`ambiguous-name`
  at every declaration of that name, naming each.
- A one-segment type name SHALL resolve only to a source object type and a
  two-segment one only to a model type; neither falls back to the other.
  A source object type whose name equals a type of a selected domain package
  (`Node` beside `M::Node`) is a different type: `Node` always names the
  source type, `M::Node` always the model type, and no resolution, identity
  row, snapshot name or evaluation substitutes one for the other.
- A one-segment type name that names no source object type SHALL be refused
  `missing_declaration`/`missing-name` at the name, also when it names a
  source record or another source declaration.
- `T::op` SHALL resolve `op` through `T`'s effective view as FR-104 resolves
  `M::T::op` (a more derived declaration hides an ancestor's; two
  incomparable declaring ancestors refuse `ambiguous_declaration`/
  `ambiguous-name`), and an operation `T` neither declares nor inherits SHALL
  be refused `missing_declaration`/`missing-name` at `op`.
- A one-segment name in a population position SHALL resolve to the source
  population of that name. A name that is both a source population and a
  binder or parameter in scope SHALL be refused `ambiguous_declaration`/
  `ambiguous-name` at the use, naming both.
- Within one object type, two members (fields, populations, operations) of
  one name SHALL be refused `ambiguous_declaration`/`ambiguous-name` at the
  second, and so SHALL a field whose name equals an inherited field. An
  operation whose name equals an inherited operation's and that carries no
  `redefines` SHALL be refused `ambiguous_declaration`/`ambiguous-name` at
  the operation.

### Declaration checks

The assembler and S3 SHALL refuse, each at the offending name:

- an `extends` entry that names a model type, or a source declaration that is
  not an object type: `invalid_model_binding`/`malformed-declaration`; one
  that names nothing: `missing_declaration`/`missing-name`; a supertype
  cycle: `invalid_package`/`definition-cycle`, naming the cycle's edges;
- an `identity` entry that names no field of the type's effective view:
  `missing_declaration`/`missing-name`;
- a `redefines T::op` whose `T` is not a proper supertype or whose `op` `T`
  does not expose: `missing_declaration`/`missing-name`; a redefinition that
  breaks QSpec FR-151's conformance axes: the refusal FR-151 gives the same
  domain declaration;
- a frame entry that names no field, source object type or parameter:
  `missing_declaration`/`missing-name`; a `deletes p` whose `p` is not a
  `Reference` parameter: `invalid_model_binding`/`malformed-declaration`; a `deletes` name that is
  both a parameter and a source object type: `ambiguous_declaration`/
  `ambiguous-name`;
- a field, parameter or result type that does not resolve: the type
  refusals FR-091 gives any declaration.

A field, parameter or result may have any type a record field may have,
model-owned value types (`M::Total`) and `Reference<M::Order>` included.

### Lowering and identity

S4 SHALL emit, for every source-declared object type, whether or not a
checked node names it, one `model`/`object_type` node with:

- `node_id`: QSpec FR-322's structural id of a declared node, as FR-092
  keys every declared node: `package` the checked package's name, `path` the
  declaration path, ordinal by FR-092's producer rule;
- `declaration`: `{qualified_name: [<type name>]}`;
- `owner`: the unit's `SourceOwner` `{kind: "source", authority, identity}`;
- `semantic_type`: itself; body `aggregate{[]}`;
- one `declaration` occurrence at the object declaration's span.

The node id is a structural id, never a hash: no digest of the declaration,
its members or its text is computed (ADR-013 QC-2). Changing a field, an
operation or a population leaves the type's node id unchanged; renaming the
type changes it. A model-owned `Node` and a source-declared `Node` have
different ids and different owners.

A field, operation or population member has no node of its own, exactly as
for a model-owned type (FR-094): an FR-322 member names it by the source
object type's node and its name, and a `state`/`operation_anchor` names an
operation by its `context`, the node of the source object type that declares
it, and its `operation` text (FR-105). The anchor's frame node, its
`operation_effects` row (FR-496) and its clause functions are built from the
source `OperationEffect` and clauses as for a model operation. A clause
function of a source-declared operation carries `declaration`
`{qualified_name: [<type name>, <operation name>]}` and the unit's
`SourceOwner` in place of a model operation's `ModelOwner` and `declaration`
`null`, with the same trailing `clause` binding (FR-094).

A source population that a checked node names (FR-391) SHALL be emitted as a
`relation`/`population` population reference node with `declaration`
`{qualified_name: [<population name>]}` and the unit's `SourceOwner`, keyed
as a declared node; a source population no checked node names has no node.
This shape is ADR-013 QC-30's proposal to QSpec FR-322.

### Source-object identity rows

A bytes-only consumer has no domain package document for a source-declared
type, so S4 SHALL produce, at the model-selection identity stage with the
source-record and source-operation rows (FR-390), one source-object identity
row for each emitted source-declared `model`/`object_type` node:
`structural_node` (the node), `source_authority`, `source_identity` (the
type's qualified source name), `abstract`, `supertypes` (the structural ids
of the direct supertype nodes in source order), `identity_fields` (field
names in declared order), `members` (one entry per own field, in source
order, in the `source_records` member encoding of FR-531) and `populations`
(one `{name, symmetric}` entry per own population, in source order). The row's
key is (`source_authority`, `source_identity`). The rows are written to and
read from the `source_objects` member of `identities.json` (FR-531 "The
identity file", FR-517); none is derived from package bytes, decoded nodes,
display names or source text. The reader view and the bytes route SHALL use
these rows where they use a model-owned type's admitted document: for field
types, identity fields, supertypes, abstractness and populations. Admission
follows FR-390's source-operation rows: at the `Package` locus, a row of
another emission is `ForeignIdentityWitness` (winning over
`UnknownIdentityWitness`); a row naming a node that is not a source-declared
`model`/`object_type` of the package, or whose authority or identity
disagrees with the node, is `UnknownIdentityWitness`; an emitted
source-declared object type with no row is `MissingIdentityWitness`; each
with no partial view and no meter charge.

### Observations and evaluation

A snapshot (FR-106 "Document forms") SHALL name a source-declared
population and type by its `source_identity`, the declared name as the unit
writes it (`"population": "nodes"`, `"type": "Node"`), in the
`population`, `type` and object-reference positions where it names a model
declaration by its `ix://` declaration identity. A name with no `ix://`
scheme names a source declaration of the checked package and never a model
declaration. A snapshot's `model` member is required exactly when the
checked package selects a domain package. An object's model component is the
unit's source owner for a source-declared type. A name that names no
declaration of the package is FR-106's unknown-declaration refusal, and an
object whose type is abstract is refused `invalid_runtime_input`/
`abstract-instance`, as for a model type.

Every evaluation over a source-declared object type SHALL give the outcome
and the ordered charge log that the same evaluation gives over its model
twin, the domain package type with the same fields, identity, supertypes,
populations, operations, frames and clauses: invariants, constraints,
preconditions and postconditions over pre and post snapshots (FR-107),
`allInstances`, `lookup` and population domains (FR-084, FR-391), dispatched
query calls and operation bodies (QSpec FR-151), frame checks (FR-103,
FR-154), protocol slots (FR-390), and every temporal claim and offline monitor
(FR-283) whose `over` parameter or state reads the type. Only the node ids,
owners and identity rows differ.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-569-AC-1 | TC-1408 fixture S parses: one `ObjectForm` per object declaration, `Order` with fields `total: Int[0, 9]` (required) and `note: Text[0, 8; nfc]?` (optional), identity `(total)`, population `orders`, operations `size(): Integer`, `count(): Integer`, `scaled(n: Integer): Integer` and `bump(): Boolean modifies self.total`; `Sub extends Order` with `size(): Integer redefines Order::size` and `scaled(m: Integer): Integer redefines Order::scaled`; `abstract object Shape { population shapes; }` with one population and no field or operation; each part in source order with its span. `operation step()` with no `;`, `modifies self.` with no field name and `object Node extends {}` each give one `invalid_syntax` diagnostic at the offending token and no `ObjectForm`. `extends` or `modifies` written as a parameter name is refused as a reserved word. | Test (TC-1408) |
| FR-569-AC-2 | Over fixture S, `invariant I using P on Order at current { self.total >= 0 }`, `pre Q using P on Order::bump { self.total < 9 }`, `body B using P on Sub::size() { 7 }`, `forall o in orders: deref(o).total >= 0` and `Reference<Sub>` resolve to the source declarations; `on Order::reset` refuses `missing_declaration`/`missing-name` at `reset`; `on Receipt` naming a source record and `on Nothing` refuse `missing_declaration`/`missing-name` at the name. With a domain package that also declares `Order` selected as `M`, `on Order` checks against the source fields and `on M::Order` against the model's, and no name resolves to the other. | Test (TC-1408) |
| FR-569-AC-3 | Each refuses `ambiguous_declaration`/`ambiguous-name` at the stated spans and yields no checked package: a source `record Order` beside `object Order` (both declarations); `object M` in a unit with `model M` (both); population `orders` beside a function `orders` (both); a quantifier binder `orders` over `orders` (the use, naming both); two fields `total` in one type (the second); a field of `Sub` named `total` (the field); an operation `size` in `Sub` without `redefines` (the operation); `deletes Order` in an operation whose parameter is also named `Order` (the entry). | Test (TC-1408) |
| FR-569-AC-4 | Each refuses with the code and cause stated, at the offending name: `extends M::Order` and `extends Receipt` (a source record) `invalid_model_binding`/`malformed-declaration`; `extends Ghost` `missing_declaration`/`missing-name`; `A extends B` with `B extends A` `invalid_package`/`definition-cycle` naming both edges; `identity (ghost)`, `redefines Order::reset`, `modifies self.ghost` and `creates Ghost` `missing_declaration`/`missing-name`; `deletes n` with `n: Integer` `invalid_model_binding`/`malformed-declaration`; a redefinition `scaled(m: Text[0, 8; nfc]): Integer` the refusal QSpec FR-151 gives the same domain declaration. | Test (TC-1408) |
| FR-569-AC-5 | Over fixture S and its model twin D (TC-1408), the checked effective views, the dispatch tables and the checked types agree member for member: `r.size()` over `r: Reference<Sub>` dispatches to `Sub.size` and over `Reference<Order>` has candidates `Order.size` and `Sub.size`; `deref(s).total` over `s: Reference<Sub>` names `Sub`'s node and `total`; `allInstances<Order>(orders)` has type `Set<Reference<Order>>`, and `Set<Reference<Order>>[0, 3]` with `bound b on orders [3]`. `Q` is `bump`'s `requires`, and a `pre` on `Sub::bump` weaker than an inherited one refuses as QSpec FR-151 refuses the same weakening in D. | Test (TC-1408) |
| FR-569-AC-6 | S4 over fixture S emits `Order`, `Sub` and `Shape` as `model`/`object_type` nodes with `declaration` `["Order"]`, `["Sub"]` and `["Shape"]`, the unit's `SourceOwner`, self `semantic_type`, body `aggregate{[]}` and one `declaration` occurrence each; `Shape`, which nothing names, is emitted. No node is emitted for a field, operation or population member. `Q`'s anchor has `context` `Order`'s node, `operation` `"bump"` and a frame node whose `semantic_type` is `Order`'s node and whose `modifies` holds (`Order`'s node, `total`, `receiver`); `B`'s clause function carries `declaration` `["Sub", "size"]`, the `SourceOwner` and the `clause` binding `body`. The forall's population reference is a `relation`/`population` node with `declaration` `["orders"]` and the `SourceOwner`; a unit that names no population has none. | Test (TC-1408) |
| FR-569-AC-7 | `Order`'s node id is unchanged when a field is added to `Order`, an operation is renamed or the declaration is moved within the unit, and changes when `Order` is renamed. D's `Order` node is `{<domain package>, [model, object_type, <IR node>], 0}` with a `ModelOwner` and S's is the declared-node id with the `SourceOwner`; the two differ. No digest is computed for either. | Test (TC-1408) |
| FR-569-AC-8 | The emission over S carries one source-object row per type: `Order` with `abstract` false, no supertypes, `identity_fields` `["total"]`, members `total` then `note` (optional) and populations `[{name: "orders", symmetric: false}]`; `Sub` with `supertypes` `[Order's node]`; `Shape` with `abstract` true, no member and populations `[{name: "shapes", symmetric: false}]`. D's emission has none. The rows are written to `identities.json` `source_objects` and read back equal. Over S, a supplier with the `Order` row removed gives `MissingIdentityWitness`, a row with `source_identity` `Ghost` or with `structural_node` a frame node `UnknownIdentityWitness` only, and the `Order` row of an emission of S under another source authority `ForeignIdentityWitness` only, each at the `Package` locus with no view and no charge. | Test (TC-1408) |
| FR-569-AC-9 | A snapshot of S naming population `orders` and objects of type `Order` and `Sub` admits; the same snapshot naming `ix://test/orders/Order` for a source type is refused as an unknown declaration; an object of type `Shape` in population `shapes` refuses `invalid_runtime_input`/`abstract-instance`; with no domain package selected, a snapshot with no `model` member admits and one with a `model` member is refused. | Test (TC-1408) |
| FR-569-AC-10 | For each TC-1408 vector V1 to V6 (`I`, `Q`, a `post` on `bump` over a pre and post snapshot, `forall o in orders`, `r.size()` dispatched to `Sub.size`, and the frame check of an invocation of `bump` that writes `total` of a second object), S and D give equal outcomes and equal ordered charge logs on the source route. | Test (TC-1408) |

## Dependencies

- [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md):
  S2 forms, assembly and the catalog codes used above.
- [FR-081](FR-081-preserve-model-correspondence-and-declaration-identity.md),
  [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md),
  [FR-154](FR-154-scope-a-modifies-entry-to-the-receiver.md): the effective
  view, operations, frames and scoped entries the source model reuses.
- [FR-092](FR-092-key-type-parameter-and-declared-nodes.md) and
  [FR-094](FR-094-key-model-owned-reference-population-and-quantity-nodes.md):
  declared-node identity and the member naming the source type reuses.
- [FR-104](FR-104-check-state-clauses.md),
  [FR-105](FR-105-emit-state-nodes.md),
  [FR-106](FR-106-admit-snapshots-and-invocations.md),
  [FR-107](FR-107-evaluate-state-clauses-at-s6a.md),
  [FR-391](FR-391-name-a-population-in-a-state-clause.md): clause checking,
  emission, admission and evaluation over the source model.
- [FR-390](FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md),
  [FR-531](FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md),
  [FR-517](FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md):
  the source-operation rows and the identity file that carries the
  source-object rows.
- QSpec FR-150, FR-151 (effective view, conformance, redefinition and
  dispatch), FR-322 (node identity, owners, population references) and
  FR-342 (operation anchors).
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  QC-30: the QSpec shared-grammar and FR-322/FR-342 changes. QSL implements
  this requirement by its proposal now. IR's strict reader on main admits a
  `model`/`object_type` node that carries a `declaration` and a
  `SourceOwner`, and resolves no operation of it against a domain document
  (`quire-contract-model` `checked_package/v2/state.rs`). Until QSpec and IR
  adopt QC-30, a package holding a source population reference node fails the
  strict reader's `DeclarationTagRules` (`malformed_wire`), so its reader and
  bytes routes are unavailable; the source route is unaffected.
