---
id: FR-092
title: "Build type, parameter, value and declared function nodes with structural identity"
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
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: traces_to
---
# FR-092: Build type, parameter, value and declared function nodes with structural identity

## Description

`check` builds the checked type, parameter, value, function and protocol nodes
below. QSpec FR-322 (quire-specification main at 686e49e8) owns their
`node_id`: the closed `{package, path, ordinal}` object, compared
componentwise and never hashed. Its `package` is the containing package's
name, a declared node's path is its declaration path, and a derived node's
path and producer-assigned ordinal follow FR-322's structural node identity
rules. This requirement does not define an identity preimage or digest.

## Inputs

- The checked declarations, types, bodies, binders and bound protocol targets.
- The package name, source owners, graph dependencies and checking limits.

## Outputs

- Checked nodes with QSpec FR-322 structural ids, or typed refusal with no
  partial checked package.

## Behavior

### Node identity and ownership

`check` SHALL give every emitted node a QSpec FR-322 `node_id`. A declared
node carries its source owner and its declaration path. A derived node's id
uses the FR-322 path for its tag, semantic form and operands outside a
recursion group; inside a group FR-322's group rules apply. The producer
assigns an ordinal unique among nodes sharing one package and path. Equal
ids denote one node; two distinct nodes SHALL NOT reuse an id. Node bodies,
source locations and JSON bytes are not hashed to derive ids.

An anonymous builtin or composite type has no declaration or source owner.
A declared type carries its source owner. A source alias adds no type node.
The checked graph resolves every reference to the structural id of its
actual target. `check` SHALL preserve the exact owner and source-map region
of each declaration even when moving it between units does not change its
package and declaration path.

For source dimensions, the checked nominal payload SHALL preserve the
explicit base-versus-derived discriminator independently of normalized
terms. A base dimension and a derived dimension whose terms cancel to empty
remain distinct declared nodes. The v2 dimension preimage and replay
admission SHALL preserve their respective `base` values; replay MUST NOT
infer that a dimension is base solely because its terms are empty. The v1
preimage's empty-means-base rule remains version-specific.

### Source enum identity rows

For each source enum declaration node it emits, the `Emission` SHALL carry
one IR `SelectedEnumDeclarationRow`, and for each of that enum's member
nodes one IR `SelectedEnumMemberRow`. A declaration row holds the node's
emitted structural id, its source owner's authority and identity, its
qualified declaration segments and its node key. A member row holds the
member node's emitted id, the same source owner, the declaration's emitted id
and qualified declaration segments, the exact case and the member's node key.
`check` produces these rows from the admitted enum it lowered, never from
package bytes, decoded nodes or display names. A model-owned variant type
carries the model enum rows instead, and no source enum row. An enum node
that is not emitted has no row. A collection type is structural and has no
declaration row. Each list holds its rows in ascending order of their node
keys; the order carries no other meaning.

### Source union identity rows

For each source union declaration node it emits, the `Emission` SHALL carry
one IR `SelectedSourceUnionRow`. The row holds the node's emitted structural
id, its source owner's authority and identity, its qualified declaration
segments, the union's type handle (the key every `ValueType::Composite` of it
carries) and its settled key (the node key every sealed member names). It
holds one member entry per declared member, in declaration order. Each entry
holds the sealed `UnionMember` the check minted for that member, the emitted
ids of the member's payload type nodes in position order, and the payload
`ValueType`s in the same order. The payload ids are the targets of the
references in the union node's member binding. `check` produces these rows
from the checked type environment's sealed declaration and the lowered union
node, never from package bytes, decoded nodes or display names, and never
rebuilds a sealed member or a payload type. A model-owned type carries no
union row. A union node that is not emitted has no row. A union whose
member, payload type or payload node cannot be joined to an emitted node is a
writer fault, not a row with a missing entry. A bare union declaration key
is not a union row. Rows are written in ascending order of their union
nodes' keys; the order carries no other meaning.

### Source record identity rows

For each source record declaration node it emits, the `Emission` SHALL carry
one `SelectedSourceRecordRow`. The row holds the node's emitted structural
id, its source owner's authority and identity, its qualified declaration
segments and the declaration's key (the key every `ValueType::Composite` of
the record carries). It holds one member entry per declared field, in
declaration order, unselected fields included. Each entry holds the declared
field name, the record node's binding name for that field, the declared
presence, the emitted id of the type node the binding references, and the
checked field type. An optional field keeps `Presence::Optional` and its
declared type; a required field of type `Option<T>` keeps `Presence::Required`
and `Option<T>`; neither is inferred from the other. `check` produces these
rows from the checked declaration and the lowered record node, never from
package bytes, decoded nodes or display names. A source record's fields
have no issued member identities, so the row carries none; model records do.
The row keeps both the declared field name and the record node's binding
name, which a writer that renames a binding changes independently; a writer
that renames a binding SHALL carry the final binding name in the row's
`admitted_name`, so the row names each member by the binding the written
package holds at that position (QSL FR-251 "Plan root"). A
model-owned record carries no source record row. A record node that is not
emitted has no row. An emitted source record node that no declared record
joins to, a record whose bindings disagree with its declaration in count, and
a record whose type node is not emitted, are writer faults, not a row with a
missing entry. Rows are written in ascending order of their record nodes' keys; the
order carries no other meaning.

### Source tuple identity rows

For each source tuple declaration node it emits, the `Emission` SHALL carry
one IR `SelectedSourceTupleRow`. The row holds the node's emitted structural
id, its source owner's authority and identity, its qualified declaration
segments and the declaration's key as the type handle (the key every
`ValueType::Composite` of the tuple carries). It holds one position entry per
declared position, in declaration order: the zero-based original ordinal, the
emitted id of the type node the tuple node's reference at that ordinal names,
and the checked position type. A source tuple position has no name and no
member identity, so the row invents none. `check` produces these rows from the
checked declaration and the lowered tuple node, never from package bytes,
decoded nodes or display names. A writer that re-points or renames a node
after lowering SHALL carry the final id in the row, so the row names each
position by the reference the written package holds at that ordinal. A model
tuple carries no source tuple row, and a tuple node that is not emitted has
no row. An emitted source tuple node that
no declared tuple joins to, a tuple whose references disagree with its
declaration in count, and a position whose type node is not emitted, are
writer faults, not a row with a missing entry. Rows are written in ascending
order of their tuple nodes' keys; the order carries no other meaning.

### Source enum shape rows

For each source enum declaration node it emits, the `Emission` SHALL carry
one `SelectedSourceEnumShapeRow` beside the declaration and member rows. The
row holds the declaration's emitted structural id and key, the checked
`EnumShape` of the declaration (its ordered flag and the canonical
`VariantId` sequence, which is declaration order for an ordered enum and
case-identifier byte order otherwise), and one entry per case in that
order: the exact case name, the case's emitted member node id and member key,
and the variant the shape holds at that case's rank. `check` reads the shape
from the admitted enum binding, so a consumer never derives a variant from a
key. The replay facade `AdmittedEnum` exposes the same shape and each case's
variant. A declaration bound twice has one row. A declaration that is not
emitted has no row, and a case whose member node is not emitted is a writer
fault. Rows are written in ascending order of their declarations' keys.

### Source collection rows

A collection type is structural: it has no declaration, owner or declaration
row. For each collection type node it emits, the `Emission` SHALL carry one
`SelectedSourceCollectionRow` pairing that node with the checker's
`ValueType::Collection` it was built from. The row holds the node's emitted
id (the `collection_bounds` domain of a bounded collection, otherwise the
collection composite node), the emitted collection composite node, the
emitted element type node that composite references, and the checked type
with its kind, element and cardinality bound. `check` reads the type from the
collection being lowered, never from package bytes or node forms. A writer
that re-points the element, or the bound's base, SHALL carry the final node
in the row. A value type that is not a collection, a collection node that
does not reference exactly the element, a bounded domain not typed by its
collection node and a node that is not emitted are writer faults. A map has
no type node, so it has no row. Rows are written in ascending order of their
nodes' keys.

### Type nodes

`check` SHALL build the type node of each checked `ValueType` as follows.
Each binding is `{term: "binding", name, value}`; an aggregate lists bindings
or references in the order shown. Bounds are typed scalar literals, not
identity-preimage spellings.

| Checked type | `node_tag` / `semantic_form` | `semantic_type` | Body |
| --- | --- | --- | --- |
| `Boolean`, `Integer`, rational, decimal, text and float base types | `scalar_type` / corresponding scalar form | itself | empty aggregate |
| `Int[lo, hi]` | `bounded_domain` / `integer_range` | `Integer` | `min`, `max` bindings |
| rational, decimal, text and float bounds | `bounded_domain` / corresponding range or bounds form | base scalar | their declared bounds and mode/profile bindings |
| `Option<T>` | `composite_type` / `option` | itself | reference to `T` |
| a collection `K<T>` | `composite_type` / collection form | itself | reference to `T` |
| bounded collection `K<T>[min,max]` | `bounded_domain` / `collection_bounds` | `K<T>` | `min`, `max` bindings |
| declared record | `composite_type` / `record` | itself | fields in declaration order; an optional field has an `optional` group |
| declared tuple | `composite_type` / `tuple` | itself | element references in position order |

The type of a bounded form is its `bounded_domain` node, not its base. A
source declaration supplies its name and owner; a caller-supplied handle is
not a node id. Type construction and expression lowering walk arbitrary
nesting with an explicit heap stack bounded by the stage's node, byte and
work limits (FR-258); neither depth nor stack size changes the result.

### Parameters and functions

`check` SHALL build one `value`/`parameter` node per binder, with the
binder's checked semantic type, no declaration or owner, and an aggregate
body binding its name as text and level as an integer. A function's
parameters have levels `0` to `n-1` in order; a nested binder's level adds
its enclosing binders. A reference to a parameter denotes the enclosing
binder at that level with the same name and type.

A function node has `node_tag: function`, its result semantic type, the
source declaration and source owner. Its `semantic_form` is
`recursive_function` in a recursion group, otherwise `predicate` for a
predicate or `pure_function`. Its aggregate body binds `parameters` in
parameter order, `body` to the checked root expression, and `decreases` to
the measure when present. Unread parameters remain in the list. Its body
contains references, not an inline application. The root expression and
applications are separate nodes. A predicate with a recursion group is
refused under FR-146's missing-measure rule. Model-owned clause functions
carry FR-094's model owner and clause binding.

A recursion group is represented under QSpec FR-322's group rules. The
producer assigns each member's structural id independently of declaration
visit order and never derives an id from group order or an RFC 8785
preimage. Two groups with different declarations retain distinct member
ids; an application in one group does not collide with an application in
another merely because their bodies are equal.

### Protocol reference nodes

`check` SHALL build one `protocol`/`protocol_reference` node for references
to a bound local protocol. Its body is exactly `{term: "reference", target}`;
its semantic type equals the target's, and its dependencies contain exactly
the target id. It has no declaration or owner. Each source reference adds
an `expression` occurrence with its own region and source-map row;
occurrence ordinals follow source position and FR-480 canonical unit order.
A second occurrence does not change the carrier id or body. FR-322 defines
the derived carrier's structural path from the target id. An absent or stale
target refuses `missing_declaration`/`missing-name` and emits no carrier.

For a protocol declaration that checks, `check` SHALL build the
`protocol`/`protocol_clause` node with the FR-383 `quire.op.protocol.control`
body, selected law and thirteen clause arguments, Boolean semantic type,
source declaration and owner, and a declaration occurrence. Its structural
id follows FR-322; a reference resolves to it. Unsupported constructs and
missing profile selections retain FR-383 and FR-218's refusals.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-092-CON-1 | Only `check` assigns the structural id for a node it constructs; no caller-supplied key or identity preimage is accepted. | Design | Inspection |
| FR-092-CON-2 | The matches that select node forms and build checked type nodes have no catch-all arm (ADR-012 §5.1). | Design | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-092-AC-1 | `Boolean`, `Integer`, text, `Int[0,9]`, `Option<Int[0,9]>`, `Sequence<Int[0,9]>`, bounded sequence and bounded text produce the type-node tags, semantic forms, semantic types and bodies in the Type nodes table, each with a valid FR-322 structural id. | Test (TC-413) |
| FR-092-AC-2 | Two units of one package using `Int[0,9]` resolve the same anonymous type node. A declared `Point` in either unit carries that unit's source owner and declaration region; its id has the package name and `Point` declaration path. Two compiles of the same package give equal ids. | Test (TC-413) |
| FR-092-AC-3 | An alias `type Digit = Int[0,9]` adds no checked node; a parameter of `Digit` resolves to the `Int[0,9]` type node. | Test (TC-413) |
| FR-092-AC-4 | Functions with two parameters, no parameters, a `let` binder and an integer literal construct the parameter, function and literal nodes and references specified above. Their ids have FR-322 structural components and no preimage bytes or digest. | Test (TC-414) |
| FR-092-AC-5 | A function with an unread second parameter still lists both parameter references in order; a second function in the same package may reference the same parameter node where its structural id is equal. | Test (TC-414) |
| FR-092-AC-6 | A function body contains no inline application term. The `a and b` expression is a separate application node, and both it and the function have FR-322 structural ids. | Test (TC-414) |
| FR-092-AC-7 | On a 512 KiB stack, 100,000 nested `Option`s check under limits raised to fit, with the same structural type id as on an 8 MiB stack. At default limits a 30-record optional-field chain checks on a 2 MiB stack. Two otherwise equal recursive declarations named `f` and `g` have distinct declared ids and do not collide solely because their conditional bodies are equal. | Test (TC-413) |
| FR-092-AC-8 | A source enum and member carry their QSpec FR-322 structural ids, the declaration path and source owner; neither is assigned a nominal preimage key. | Test (TC-413) |
| FR-092-AC-9 | Rational and decimal bounded types, a tuple and two records differing only by optional-field syntax produce the distinct type-node shapes above. `rational(1,2)` and `rational(2,4)` lower to the same reduced rational value. All nodes have structural ids. | Test (TC-413) |
| FR-092-AC-10 | A function with `decreases(x)` binds the measure to a reference to its parameter node and retains parameter order. | Test (TC-414) |
| FR-092-AC-11 | Recursive function and record groups (`f`, `List`, `Tree`, `ping` and `pong`) retain their references, semantic-type links and QSpec FR-322 group membership; reversing `ping` and `pong` declarations leaves their declared structural ids unchanged. Repeated references to one call use one node id where their package, path and ordinal are equal. | Test (TC-413) |
| FR-092-AC-12 | The id of declared `Point` derives from its package, declaration path and producer ordinal, independent of any caller handle; no checked node stores a caller handle as its id. | Test (TC-413) |
| FR-092-AC-13 | A predicate `Positive` and pure function `Positive` have distinct semantic forms. A source enum `Status` and member `READY` have their declared node tags, semantic forms, declaration paths and source owners. | Test (TC-481) |
| FR-092-AC-14 | Parameters with `Int` bounds through unsigned-64 and signed-128 extremes construct valid bounded-domain type nodes with exact integer bounds, without magnitude-based identity refusal; their ids satisfy FR-322's structural ordinal bound. | Test (TC-910) |
| FR-092-AC-15 | Withdrawn: QSpec FR-322 replaced RFC 8785 preimage-counter spelling with a bounded structural `node_id.ordinal`; no counter-encoding key oracle remains. | Inspection |
| FR-092-AC-16 | A unit that declares `protocol Fill` and holds `protocol Fill` as an expression emits one `protocol`/`protocol_reference` node whose body is exactly `{term: "reference", target: Fill's node_id}`, whose `semantic_type` equals `Fill`'s, and whose `dependencies` are exactly that id. It has one `expression` occurrence at the reference's region (ordinal 0), its own source-map row, and no `declaration`, `owner` or recursion group. The graph has one node collection and no protocol-reference collection. The IR V2 reader admits it as `ProtocolForm::ProtocolReference`. | Test (TC-1021) |
| FR-092-AC-17 | Repeating a reference to one protocol adds an occurrence and source-map row without changing the carrier id, body or dependencies. Changing its bound target changes the target operand in its FR-322 structural path and gives a distinct id. Removing the second occurrence leaves the carrier id unchanged. | Test (TC-1021) |
| FR-092-AC-18 | A unit that declares `protocol Fill` made only of FR-383's emitted constructs (static and replicated roles, channels and queues, `sequence`/`branch`/`case` controls, binders, captures, an `each` guard with no declaration read, a finish with no declaration read, and scheduling) emits one `protocol`/`protocol_clause` node. Its body is a `quire.op.protocol.control` application with the selected `protocol_profile` law, the `protocol` `profile_operator` member and FR-383's thirteen clause arguments; it has Boolean semantic type and one `declaration` occurrence at the declaration's region. The IR V2 reader admits it, and a reference to `Fill` resolves to its structural id. If no profile is selected, QSpec FR-322-AC-22 admits no `protocol_control` application: QSL emits no protocol node and no refusal for the declaration, while a reference to it refuses `missing_declaration`/`missing-selection` (FR-218-AC-8). An FR-383 construct not yet emitted refuses `unsupported_construct`/`not-yet-implemented` at that construct, names it and the ticket that writes it, and writes no clause node. | Test (TC-1021) |
| FR-092-AC-19 | Emitting a unit that declares `enum Status { READY, DONE }` and reads `Status::READY` gives one `SelectedEnumDeclarationRow` and two `SelectedEnumMemberRow`s. Each row's structural id names an emitted `scalar_type`/`enum` or `value`/`enum_value` node; its source authority and identity equal that node's source owner; its qualified declaration is `["Status"]`; its key is the node's key; each member row names the declaration's emitted id and its own case. A unit with a model variant type and no source enum has no source enum row. | Test (TC-1021) |
| FR-092-AC-20 | Emitting a unit that declares `union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }` and constructs `Shape::Rect(2, 3)` gives one `SelectedSourceUnionRow`. Its structural id names the emitted `composite_type`/`union` node; its source authority and identity equal that node's source owner; its qualified declaration is `["Shape"]`; its type handle is the checked declaration's key and its settled key is the key of the node it names. Its members are `Empty`, `Circle` and `Rect` in declaration order, each the sealed `UnionMember` the check minted, with payload types `[]`, `[Integer]` and `[Integer, Integer]` and payload node ids equal to the references in the union node's member bindings. For a recursive union the payload node of an optional self-reference is the emitted `option` node naming the union node. IR's `with_source_union_rows` accepts the row against the package read back and `project_exact_type` of the union node returns `Composite` of the type handle, while the same node with no row refuses `UnavailableIdentityWitness` and a row that misorders its members or drops a payload node refuses `MismatchedIdentityWitness`. A unit with no source union, and a unit with a model variant type, has no union row, and every emitted union node has exactly one row. | Test (TC-1021) |
| FR-092-AC-21 | Emitting a unit that declares `record Point { x: Integer; y: Integer; }` and `record Tagged { p: Point; note: Boolean?; maybe: Option<Integer>; }` gives one `SelectedSourceRecordRow` per record. `Tagged`'s row names the emitted `composite_type`/`record` node; its source authority and identity equal that node's source owner; its qualified declaration is `["Tagged"]`; its record key is the checked declaration's key. Its members are `p`, `note` and `maybe` in declaration order with equal declared and binding names, presence `Required`, `Optional` and `Required`, field types `Composite(Point)`, `Boolean` and `Option<Integer>`, and type node ids equal to the targets of the node's bindings; `p`'s type node is `Point`'s row node. For a recursive record each member type node that reaches the record shares the record node's recursion group. A binding list whose count disagrees with the declaration, a type node that is not emitted, and an emitted source record node that no declared record joins to are writer faults; a binding whose name differs from the declared name keeps both names. A unit with no source record, a tuple or union unit, and a unit whose records are model-owned has no source record row, and every emitted source record node has exactly one row. | Test (TC-1021) |
| FR-092-AC-22 | Emitting a unit that declares `tuple Pair(Integer, Boolean);` and `tuple Triple(Pair, Option<Integer>, Boolean);` gives one `SelectedSourceTupleRow` per tuple. `Triple`'s row names the emitted `composite_type`/`tuple` node; its source authority and identity equal that node's source owner; its qualified declaration is `["Triple"]`; its type handle is the checked declaration's key. Its positions have ordinals 0, 1 and 2 with checked types `Composite(Pair)`, `Option<Integer>` and `Boolean`, and type node ids equal to the node's references in order; position 0's type node is `Pair`'s row node. IR's `with_source_tuple_rows` accepts the rows against the package read back and `project_exact_type` of the tuple node returns `Composite` of the type handle with the complete view, while the same node with no row refuses `UnavailableIdentityWitness` and a row that permutes, drops or adds a position, renumbers an ordinal or re-points a type node refuses `MismatchedIdentityWitness`. A position list that disagrees with the declaration in count, a type node that is not emitted, and an emitted source tuple node that no declared tuple joins to are writer faults; a position the writer re-pointed is named by its final type node. A unit with no source tuple has no tuple row, and every emitted source tuple node has exactly one row. | Test (TC-1021) |
| FR-092-AC-23 | Emitting a unit that declares `enum Status { READY, DONE }` and `ordered enum Level { LOW, MID, HIGH }` gives one `SelectedSourceEnumShapeRow` per enum. `Status`'s shape is unordered with its variants in case-name byte order `DONE`, `READY`; `Level`'s is ordered with `LOW`, `MID`, `HIGH`. Each row's shape equals the admitted binding's shape, each case's variant is the shape's variant at that rank, the declaration id equals the declaration row's id and each case's node id equals the matching member row's id. A shape with a variant reordered or removed, or with the other ordered flag, is not equal to the row's shape. A case whose member node is not emitted is a writer fault. `AdmittedEnum` exposes the same shape and each case's variant. Collection types have no source row. | Test (TC-1021) |
| FR-092-AC-24 | Emitting a unit that declares `record R { xs: Sequence<Integer>[0, 3]; s: Set<Boolean>; }` gives one `SelectedSourceCollectionRow` per collection type node. The bounded sequence's row names its emitted `collection_bounds` node, typed by its emitted `sequence` composite node, which references the `integer` element node the row names, with the checked `Sequence<Integer>[0, 3]`; the set's row has equal structural and base nodes, a `set` composite and a `boolean` element. `R`'s source record row binds each field to its collection row's node. A value type that is not a collection, a base that does not reference exactly the row's element and an unemitted node are writer faults; a re-pointed element or a bound typed by another node than its collection is followed or refused from the final nodes. A unit with no collection has none. | Test (TC-1021) |
| FR-092-AC-25 | With `dimension L; dimension N = L / L; unit m : L = rational(1, 1); unit one : N = rational(1, 1);`, checked and replay-admitted dimension nodes remain distinct: `L` has `base=true` and empty base terms, while `N` has `base=false` and empty derived terms. Their checked payloads and v2 preimages preserve those values; replay reports `L.is_base()` true and `N.is_base()` false. The units reference their respective dimensions and only `one` is dimensionless. | Test (TC-483) |

## Dependencies

- QSpec FR-322, `Structural node identity` and the checked-package V2 reader
  contract, at quire-specification main 686e49e8. The authoritative file is
  read in place; it is not copied into this repository.
- FR-091, FR-093, FR-094, FR-218, FR-258, FR-383 and ADR-011/012 for
  checked graph construction, source owners, type and expression lowering,
  protocol emission, resource limits and exhaustive dispatch.
