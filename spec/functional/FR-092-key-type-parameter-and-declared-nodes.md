---
id: FR-092
title: "Key type, parameter, value and declared function nodes with the structural-node preimage"
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
# FR-092: Key type, parameter, value and declared function nodes with the structural-node preimage

## Description

`check` mints every checked node id (ADR-011 E3, FB-13; ADR-013 O-04). QSpec
FR-322 publishes preimages for two kinds of node: the nominal enum, enum
member, dimension and unit nodes, and every node whose body contains an
application (`quire.application-node/v1`). QSpec FR-440 additionally selects
that application preimage for a `value`/`union_value` node even when its
aggregate body contains no application. The `Value` family also emits type
nodes, literal and aggregate value nodes, parameter nodes and function nodes
whose bodies hold no application. Apart from the `union_value` exception,
this requirement defines QSL's key for those nodes: the
`quire.structural-node/v1` preimage. It also fixes the node
shape of a type, a parameter and a function, which that key hashes.

ADR-013 O-04 keys these nodes by this QSL-proposed preimage, and QSL
conforms to QSpec's arm once QSpec publishes one (QC-18, QC-24). It follows the ADR-013 OQ-G ruling
(ADR-011 M-6a OQ-7): a declared node carries its owner, and a builtin or
anonymous type node carries none and shares one id across packages. QSpec
references (`ix://agent-ix/quire-specification`, cited by reference, never
copied): FR-322, `proposals/checked-package-v2/node-identity-preimage.schema.json`,
`schema.json`, `operation-catalog.json` and the positive fixtures.

## Inputs

- A checked node's `node_tag`, `semantic_form`, semantic type node, optional
  declaration and its FR-322 body, as the check stage builds them (this
  requirement and [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)).
- For a declared node, the declaring unit's `SourceOwner{authority, identity}`
  (FR-091, the required E3 input).
- The checked graph's key-naming edges, from which `check` derives each
  node's recursion group (Recursion groups).

## Outputs

- The node's RFC 8785 preimage bytes and its `quire.checked-semantic-node/v1`
  key: SHA-256 of exactly those bytes.
- Or a typed refusal and no key.

## Behavior

### Which preimage keys a node

`check` SHALL key each checked node by exactly one preimage:

1. an enum declaration, enum member, dimension or unit node by its QSpec
   nominal preimage (`quire.enum-declaration-node/v1`,
   `quire.enum-member-node/v1`, `quire.dimension-node/v1`,
   `quire.unit-node/v1`);
2. any `value`/`union_value` node, and any other node whose body contains an
   `application` term, by QSpec's `quire.application-node/v1` preimage,
   exactly as FR-322 and FR-440 define it;
3. every other node by the `quire.structural-node/v1` preimage below.

The `union_value` selection is independent of payload arity and of whether
the body contains an `application` term. Its aggregate body is the body
specified by [FR-319](FR-319-key-union-member-and-case-nodes.md) and
[FR-320](FR-320-lower-and-emit-union-and-case-nodes.md).
The exception applies only to `value`/`union_value`: a `record_value` or
`tuple_value` whose body contains no application still selects rule 3.

The key is SHA-256 of the RFC 8785 bytes of the preimage, produced by the one
JCS implementation ADR-013 §2 names (`quire-canonical`).

### The `quire.structural-node/v1` preimage

For rule 3 above, `check` SHALL build the closed structural preimage defined
by QSpec FR-322, “Structural node preimage,” and compute its key using the
canonical encoder ADR-013 §2 names. The `owner` value on a declared source
node comes from its declaring unit; a model-owned node uses the owner supplied
under FR-094. QSL does not define a second preimage shape or literal spelling.
The type/form-to-node mapping below and the group ordering in this
requirement determine the inputs QSL supplies to that QSpec preimage.

QSpec FR-322, “Scalar literal values” and “Integers in a node-key preimage,”
own literal canonical forms and counter encodings. QSL's check stage SHALL
build those values from its checked types and literals, and SHALL NOT reject
a key input solely for integer magnitude.


Building a type node and lowering a checked expression each walk a tree of
any depth over an explicit heap stack, bounded by the check stage's node,
input-byte and work limits (`CheckingLimits`;
[FR-258](FR-258-check-and-lower-expressions-at-any-depth.md), ADR-030
D-4.3). A type node's walk uses stack that does not grow with the nesting of
the options, collections and declared composites it follows.

A declared node's qualified name, a binding name and a `semantic_form` are
never empty: S2 builds identifiers from non-empty tokens. A preimage that
would hold an empty one is an internal fault (ADR-013 T-4), and yields no
key.

### Recursion groups

A node **names** another node when the other node's key is a member of its
preimage: a body `reference` target, its `semantic_type`, an application's
`result_type`, a literal's `type`, or an operation member's `declaration`.
A node's **recursion group** is its strongly connected component in the graph
of these edges when that component has more than one node or a node that
names itself. Recursive functions form groups (a function node, its body's
conditional and its recursive call). Source-level recursive record forms
follow QSpec FR-143; this requirement adds no record-cycle admission rule.

`check` SHALL key nodes in dependency order. It keys a node outside every
group after every node that node names. It keys a group after every node
outside the group that a member names. The components of the graph form a
directed acyclic graph, so this order exists. When `check` keys a group, it
already knows every key that the members name outside the group.

#### An in-group node's preimage

`check` SHALL form each in-group preimage under QSpec FR-322's structural or
application node-key rules. The group's `size`, member ordinal, and
`group_reference` substitutions are determined by the QSL ordering below;
QSL's package emitter writes group members in that same ordinal order so the
reader can recover the order from graph position. In particular, an in-group
structural node may name a member through `semantic_type`, as the
`collection_bounds` node does for `Sequence<Tree>` in G9. QSpec FR-322 owns
the corresponding reader checks for retained keys and group labels; QSL's
producer MUST emit keys current under that contract.

#### The group order

`check` SHALL order a group's members by their content, as follows.

1. A member's **full shape** is its preimage with `recursion` `null` and each
   position that names a member of the group written
   `{term: "group_reference"}`, with no ordinal. Its **anonymous shape** is
   its full shape with `declaration` `null` and no `owner` member.
2. A member's **targets** are the members named at those positions, in the
   order the positions occur in the RFC 8785 bytes of its full shape. A
   member named twice is listed twice.
3. A **refinement pass** over one kind of shape computes a signature for
   each member. `h0(m)` is SHA-256 of the RFC 8785 bytes of `m`'s shape.
   `h(r+1)(m)` is SHA-256 of the RFC 8785 bytes of
   `{"shape": h0(m), "targets": [hr(t) for each target t of m]}`. Each digest
   is written as a lowercase hex string. The pass stops at the first round
   `r` of 1 or more in which the number of distinct `hr` values equals the
   number of distinct `h(r-1)` values, and `hr(m)` is `m`'s signature. Each
   round before the last adds at least one distinct value, so a pass stops
   within `n` rounds for a group of `n` members.
4. `check` runs one pass over the anonymous shapes and one over the full
   shapes, and orders the members by anonymous signature, then by full
   signature, each compared as a lowercase hex string.
5. Members with equal full signatures form one **class**. They have equal
   preimages, so they are one node (FR-093's content addressing), and
   `check` keeps one node for the class. A class's `ordinal` is its rank in
   the order, from 0, and `size` is the number of classes.

A member's **group-local preimage** is its preimage built with these
ordinals, without the `group` member. The **group digest** is SHA-256 of the
RFC 8785 bytes of the array whose `k`-th element is the lowercase hex SHA-256
of the group-local preimage of ordinal `k`. A structural in-group node's key
is SHA-256 of its group-local preimage with `recursion.group` set to the
group digest. An application in-group node's key is SHA-256 of its
group-local preimage, which is its FR-322 preimage.

The order is well defined and not circular. The shapes name only keys
outside the group, which dependency order has already fixed, and
placeholders. The signatures are digests of shapes and of other signatures.
The ordinals come from the order, the preimages from the ordinals and the
keys from the preimages. No step reads the key of a member of the group, so
the order does not depend on node keys.

FR-322 reads a member's `ordinal` from the member's position among its
group's nodes in the v2 graph's node order, which the package's writer
chooses. QSL's v2 emission writes each group's members in ordinal order
([FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md), Who
builds the lowering), so a reader that derives the ordinal from graph order,
as IR-242's does, recomputes the keys `check` minted. A reader reads a node's
`owner` from the node's wire `owner` member, which QSL's v2 emission writes
on every node whose preimage carries one (FR-093, QSpec FR-322 `owner`), so
it recomputes an in-group key and its group digest from the package alone.

A pass computes at most `n` signatures per round for `n` rounds, so at most
`n²` signatures for a group of `n` members, and a group holds no more nodes
than the check stage's node limit (`CheckingLimits`) admits.

The order is independent of declaration order, source regions and the order
in which `check` visits the declarations: no shape holds any of them. It
depends on a member's declared name and owner only through the full pass,
which breaks ties that the anonymous pass leaves. Names belong in the order
for three reasons:

- A declared member's key already hashes its `declaration` and `owner`, so
  renaming the member changes its key whatever the order is.
- Without names, the members of a mutually recursive pair whose bodies differ
  only in which function they call tie (G10 to G15). Content would then give
  no order for them, and any tie-break would take the declaration order.
- Running the anonymous pass first means that a group whose anonymous pass
  separates every member gets ordinals that do not depend on names. Two such
  groups that differ only in declared names then always get equal ordinals
  and collide, whatever the names are.

#### Groups that collide

The application-node preimage has no member that names the group: an
in-group application node's preimage holds its body, its types, `size` and
its own and its targets' ordinals. Two application nodes of different groups
with the same body shape, the same `size` and the same ordinals therefore
have equal preimages. Groups that differ only in declared names or owners
always give their in-group application nodes equal preimages: in
FR-092-AC-7, the preimages of the conditionals of `f` and of `g` both hash
to G5. Groups that differ in other content can also coincide, when their
application nodes' shapes and ordinals happen to match. A structural
in-group node carries its group digest, and every group holds a declared
record or function whose `declaration` and `owner` enter that digest, so a
structural in-group node's key differs from every other group's members'.

`check` SHALL refuse a package in which two members of different recursion
groups have equal keys, with `unknown_required_feature`/
`unsupported-feature` naming the region of every declared member of both
groups, and yield no key for any member of either group (FR-092-OQ-1). Two
nodes outside any recursion group with equal preimages are one node, keyed
once (FR-093's content addressing).

### Type nodes

`check` SHALL build the type node of each checked `ValueType` of the `Value`
family as follows. The literal types inside a type node's body are the
builtin `Integer` scalar node (T2) for an integer and the builtin text scalar
node (T3) for a text value.

| Checked type | `node_tag` / `semantic_form` | `semantic_type` | Body |
|---|---|---|---|
| `Boolean`, `Integer` | `scalar_type` / `boolean`, `integer` | itself | `aggregate{[]}` |
| base of `Rational[..]`, `Decimal[..]`, `Text[..]` | `scalar_type` / `rational`, `decimal`, `text` | itself | `aggregate{[]}` |
| `Int[lo, hi]` | `bounded_domain` / `integer_range` | the `Integer` node | bindings `min` = `lo`, `max` = `hi` |
| `Rational[n1, n2; d1, d2]` | `bounded_domain` / `rational_range` | the `rational` scalar node | bindings `numerator_min`, `numerator_max`, `denominator_min`, `denominator_max` |
| `Decimal[lo, hi; s1, s2; mode]` | `bounded_domain` / `decimal_range` | the `decimal` scalar node | bindings `coefficient_min`, `coefficient_max`, `scale_min`, `scale_max`, and `rounding` as text |
| `Text[min, max; profile]` | `bounded_domain` / `text_bounds` | the `text` scalar node | bindings `min`, `max`, and `text_profile` as text |
| `Float32[mode]`, `Float64[mode]` | `bounded_domain` / `float_rounding` over the `scalar_type` / `float32`, `float64` node | the float scalar node | binding `rounding` as text; a type written without a mode has `exact` |
| `Option<T>` | `composite_type` / `option` | itself | `aggregate{[reference(T)]}` |
| `K<T>` for a collection kind `K` | `composite_type` / `sequence`, `set`, `bag`, `ordered_set` | itself | `aggregate{[reference(T)]}` |
| `K<T>[min, max]` | `bounded_domain` / `collection_bounds` | the `K<T>` node | bindings `min`, `max` |
| a declared `record R { f: T; ... }` | `composite_type` / `record`, with `declaration` and `owner` | itself | one binding per field in declaration order, `f` = `reference(T)`; an optional field `f: T?` is `f` = `aggregate{binding{name: "optional", value: reference(Option<T>)}}`, a Group, so `f: T?` and `f: Option<T>` differ and no binding holds a binding |
| a declared `tuple Q(T0, ...)` | `composite_type` / `tuple`, with `declaration` and `owner` | itself | `aggregate{[reference(T0), ...]}` in position order |

Each binding is `{term: "binding", name, value}` and each body with bindings
is an `aggregate` of them in the order listed. A bound's value is a
`literal`. The spellings follow QSpec's positive operation fixture
(`fixtures/positive-operation-identities.json`) wherever it spells the same
form.

The type of a value `K<T>[min, max]`, `Int[lo, hi]` and the other bounded
forms is the `bounded_domain` node, not its base. Builtin and anonymous type
nodes carry no `declaration` and no `owner` (ADR-013 OQ-G), so equal
structure gives one node id in every package. A declared record or tuple
carries its `declaration` and its unit's `owner`, so the same declaration
under two owners gives two ids.

The key that a caller passes to `CompositeDeclaration::new`, which
`ValueType::Composite` carries, is a handle local to one check. It selects
one declaration in the check stage's `TypeEnvironment` and orders type
admission's refusals. Only `check` mints a node id (ADR-011 FB-13): `check`
SHALL key a declared record or tuple by its `quire.structural-node/v1`
preimage alone, and SHALL give its checked type node (`CheckedTypeNode`) that
key as its id. Every reference to the declaration resolves to that key
(FR-088-AC-7). Preimages, checked-graph nodes, checked type nodes, model
correspondence entries and v2 wire members hold node keys only, so two type
environments that give one declaration different handles give it one node
id.

A type alias introduces no type node. A type form that names an alias lowers
to the node of the alias's resolved type (FR-091 resolves the alias to that
type).

An enum type is its QSpec nominal node (rule 1). A quantity type, a
`Reference<T>` and a `Population<T>[N]` type node are keyed as
[FR-094](FR-094-key-model-owned-reference-population-and-quantity-nodes.md)
fixes: a declared unit's quantity type is the unit's nominal node, and the
others are structural nodes.

### Parameter nodes

A parameter node names a value that an enclosing binder binds: a function
parameter, a `let` name, a query, `count` or `sum` binder, or a `fold` or
`reduce` accumulator or element binder. `check` SHALL build one node per
binder as follows:

- `node_tag` `value`, `semantic_form` `parameter`;
- `semantic_type`: the node of the binder's checked type;
- `declaration`: `null`, and no `owner`;
- body: `aggregate{[binding "name" = the name as a text literal typed at the
  text scalar node (T3), binding "level" = the binder's level as an integer
  literal typed at the `Integer` node (T2)]}`.

A binder's **level** is its position among the binders in scope where it is
bound. A function's parameters have levels `0` to `n - 1` in parameter order.
A binder inside a function body or measure has level `n` plus the number of
binders whose scope encloses it. A `let` binder's scope is its body, a query,
`count` or `sum` binder's scope is its body or predicate, and a `fold` or
`reduce` binds its accumulator at level `k` and its element at `k + 1` over
the step. Two sibling binders therefore have the same level.

A `reference` to a parameter node of level `k` denotes the value bound by the
one enclosing binder at level `k`. Its name equals that binder's name, and
its type equals the binder's type. A parameter node carries no owner and no
function identity, so its key does not depend on the function that declares
it, and the function's key can depend on it without a cycle.

### Function nodes

`check` SHALL build the node of a function declaration as follows:

- `node_tag` `function`; `semantic_form` `recursive_function` when the node is
  in a recursion group, else `predicate` for a declaration of kind
  `Predicate` ([FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md)),
  else `pure_function`. A predicate has no measure, so `check` refuses any
  recursion group that holds one (FR-146 `missing-measure`), and a
  predicate node is never `recursive_function`;
- `semantic_type`: the node of the declared result type;
- `declaration`: `{qualified_name}`, the declared name's `::`-separated
  segments (ADR-013 O-11), and `owner`: the unit's `SourceOwner`;
- body: an `aggregate` of bindings, in this order:
  - `parameters` = `aggregate{[reference(p0), ..., reference(pn-1)]}`, the
    parameter nodes in parameter order, so the arity and the order include
    parameters the body never reads;
  - `body` = `reference` to the node of the checked body's root expression
    ([FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md));
  - `decreases` = `reference` to the node of the measure, when one is written.

A function node's body holds references only, never an `application` term. It
is therefore keyed by `quire.structural-node/v1`, and its preimage carries its
owner. The root expression and every application inside the body are their
own `expression` nodes, keyed by `quire.application-node/v1`. They carry no
`declaration`, so their preimage carries no owner, and their keys equal
QSpec's application-node vectors for the same node.

A clause function that `check` synthesizes from a domain package clause has
this shape with a `ModelOwner`, a `null` `declaration` and a `clause` binding
([FR-094](FR-094-key-model-owned-reference-population-and-quantity-nodes.md)).

### Protocol reference nodes

`check` SHALL build the node of a protocol reference expression
(FR-308, FR-218) as follows, and S4 SHALL emit it as the one carrier QSpec
FR-322 specifies (QSpec FR-322-AC-66):

- `node_tag` `protocol`, `semantic_form` `protocol_reference`;
- body: exactly `{term: "reference", target}`, where `target` is the typed
  node key of the local `protocol` node the reference bound (FR-218), and no
  other member;
- `semantic_type`: the target node's `semantic_type`, as the target node
  carries it;
- `dependencies`: exactly the target key, and nothing else;
- occurrences: one or more, all of role `expression`, one per source
  reference to the target, each at that reference's source region, ordered by
  source position, the units of a several-unit package taken in FR-480's
  canonical unit order, with ordinals 0 to n-1 and each with its own source-map row
  keyed (node id, `expression`, ordinal);
- no `declaration`, `owner`, `nominal_identity_preimage` or recursion group.

A reference emits no separate collection, package selector or dependency
reference, and a target that is not in the package's own graph is never
satisfied by a selected dependency. The node is keyed by rule 3 (the
`quire.structural-node/v1` preimage), so its body, and therefore its target
key, enters the key. Two references to different targets, or one reference
whose target protocol was changed and re-keyed, have different keys; a
reference's key changes exactly when its bound target key changes. A second
reference to the same target adds an `expression` occurrence to the existing
node and leaves the node's key, body and `dependencies` unchanged. The
occurrences and their source-map rows enter neither the node key nor the
package identity: removing the second occurrence and its source-map row from
the emitted package leaves the carrier key and `package_id` unchanged. The
emitter SHALL NOT write the node when the bound target key is not the key of
a node it writes in the same graph (FR-218 stale target); that refusal is
`missing_declaration`/`missing-name`, the cause the IR reader gives an absent
target.

#### The protocol node a reference targets

`check` SHALL build one `protocol`/`protocol_clause` node for each protocol
declaration that checks (FR-218, FR-113), and S4 SHALL emit it, together with
the role, channel, queue, obligation, control and compensation nodes it
lists, as FR-383 specifies. The IR reader admits any local `protocol`-tagged
node as a reference target; QSL emits the clause node:

- `node_tag` `protocol`, `semantic_form` `protocol_clause` (the target form
  QSpec FR-322-AC-66 and TC-931 name);
- body: the `quire.op.protocol.control` application carrying the selected
  `protocol_profile` law and the `protocol` `profile_operator` member, with
  the clause arguments FR-383 lists, as QSpec FR-322-AC-22 and FR-464
  require;
- `semantic_type`: the node of the Boolean scalar type, the semantic type
  every clause node carries (TC-931's fixture);
- key: `quire.application-node/v1`, by rule 2, because the body holds an
  application;
- `declaration` and `owner` as FR-322 gives a declared clause node, and one
  `declaration` occurrence at the protocol declaration's region.

The reference binds this node's key (FR-218). A change to any member of the
protocol changes the clause node's key (FR-383), and a reference bound to
the old key is stale.

### Golden vectors

Each vector below is the exact preimage bytes and the key `check` SHALL mint
for the named node. Owners are `SourceOwner{authority: "a", identity: "u"}`
and, for D2, `{authority: "a", identity: "w"}`. Every function below is
declared under owner (`a`, `u`). A vector that names another vector's key
uses the digest that vector gives. E1 to E3 are `quire.application-node/v1`
keys of the expression nodes [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
builds; they are listed here because F2 and E2 depend on them.

| Vector | Node | Key |
|---|---|---|
| T1 | Boolean | the SHA-256 of its preimage |
| T2 | Integer | the SHA-256 of its preimage |
| T3 | Text scalar | the SHA-256 of its preimage |
| T4 | Int[0, 9] | the SHA-256 of its preimage |
| T5 | Option<Int[0, 9]> | the SHA-256 of its preimage |
| T6 | Sequence<Int[0, 9]> | the SHA-256 of its preimage |
| T7 | Sequence<Int[0, 9]>[0, 5] | the SHA-256 of its preimage |
| T8 | Text[0, 64; nfc] | the SHA-256 of its preimage |
| T9 | Rational scalar | the SHA-256 of its preimage |
| T10 | Rational[-9, 9; 1, 9] | the SHA-256 of its preimage |
| T11 | Decimal scalar | the SHA-256 of its preimage |
| T12 | Decimal[-100000, 100000; 2, 2; nearest-even] | the SHA-256 of its preimage |
| T13 | Int[0, 18446744073709551615] | the SHA-256 of its preimage |
| T14 | Int[0, 9223372036854775808] | the SHA-256 of its preimage |
| T15 | Int[-170141183460469231731687303715884105728, 170141183460469231731687303715884105727] | the SHA-256 of its preimage |
| T16 | Int[0, 18446744073709551616] | the SHA-256 of its preimage |
| D1 | record Point, owner (a, u) | the SHA-256 of its preimage |
| D2 | record Point, owner (a, w) | the SHA-256 of its preimage |
| D3 | record Opt { a: Int[0, 9]; b: Int[0, 9]?; }, owner (a, u) | the SHA-256 of its preimage |
| D4 | record Opt { a: Int[0, 9]; b: Option<Int[0, 9]>; }, owner (a, u) | the SHA-256 of its preimage |
| D5 | tuple Pair(Int[0, 9], Int[0, 9]), owner (a, u) | the SHA-256 of its preimage |
| P1 | parameter a: Boolean, level 0 | the SHA-256 of its preimage |
| P2 | parameter b: Boolean, level 1 | the SHA-256 of its preimage |
| P3 | let binder y: Boolean, level 1 | the SHA-256 of its preimage |
| P4 | parameter x: Int[0, 9], level 0 | the SHA-256 of its preimage |
| L1 | literal true | the SHA-256 of its preimage |
| L2 | literal 7 | the SHA-256 of its preimage |
| L3 | literal rational(1, 2) of type Rational[-9, 9; 1, 9] | the SHA-256 of its preimage |
| L7 | literal `-170141183460469231731687303715884105728` (`i128::MIN`, folded; FR-091) | the SHA-256 of its preimage |
| F1 | function f() -> Boolean { true }, owner (a, u) | the SHA-256 of its preimage |
| E1 | a and b | the SHA-256 of its preimage |
| F2 | function both(a: Boolean, b: Boolean): Boolean { a and b }, owner (a, u) | the SHA-256 of its preimage |
| E2 | both(a, true) | the SHA-256 of its preimage |
| E3 | let y = a in y | the SHA-256 of its preimage |
| F3 | function m(x: Int[0, 9]): Boolean decreases(x) { true }, owner (a, u) | the SHA-256 of its preimage |
| L5 | literal `0` | the SHA-256 of its preimage |
| L6 | literal `1` | the SHA-256 of its preimage |
| E11 | `x > 0` | the SHA-256 of its preimage |
| E12 | `x - 1` | the SHA-256 of its preimage |
| E13 | `x - 1` narrowed into `Int[0, 9]` | the SHA-256 of its preimage |
| G1 | an `option` node over itself, a one-member group, ordinal 0 | the SHA-256 of its preimage |
| G2 | `record List { next: List?; }`, ordinal 1 | the SHA-256 of its preimage |
| G3 | `Option<List>`, ordinal 0 | the SHA-256 of its preimage |
| G4 | function `f`, ordinal 2 | the SHA-256 of its preimage |
| G5 | `if x > 0 then f(x - 1) else true`, ordinal 0 | the SHA-256 of its preimage |
| G6 | `f(x - 1)`, ordinal 1 | the SHA-256 of its preimage |
| G7 | `record Tree { kids: Sequence<Tree>[0, 3]; }`, ordinal 0 | the SHA-256 of its preimage |
| G8 | `Sequence<Tree>`, ordinal 1 | the SHA-256 of its preimage |
| G9 | `Sequence<Tree>[0, 3]`, ordinal 2 | the SHA-256 of its preimage |
| G10 | function `ping`, ordinal 4 | the SHA-256 of its preimage |
| G11 | function `pong`, ordinal 5 | the SHA-256 of its preimage |
| G12 | `if x > 0 then pong(x - 1) else true`, in `ping`, ordinal 1 | the SHA-256 of its preimage |
| G13 | `if x > 0 then ping(x - 1) else true`, in `pong`, ordinal 0 | the SHA-256 of its preimage |
| G14 | `ping(x - 1)`, in `pong`, ordinal 2 | the SHA-256 of its preimage |
| G15 | `pong(x - 1)`, in `ping`, ordinal 3 | the SHA-256 of its preimage |

**T1**: Boolean

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"boolean","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**T2**: Integer

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"integer","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**T3**: Text scalar

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"text","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**T4**: Int[0, 9]

```json
{"body":{"members":[{"name":"min","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}},{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"9","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"integer_range","semantic_type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**T5**: Option<Int[0, 9]>

```json
{"body":{"members":[{"target":{"digest":"<digest of T4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":null,"semantic_form":"option","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**T6**: Sequence<Int[0, 9]>

```json
{"body":{"members":[{"target":{"digest":"<digest of T4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":null,"semantic_form":"sequence","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**T7**: Sequence<Int[0, 9]>[0, 5]

```json
{"body":{"members":[{"name":"min","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}},{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"5","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"collection_bounds","semantic_type":{"digest":"<digest of T6>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**T8**: Text[0, 64; nfc]

```json
{"body":{"members":[{"name":"min","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}},{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"64","value_kind":"integer"}},{"name":"text_profile","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T3>","domain":"quire.checked-semantic-node/v1"},"value":"nfc","value_kind":"text"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"text_bounds","semantic_type":{"digest":"<digest of T3>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**T9**: Rational scalar

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"rational","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**T10**: Rational[-9, 9; 1, 9]

```json
{"body":{"members":[{"name":"numerator_min","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"-9","value_kind":"integer"}},{"name":"numerator_max","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"9","value_kind":"integer"}},{"name":"denominator_min","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"}},{"name":"denominator_max","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"9","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"rational_range","semantic_type":{"digest":"<digest of T9>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**T11**: Decimal scalar

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"decimal","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**T12**: Decimal[-100000, 100000; 2, 2; nearest-even]

```json
{"body":{"members":[{"name":"coefficient_min","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"-100000","value_kind":"integer"}},{"name":"coefficient_max","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"100000","value_kind":"integer"}},{"name":"scale_min","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"2","value_kind":"integer"}},{"name":"scale_max","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"2","value_kind":"integer"}},{"name":"rounding","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T3>","domain":"quire.checked-semantic-node/v1"},"value":"nearest-even","value_kind":"text"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"decimal_range","semantic_type":{"digest":"<digest of T11>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**T13**: Int[0, 18446744073709551615]

```json
{"body":{"members":[{"name":"min","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}},{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"18446744073709551615","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"integer_range","semantic_type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**T14**: Int[0, 9223372036854775808]

```json
{"body":{"members":[{"name":"min","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}},{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"9223372036854775808","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"integer_range","semantic_type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**T15**: Int[-170141183460469231731687303715884105728, 170141183460469231731687303715884105727]

```json
{"body":{"members":[{"name":"min","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"-170141183460469231731687303715884105728","value_kind":"integer"}},{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"170141183460469231731687303715884105727","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"integer_range","semantic_type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**T16**: Int[0, 18446744073709551616]

```json
{"body":{"members":[{"name":"min","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}},{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"18446744073709551616","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"integer_range","semantic_type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**D1**: record Point, owner (a, u)

```json
{"body":{"members":[{"name":"x","term":"binding","value":{"target":{"digest":"<digest of T4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"y","term":"binding","value":{"target":{"digest":"<digest of T4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["Point"]},"node_tag":"composite_type","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":null,"semantic_form":"record","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**D2**: record Point, owner (a, w)

```json
{"body":{"members":[{"name":"x","term":"binding","value":{"target":{"digest":"<digest of T4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"y","term":"binding","value":{"target":{"digest":"<digest of T4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["Point"]},"node_tag":"composite_type","owner":{"authority":"a","identity":"w","kind":"source"},"recursion":null,"semantic_form":"record","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**D3**: record Opt { a: Int[0, 9]; b: Int[0, 9]?; }, owner (a, u)

```json
{"body":{"members":[{"name":"a","term":"binding","value":{"target":{"digest":"<digest of T4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"b","term":"binding","value":{"members":[{"name":"optional","term":"binding","value":{"target":{"digest":"<digest of T5>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"}}],"term":"aggregate"},"declaration":{"qualified_name":["Opt"]},"node_tag":"composite_type","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":null,"semantic_form":"record","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**D4**: record Opt { a: Int[0, 9]; b: Option<Int[0, 9]>; }, owner (a, u)

```json
{"body":{"members":[{"name":"a","term":"binding","value":{"target":{"digest":"<digest of T4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"b","term":"binding","value":{"target":{"digest":"<digest of T5>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["Opt"]},"node_tag":"composite_type","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":null,"semantic_form":"record","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**D5**: tuple Pair(Int[0, 9], Int[0, 9]), owner (a, u)

```json
{"body":{"members":[{"target":{"digest":"<digest of T4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"<digest of T4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":{"qualified_name":["Pair"]},"node_tag":"composite_type","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":null,"semantic_form":"tuple","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**P1**: parameter a: Boolean, level 0

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T3>","domain":"quire.checked-semantic-node/v1"},"value":"a","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**P2**: parameter b: Boolean, level 1

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T3>","domain":"quire.checked-semantic-node/v1"},"value":"b","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**P3**: let binder y: Boolean, level 1

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T3>","domain":"quire.checked-semantic-node/v1"},"value":"y","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**P4**: parameter x: Int[0, 9], level 0

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T3>","domain":"quire.checked-semantic-node/v1"},"value":"x","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"<digest of T4>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**L1**: literal true

```json
{"body":{"term":"literal","type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"value":true,"value_kind":"boolean"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"literal","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**L2**: literal 7

```json
{"body":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"7","value_kind":"integer"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"literal","semantic_type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**L7**: literal `-170141183460469231731687303715884105728` (`i128::MIN`, folded; FR-091)

```json
{"body":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"-170141183460469231731687303715884105728","value_kind":"integer"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"literal","semantic_type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**L3**: literal rational(1, 2) of type Rational[-9, 9; 1, 9]

```json
{"body":{"term":"literal","type":{"digest":"<digest of T10>","domain":"quire.checked-semantic-node/v1"},"value":"1/2","value_kind":"rational"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"literal","semantic_type":{"digest":"<digest of T10>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**F1**: function f() -> Boolean { true }, owner (a, u)

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[],"term":"aggregate"}},{"name":"body","term":"binding","value":{"target":{"digest":"<digest of L1>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["f"]},"node_tag":"function","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":null,"semantic_form":"pure_function","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**E1**: a and b

```json
{"body":{"arguments":[{"target":{"digest":"<digest of P1>","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"<digest of P2>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.boolean.and","laws":[],"leaves":[],"member":null,"mode":null},"operator":"binary","result_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"binary","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**F2**: function both(a: Boolean, b: Boolean): Boolean { a and b }, owner (a, u)

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"<digest of P1>","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"<digest of P2>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"target":{"digest":"<digest of E1>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["both"]},"node_tag":"function","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":null,"semantic_form":"pure_function","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**E2**: both(a, true)

```json
{"body":{"arguments":[{"target":{"digest":"<digest of F2>","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"<digest of P1>","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"<digest of L1>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.function.call","laws":[],"leaves":[],"member":null,"mode":null},"operator":"call","result_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"call","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**E3**: let y = a in y

```json
{"body":{"arguments":[{"name":"y","term":"binding","value":{"target":{"digest":"<digest of P1>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"target":{"digest":"<digest of P3>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.control.let","laws":[],"leaves":[],"member":null,"mode":null},"operator":"let","result_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"let","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**F3**: function m(x: Int[0, 9]): Boolean decreases(x) { true }, owner (a, u)

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"<digest of P4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"target":{"digest":"<digest of L1>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"decreases","term":"binding","value":{"target":{"digest":"<digest of P4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["m"]},"node_tag":"function","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":null,"semantic_form":"pure_function","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

#### Recursion-group vectors

L5 and E11 to E13 are nodes outside any group that the recursive
functions below name. L6 is the standalone literal `1`, which no recursive
function below names: the `1` in `x - 1` is not a node but a literal term in
the argument list of the `quire.op.integer.sub` application (E12), as every
integer literal operand of `quire.op.integer.add`, `sub`, `mul` and `negate`
is (FR-358, ADR-013 O-09). A literal anywhere else, such as a function body
or an operand of another operator, is a `value`/`literal` node like L6. G1 to G15 are members of recursion groups, and every
declared member is under owner (`a`, `u`):

- G1: a one-member group, a `composite_type`/`option` node whose body
  references itself. No QSL source forms a one-member group: a function
  names its body's root expression, which is never the function, and a
  record that names itself directly (`record R { next: R; }`) is a cycle
  that FR-143 refuses, since it passes no optional field, `Option` or
  minimum-zero collection. TC-413 keys G1
  through the key function directly.
- G2 and G3: `record List { next: List?; }`, a group of two.
- G4 to G6: `function f using v(x: Int[0, 9]): Boolean pure decreases(x) { if x > 0 then f(x - 1) else true }`,
  a self-recursive function, a group of three. `x - 1` is `Integer` (E12),
  and the call narrows it into the parameter's `Int[0, 9]` (E13).
- G7 to G9: `record Tree { kids: Sequence<Tree>[0, 3]; }`, a group of three
  whose `collection_bounds` member names another member at its
  `semantic_type`.
- G10 to G15: `function ping using v(x: Int[0, 9]): Boolean pure decreases(x) { if x > 0 then pong(x - 1) else true }`
  and `function pong`, the same body calling `ping`, a mutually recursive
  pair whose bodies differ only in which function they call, a group of six.

Each group's members in group order and its group digest are below, and the
tests assert both against the lowered graph. The group order separates G4 to
G6, G2 and G3 and G7 to G9 by their content alone. G10 to G15 holds three
pairs of members that differ only in which function they call, ordered by the
names `ping` and `pong`. Only G1's signatures are published, because a unit
test asserts them; the signatures of the other groups are intermediate values
that the group digest binds.

**L5**: literal `0`

```json
{"body":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"literal","semantic_type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**L6**: literal `1`

```json
{"body":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"literal","semantic_type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**E11**: `x > 0`

```json
{"body":{"arguments":[{"target":{"digest":"<digest of P4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"<digest of L5>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.integer.gt","laws":[],"leaves":[],"member":null,"mode":null},"operator":"binary","result_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"binary","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**E12**: `x - 1`

```json
{"body":{"arguments":[{"target":{"digest":"<digest of P4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"}],"operation":{"identity":"quire.op.integer.sub","laws":[],"leaves":[],"member":null,"mode":null},"operator":"binary","result_type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"binary","semantic_type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**E13**: `x - 1` narrowed into `Int[0, 9]`

```json
{"body":{"arguments":[{"target":{"digest":"<digest of E12>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.numeric.narrow","laws":[],"leaves":[],"member":{"declaration":{"digest":"<digest of T4>","domain":"quire.checked-semantic-node/v1"},"kind":"type_argument"},"mode":null},"operator":"convert","result_type":{"digest":"<digest of T4>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"conversion","semantic_type":{"digest":"<digest of T4>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**G1**: an `option` node over itself, a one-member group, ordinal 0

```json
{"body":{"members":[{"ordinal":0,"term":"group_reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":{"group":"<group digest of G1>","ordinal":0,"size":1},"semantic_form":"option","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**G2**: `record List { next: List?; }`, ordinal 1

```json
{"body":{"members":[{"name":"next","term":"binding","value":{"members":[{"name":"optional","term":"binding","value":{"ordinal":0,"term":"group_reference"}}],"term":"aggregate"}}],"term":"aggregate"},"declaration":{"qualified_name":["List"]},"node_tag":"composite_type","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":{"group":"<group digest of G2>","ordinal":1,"size":2},"semantic_form":"record","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**G3**: `Option<List>`, ordinal 0

```json
{"body":{"members":[{"ordinal":1,"term":"group_reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":{"group":"<group digest of G2>","ordinal":0,"size":2},"semantic_form":"option","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**G4**: function `f`, ordinal 2

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"<digest of P4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"ordinal":0,"term":"group_reference"}},{"name":"decreases","term":"binding","value":{"target":{"digest":"<digest of P4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["f"]},"node_tag":"function","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":{"group":"<group digest of G4>","ordinal":2,"size":3},"semantic_form":"recursive_function","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**G5**: `if x > 0 then f(x - 1) else true`, ordinal 0

```json
{"body":{"arguments":[{"target":{"digest":"<digest of E11>","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"ordinal":1,"term":"group_reference"},{"target":{"digest":"<digest of L1>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.control.if","laws":[],"leaves":[],"member":null,"mode":null},"operator":"conditional","result_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":{"ordinal":0,"size":3},"semantic_form":"conditional","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**G6**: `f(x - 1)`, ordinal 1

```json
{"body":{"arguments":[{"ordinal":2,"term":"group_reference"},{"target":{"digest":"<digest of E13>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.function.call","laws":[],"leaves":[],"member":null,"mode":null},"operator":"call","result_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":{"ordinal":1,"size":3},"semantic_form":"call","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**G7**: `record Tree { kids: Sequence<Tree>[0, 3]; }`, ordinal 0

```json
{"body":{"members":[{"name":"kids","term":"binding","value":{"ordinal":2,"term":"group_reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["Tree"]},"node_tag":"composite_type","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":{"group":"<group digest of G7>","ordinal":0,"size":3},"semantic_form":"record","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**G8**: `Sequence<Tree>`, ordinal 1

```json
{"body":{"members":[{"ordinal":0,"term":"group_reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":{"group":"<group digest of G7>","ordinal":1,"size":3},"semantic_form":"sequence","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**G9**: `Sequence<Tree>[0, 3]`, ordinal 2

```json
{"body":{"members":[{"name":"min","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}},{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"<digest of T2>","domain":"quire.checked-semantic-node/v1"},"value":"3","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":{"group":"<group digest of G7>","ordinal":2,"size":3},"semantic_form":"collection_bounds","semantic_type":{"ordinal":1,"term":"group_reference"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**G10**: function `ping`, ordinal 4

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"<digest of P4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"ordinal":1,"term":"group_reference"}},{"name":"decreases","term":"binding","value":{"target":{"digest":"<digest of P4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["ping"]},"node_tag":"function","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":{"group":"<group digest of G10>","ordinal":4,"size":6},"semantic_form":"recursive_function","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**G11**: function `pong`, ordinal 5

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"<digest of P4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"ordinal":0,"term":"group_reference"}},{"name":"decreases","term":"binding","value":{"target":{"digest":"<digest of P4>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["pong"]},"node_tag":"function","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":{"group":"<group digest of G10>","ordinal":5,"size":6},"semantic_form":"recursive_function","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**G12**: `if x > 0 then pong(x - 1) else true`, in `ping`, ordinal 1

```json
{"body":{"arguments":[{"target":{"digest":"<digest of E11>","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"ordinal":3,"term":"group_reference"},{"target":{"digest":"<digest of L1>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.control.if","laws":[],"leaves":[],"member":null,"mode":null},"operator":"conditional","result_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":{"ordinal":1,"size":6},"semantic_form":"conditional","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**G13**: `if x > 0 then ping(x - 1) else true`, in `pong`, ordinal 0

```json
{"body":{"arguments":[{"target":{"digest":"<digest of E11>","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"ordinal":2,"term":"group_reference"},{"target":{"digest":"<digest of L1>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.control.if","laws":[],"leaves":[],"member":null,"mode":null},"operator":"conditional","result_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":{"ordinal":0,"size":6},"semantic_form":"conditional","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**G14**: `ping(x - 1)`, in `pong`, ordinal 2

```json
{"body":{"arguments":[{"ordinal":4,"term":"group_reference"},{"target":{"digest":"<digest of E13>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.function.call","laws":[],"leaves":[],"member":null,"mode":null},"operator":"call","result_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":{"ordinal":2,"size":6},"semantic_form":"call","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.

**G15**: `pong(x - 1)`, in `ping`, ordinal 3

```json
{"body":{"arguments":[{"ordinal":5,"term":"group_reference"},{"target":{"digest":"<digest of E13>","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.function.call","laws":[],"leaves":[],"member":null,"mode":null},"operator":"call","result_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":{"ordinal":3,"size":6},"semantic_form":"call","semantic_type":{"digest":"<digest of T1>","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: the SHA-256 of the preimage above, as lowercase hex.


G1, group digest (the SHA-256 of the group preimage):

| Ordinal | Member | Anonymous signature | Full signature |
|---|---|---|---|
| 0 | G1 | the group member's own signature digest | the group member's own signature digest |

G2-G3, group digest (the SHA-256 of the group preimage):

| Ordinal | Member |
|---|---|
| 0 | G3 `Option<List>` |
| 1 | G2 `List` |

G4-G6, group digest (the SHA-256 of the group preimage):

| Ordinal | Member |
|---|---|
| 0 | G5 conditional |
| 1 | G6 call |
| 2 | G4 `f` |

G7-G9, group digest (the SHA-256 of the group preimage):

| Ordinal | Member |
|---|---|
| 0 | G7 `Tree` |
| 1 | G8 `Sequence<Tree>` |
| 2 | G9 `Sequence<Tree>[0, 3]` |

G10-G15, group digest (the SHA-256 of the group preimage):

| Ordinal | Member |
|---|---|
| 0 | G13 conditional in `pong` |
| 1 | G12 conditional in `ping` |
| 2 | G14 `ping(x - 1)` in `pong` |
| 3 | G15 `pong(x - 1)` in `ping` |
| 4 | G10 `ping` |
| 5 | G11 `pong` |

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-092-CON-1 | Only `check` builds a structural-node preimage and calls the kernel `NodeKey` constructor with its digest (ADR-011 FB-13, T-12). | Design | Inspection |
| FR-092-CON-2 | The match that selects a preimage by node kind, and the match that builds a type node from a checked `ValueType`, have no `_` or catch-all arm (ADR-012 §5.1). | Design | Test (TC-413) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-092-AC-1 | For the builtin `Boolean`, `Integer` and text scalar types, `Int[0, 9]`, `Option<Int[0, 9]>`, `Sequence<Int[0, 9]>` and `Sequence<Int[0, 9]>[0, 5]`, and `Text[0, 64; nfc]`, the type node `check` builds has the preimage bytes and key of vectors T1 to T8 exactly. | Test (TC-413) |
| FR-092-AC-2 | A unit under owner (`a`, `u`) and a unit under owner (`a`, `w`), each with a parameter typed `Int[0, 9]`, give that parameter's type node the same key, T4's. The declaration `record Point { x: Int[0, 9]; y: Int[0, 9]; }` keys to D1 under (`a`, `u`) and to D2 under (`a`, `w`), and two compiles under (`a`, `u`) give D1 both times. D1's preimage carries `owner` and `declaration`; T4's carries neither member. | Test (TC-413) |
| FR-092-AC-3 | With `type Digit = Int[0, 9];` and `function g using v(x: Digit): Boolean pure { true }`, the semantic type of `g`'s parameter node is T4, and the checked graph holds no node whose `declaration` is `Digit`. | Test (TC-413) |
| FR-092-AC-4 | For `function both using v(a: Boolean, b: Boolean): Boolean pure { a and b }`, `a`'s and `b`'s parameter nodes key to P1 and P2, and `both` keys to F2. For `function f using v(): Boolean pure { true }`, the literal node keys to L1 and `f` keys to F1. For `function h using v(a: Boolean): Boolean pure { let y = a in y }`, `y`'s parameter node keys to P3. For `function k using v(n: Boolean): Integer pure { 7 }`, the literal node keys to L2, whose preimage spells the value as the string `"7"`. | Test (TC-414) |
| FR-092-AC-5 | `function unused using v(a: Boolean, b: Boolean): Boolean pure { a }` gives a function node whose `parameters` binding lists P1 and then P2, although the body never reads `b`. `function both2 using v(a: Boolean, b: Boolean): Boolean pure { a and b }`, declared beside `both` in the same unit, reuses P1 and P2. | Test (TC-414) |
| FR-092-AC-6 | No function node's body contains an `application` term, and every function node's key is the SHA-256 of its `quire.structural-node/v1` preimage, which carries the unit's `owner`. The node of `a and b` in `both` is keyed by `quire.application-node/v1`; its preimage has no `owner` member, and its key is E1. | Test (TC-414) |
| FR-092-AC-7 | On a thread with a 512 KiB stack, a parameter typed with 100,000 nested `Option`s around `Boolean`, under S1 and S3 limits raised to fit it, is keyed, and its type node's key equals the key of the same type keyed on a thread with an 8 MiB stack. The recursive `f` of vectors G4 to G6 and the same declaration under the name `g`, calling `g`, in one unit: the preimages of the conditionals of `f` and `g` both hash to G5, and the package refuses with `unknown_required_feature`/`unsupported-feature` naming the regions of `f` and `g`, with no key for any member of either group. On a 2 MiB stack, at the default limits, a chain of 30 records each holding an optional field of the next, the last into a text field, checks. | Test (TC-413) |
| FR-092-AC-8 | An enum declaration is keyed by `quire.enum-declaration-node/v1` and its member by `quire.enum-member-node/v1`, never by `quire.structural-node/v1`: for QSpec's `enum-status` and `enum-status-ready` preimages in `node-identity-vectors.json`, the minted keys equal the recorded `sha256`. | Test (TC-413) |
| FR-092-AC-9 | `Rational[-9, 9; 1, 9]` and its base key to T10 and T9, `Decimal[-100000, 100000; 2, 2; nearest-even]` and its base to T12 and T11, and `tuple Pair(Int[0, 9], Int[0, 9]);` under (`a`, `u`) to D5. `record Opt { a: Int[0, 9]; b: Int[0, 9]?; }` keys to D3 and `record Opt { a: Int[0, 9]; b: Option<Int[0, 9]>; }` to D4, which differs. `rational(1, 2)` as a `Rational[-9, 9; 1, 9]` literal keys to L3, spelled `"1/2"`, and `rational(2, 4)` keys to L3 too. | Test (TC-413) |
| FR-092-AC-10 | `function m using v(x: Int[0, 9]): Boolean pure decreases(x) { true }` keys `x`'s parameter node to P4 and `m` to F3, whose body binds `decreases` to a `reference` to P4. | Test (TC-414) |
| FR-092-AC-11 | Each recursion group of the Recursion-group vectors checks and keys to its vectors' preimage bytes and keys: `f` to G4, G5 and G6 over L5 and E11 to E13, whose E12 holds the literal `1` as an inline literal term; a function whose body is the literal `1` to a graph holding L6; `List` to G2 and G3; `Tree` to G7, G8 and G9, whose G9 preimage writes its `semantic_type` as `{term: "group_reference", ordinal: 1}`; and `ping` and `pong` to G10 to G15. The key function keys G1. Declaring `pong` before `ping` gives the same keys as declaring `ping` first. Each in-group application node's preimage has `recursion` `{size, ordinal}` and no `group` member, and each structural one's `recursion.group` equals its group's digest. In `function h using v(x: Int[0, 9]): Boolean pure decreases(x) { if x > 0 then h(x - 1) and h(x - 1) else true }`, the two calls are one node, and `h`'s group has `size` 4: `h`, the conditional, the conjunction and the call. | Test (TC-413) |
| FR-092-AC-12 | `record Point { x: Int[0, 9]; y: Int[0, 9]; }` under (`a`, `u`), declared once through a `CompositeDeclaration` whose key is 32 bytes of `0x11` and once through one whose key is 32 bytes of `0x22`, keys to D1 both times, and its checked type node's id is D1 both times. No preimage, checked-graph node or checked type node holds either supplied key. | Test (TC-413) |
| FR-092-AC-13 | Under (`a`, `u`), `predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }` gives a node with `node_tag` `function`, `semantic_form` `predicate`, `declaration.qualified_name` `["Positive"]` and the unit's `owner`. `function Positive using v(x: Int[0, 9]): Boolean pure { x > 0 }`, alone in another unit under (`a`, `u`), gives a node with `semantic_form` `pure_function` and a key different from the predicate's. For `ordered enum Status { READY, DONE }` from source, the checked graph holds the declaration node with key FR-091 vector N1 (`scalar_type`, `enum`, `declaration.qualified_name` `["Status"]`) and the member node with key N2 (`value`, `enum_value`). | Test (TC-481) |
| FR-092-AC-14 | Parameters typed `Int[0, 18446744073709551615]`, `Int[0, 9223372036854775808]` and `Int[-170141183460469231731687303715884105728, 170141183460469231731687303715884105727]` and `Int[0, 18446744073709551616]` give type nodes with the preimage bytes and keys of vectors T13, T14, T15 and T16 exactly, each bound spelled as its decimal string. With those parameters in the same unit, T4's `Int[0, 9]` and every other vector of this requirement key unchanged. | Test (TC-910) |
| FR-092-AC-15 | QSL's node-key builder encodes `recursion.size`, `recursion.ordinal`, `group_reference.ordinal` and `operation.member.position` according to QSpec FR-322's counter rule. For both `group_reference.ordinal` and `operation.member.position`, the encoded preimage uses the JSON number `9007199254740991` at that value and the canonical decimal string `"9007199254740992"` at the next value; each key equals the SHA-256 of the RFC 8785 bytes of its encoded preimage. The result also equals every published `integer_encoding_vectors` key (FR-322-AC-58). Keying never refuses a counter for its magnitude: `NodeKeyRefusal` has no magnitude variant. | Test (TC-910) |
| FR-092-AC-16 | A unit that declares `protocol Fill` and holds `protocol Fill` as an expression emits a graph with one `protocol`/`protocol_reference` node whose body is exactly `{term: "reference", target: Fill's node key}`, whose `semantic_type` equals the `Fill` node's, whose `dependencies` are exactly that key, and which has one `expression` occurrence at the reference's region (ordinal 0) and no `declaration`, `owner`, nominal preimage or recursion group. The graph has one node collection and no protocol-reference collection. The emitted package admits through the IR v2 reader as `ProtocolForm::ProtocolReference`. | Test (TC-1021) |
| FR-092-AC-17 | Changing the target protocol so its node key changes, and re-emitting, changes the reference node's key and the package identity; a unit with two protocols and the reference written against each gives two different reference keys; the same unit emitted twice gives equal keys. A unit that holds `protocol Fill` twice emits one `protocol_reference` node for `Fill` with two `expression` occurrences, ordinals 0 and 1 in source order at the two references' regions, each with its own source-map row, and the node's key, body and `dependencies` equal the one-reference unit's carrier. Removing the second occurrence and its source-map row from that emitted package leaves the carrier key and the package identity unchanged. The reference's key equals the SHA-256 of the RFC 8785 bytes of its `quire.structural-node/v1` preimage. | Test (TC-1021) |
| FR-092-AC-18 | A unit that declares `protocol Fill` made only of the constructs FR-383's staged-emission paragraph lists as emitted (static and replicated roles, channels and their queues, `sequence`/`branch`/`case` controls, binders, captures and an `each` guard with no declaration read, a finish with no declaration read, scheduling) and that checks emits one `protocol`/`protocol_clause` node whose body is a `quire.op.protocol.control` application carrying the selected `protocol_profile` law and the `protocol` `profile_operator` member and the thirteen clause arguments of FR-383, whose `semantic_type` is the Boolean scalar type node, which is keyed by `quire.application-node/v1`, and which has one `declaration` occurrence at the declaration's region. The IR v2 reader admits it, and a reference to `Fill` (AC-16) resolves to this node's key. The unit selects a protocol profile by a header `profile` (FR-110): QSpec FR-322-AC-22 admits no `protocol_control` application without a selected law, so a unit that selects none emits no protocol node and no refusal for the declaration, and a reference to a protocol in it refuses `missing_declaration`/`missing-selection` (FR-218-AC-8). A protocol that names a construct FR-383 lists as not yet emitted refuses `unsupported_construct`/`not-yet-implemented` at the construct, naming it and the ticket that writes it, and writes no clause node. | Test (TC-1021) |

## Dependencies

- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  O-04 and OQ-G (owner scope; builtin and anonymous type nodes carry no
  owner), §2 (the one RFC 8785 implementation), QC-18 and QC-24 (the QSpec
  requests this preimage and node shapes make).
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §2.2 E3 (node identity is minted at E3), §3 FB-13 (only `check` mints a
  `NodeKey`), §2.3 (stage limits), and the M-6a OQ-7 ruling.
- [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md):
  the unit's `SourceOwner` input, the resolved types and aliases.
- [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md): the
  expression, literal and value nodes a function's body references.
- [FR-094](FR-094-key-model-owned-reference-population-and-quantity-nodes.md):
  model-owned nodes (`ModelOwner`), the `Reference`, `Population` and
  quantity type nodes, and clause functions.
- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md)
  §5.1: no catch-all arm in a family dispatch.
- [FR-088](FR-088-clause-name-and-type-identity.md) AC-7: every reference
  to a declared type resolves to its one node id, the FR-092 key.
- [FR-218](FR-218-check-every-protocol-control-construct-at-s3.md): the
  protocol reference's binding to its target key, and its refusals.
- [FR-383](FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md): the
  protocol nodes the clause node lists and its argument layout.
- QSpec FR-322-AC-66: the protocol-reference carrier this emission writes.
- QSpec FR-143 and FR-146: the recursive records and recursive functions
  that form recursion groups.
- `SourceOwner`'s `authority` and `identity` are those of the unit's
  `RawSourceRef` ([FR-001](FR-001-read-exact-source.md); ADR-013 §7 slice
  S-4b).
- QSpec. `quire.structural-node/v1`, the `value`/`parameter` semantic form
  and the function node body shape are QSL proposals (ADR-013 QC-24). QSL
  keys its nodes by them now and conforms to QSpec's arm once QSpec publishes
  one. The group order, the structural `recursion.group` member and the
  `group_reference` at a `semantic_type` position are QSL proposals too
  (QC-24). FR-322 and IR-242's reader derive an in-group ordinal from the
  writer's graph order, which QSL's emission sets to the group order
  (FR-093).

## Open Questions

- **FR-092-OQ-1: How does an in-group application node's key tell two
  recursion groups apart?** FR-322's `group_reference` names a member by its
  ordinal, and the `quire.application-node/v1` preimage has no member that
  names the group, so two groups that differ only in declared names give
  their in-group application nodes one key. QSL's structural preimage
  carries the group digest, so only application nodes collide. Until QSpec
  adds a group identity to the application-node `recursion` member (ADR-013
  QC-24), `check` refuses the collision (Groups that collide) rather than
  merge the nodes.
