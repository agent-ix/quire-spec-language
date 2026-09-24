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
application (`quire.application-node/v1`). The `Value` family also emits type
nodes, literal and aggregate value nodes, parameter nodes and function nodes
whose bodies hold no application. This requirement defines QSL's key for
those nodes: the `quire.structural-node/v1` preimage. It also fixes the node
shape of a type, a parameter and a function, which that key hashes.

ADR-013 O-04 keys these nodes by this QSL-proposed preimage, and QSL
conforms to QSpec's arm once QSpec publishes one (QC-18, QC-24). It follows the ADR-013 OQ-G ruling
(ADR-011 M-6a OQ-7): a declared node carries its owner, and a builtin or
anonymous type node carries none and shares one id across packages. QSpec
references (`ix://agent-ix/quire-specification`, cited by reference, never
copied): FR-322, `proposals/checked-package-v2/node-identity-preimage.schema.json`,
`schema.json`, `operation-catalog.json` and the positive fixtures, at
`e72756f`.

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
2. any other node whose body contains an `application` term by QSpec's
   `quire.application-node/v1` preimage, exactly as FR-322 defines it;
3. every other node by the `quire.structural-node/v1` preimage below.

The key is SHA-256 of the RFC 8785 bytes of the preimage, produced by the one
JCS implementation ADR-013 §2 names (`quire-canonical`).

### The `quire.structural-node/v1` preimage

The preimage is one JSON object with these members:

| Member | Value |
|---|---|
| `version` | `"quire.structural-node/v1"` |
| `owner` | Present exactly when `declaration` is not `null` or the node is model-owned: the declaring node's QSpec `Owner` (ADR-013 O-04). A `Value` declaration compiled from source carries its unit's `SourceOwner`, `{kind: "source", authority, identity}`. A model-owned node carries `ModelOwner` and a `null` `declaration` ([FR-094](FR-094-key-model-owned-reference-population-and-quantity-nodes.md)). Absent otherwise. |
| `node_tag` | The node's FR-322 `node_tag`. |
| `semantic_form` | The node's FR-322 `semantic_form`. |
| `semantic_type` | The `NodeId` (`{domain: "quire.checked-semantic-node/v1", digest}`) of the node's semantic type, or `null` when the node's semantic type is the node itself. When the semantic type is a member of the node's own recursion group, `{term: "group_reference", ordinal}` (Recursion groups). |
| `declaration` | `{qualified_name}` for a node with a `declaration` source occurrence (FR-322 `declaration`), else `null`. |
| `recursion` | `null` for a node outside every recursion group, else `{size, ordinal, group}` (Recursion groups). |
| `body` | The node's FR-322 body term. A `reference` to a member of the node's own recursion group is written `{term: "group_reference", ordinal}`, as `quire.application-node/v1` writes it. |

The members, the `group_reference` rule and the term shapes are those of
`quire.application-node/v1`, with four differences: the `version`, the
`owner` member of a declared or model-owned node, a `null`
`semantic_type` for a self-typed node, and the `group` member and
`semantic_type` position of an in-group node (Recursion groups). A
`scalar_type` and a `composite_type` node are their own semantic type, so
this `null` keeps their preimage acyclic.

A number never appears in a preimage as a JSON number except `recursion`'s
`size` and `ordinal`, a `group_reference` `ordinal` and an `operation.member`
`position`. `recursion`'s `group` is a lowercase hex string. A literal's value
is spelled by its `value_kind`:

- `integer`: the canonical decimal string of the value: no leading zero, no
  `+`, and `-` only before a nonzero value (QSpec `IntegerString`);
- `rational`: `"n/d"`, the reduced fraction with a positive denominator, and
  `"0/1"` for zero;
- `boolean`: a JSON boolean;
- `text`: the text as a JSON string;
- `none`: `null`.

Building a type node and lowering a checked expression each walk a tree
whose depth the check stage's depth limit bounds (`CheckingLimits`, at most
`MAX_CHECKING_DEPTH`; ADR-011 §2.3). A walk that would pass the limit
refuses with `resource_exhausted`/`insufficient-next-charge` naming the depth
limit (`CheckCause::ResourceExhausted`), and yields no key. A type node's
walk uses stack that does not grow with the nesting of the options,
collections and declared composites it follows; only the depth limit bounds
that nesting.

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
conditional and its recursive call), and so do the recursive records that
QSpec FR-143 admits (a record and the `Option` or collection node that its
field names). FR-143 admits a record cycle only through an optional field,
an `Option` or a collection whose minimum is zero, and admits no cycle
through a tuple position.

`check` SHALL key nodes in dependency order. It keys a node outside every
group after every node that node names. It keys a group after every node
outside the group that a member names. The components of the graph form a
directed acyclic graph, so this order exists. When `check` keys a group, it
already knows every key that the members name outside the group.

#### An in-group node's preimage

`check` SHALL key a member of a recursion group by the preimage its kind
selects (rule 2 or rule 3), with these members:

- `recursion` is `{size, ordinal}`, where `size` is the number of the
  group's classes and `ordinal` is the rank of the member's class in the
  group order below. In a `quire.structural-node/v1` preimage, `recursion`
  also has `group`, the group digest below. In a
  `quire.application-node/v1` preimage, `recursion` is exactly
  `{size, ordinal}`, as FR-322 writes it.
- Each position that names a member of the node's own group holds
  `{term: "group_reference", ordinal}`, with that member's ordinal, in place
  of the member's `reference` term or `NodeId`.

An application node names members of its own group only through body
`reference` terms: its `result_type`, its literal types and its member
declarations are type or model nodes, and a type or model node names no
expression, value or function node, so it is never in the application
node's group. The application-node preimage of an in-group node therefore
keeps FR-322's shape. A structural in-group node can name a member at its
`semantic_type`: in `record Tree { kids: Sequence<Tree>[0, 3]; }`, the
`collection_bounds` node's semantic type is the `Sequence<Tree>` node of the
same group (G9).

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
as IR-242's does, recomputes the keys `check` minted.

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
| a declared `record R { f: T; ... }` | `composite_type` / `record`, with `declaration` and `owner` | itself | one binding per field in declaration order, `f` = `reference(T)`; an optional field `f?: T` is `f` = `binding{name: "optional", value: reference(Option<T>)}`, so `f?: T` and `f: Option<T>` differ |
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
  in a recursion group, else `pure_function`;
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
| T1 | Boolean | `9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa` |
| T2 | Integer | `07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32` |
| T3 | Text scalar | `0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659` |
| T4 | Int[0, 9] | `652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477` |
| T5 | Option<Int[0, 9]> | `7bacf8b16f079a2352aac1a83b88a5925ed3f8c00e3a02735e8d3c646bb6461e` |
| T6 | Sequence<Int[0, 9]> | `27355f7b7768c0596876c2173262651d3d36946f3e8e53cef42d51551c804956` |
| T7 | Sequence<Int[0, 9]>[0, 5] | `515cb5664eae047ff9a2568809e02e6936cc6c8ab63fe54864f1e8f546e5f2c2` |
| T8 | Text[0, 64; nfc] | `bc0a6c39a41f825ff618b1c046130085607d7003db84df32c4c5059ebf57b297` |
| T9 | Rational scalar | `9587ede6610978546ca75deb5ff42a874e1b623c9c16f25678062ecc32c24ae9` |
| T10 | Rational[-9, 9; 1, 9] | `07d677acac076563319745323e09e3a6b3e83d658f0c1ea7e572e08f33732b9e` |
| T11 | Decimal scalar | `61e127863156fc90ecc98bf5563dc20687f4e481cfa6752c25cffb654c61ef2d` |
| T12 | Decimal[-100000, 100000; 2, 2; nearest-even] | `b5616d031335fd951ca74ed52996b665a5673769662dfcc6ee7194a0fb5dc9cc` |
| D1 | record Point, owner (a, u) | `45ff50317a846ffbc0853f4a4837507f244a3d302d0fa8605a7edf36da532ae4` |
| D2 | record Point, owner (a, w) | `c6894922cc1c3d8df23386b264075d71b1c62d1e9890b86b58b04200ad72dd88` |
| D3 | record Opt { a: Int[0, 9]; b?: Int[0, 9]; }, owner (a, u) | `05067eac40eb72b83e944e9d7a97b285c47a569a0404c2d973862d46aafc91f6` |
| D4 | record Opt { a: Int[0, 9]; b: Option<Int[0, 9]>; }, owner (a, u) | `11e8d0d336ec22793fc88addb6b493975d8064dcd2de4c2184f15a1302925600` |
| D5 | tuple Pair(Int[0, 9], Int[0, 9]), owner (a, u) | `e519e1b5b543cfa0cf021c9e489d6d5bac5d13b6545a124e8247200195aed560` |
| P1 | parameter a: Boolean, level 0 | `838088fb2300dd016cf10707e297afbd2f6209eb7e20c24757d515fda8ee6cf1` |
| P2 | parameter b: Boolean, level 1 | `555416913f6f787765eeb816c83af4650d7a0e122b8e3202f8b8d13b144fef59` |
| P3 | let binder y: Boolean, level 1 | `2acded6f19f94b965e38043f06716f9b775e790cecd3f9557baa4f4b17073365` |
| P4 | parameter x: Int[0, 9], level 0 | `ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda` |
| L1 | literal true | `03a8898fec9612e8a5faac4927eef10568cd62179e71acc17c86eddb12a792e9` |
| L2 | literal 7 | `9f42fd255e4db2821dffdd2da201755df67dec54e8b0814f8160cab40a44b29b` |
| L3 | literal rational(1, 2) of type Rational[-9, 9; 1, 9] | `2ada675e3655813b63b63a9d926d9a3969c7950e53bb6d78b72dd64948911c71` |
| F1 | function f() -> Boolean { true }, owner (a, u) | `dbd06f242fc36f1ed1b5773a7e59fb89ebc862494d8512b44e84942bea153e79` |
| E1 | a and b | `a98896386ccee595ae1cc04f4f83c8792fc04e1027e779ac7def428592c8b21e` |
| F2 | function both(a: Boolean, b: Boolean): Boolean { a and b }, owner (a, u) | `2597b9bf514c3dd93654daca8fbea64d0a4622ea8bc5002888ce72ecb2520454` |
| E2 | both(a, true) | `d5af48cd20c8ebceb650b0137d834b4ae764dd99ae910214d33a3f3cd2785b36` |
| E3 | let y = a in y | `faa9bf455e92d25b5622dd31501c1341fe447b1e641407dae5522ee6dd5e8a4a` |
| F3 | function m(x: Int[0, 9]): Boolean decreases(x) { true }, owner (a, u) | `e13010a50a476f31b5955ef7ac6e008d83a7af661b0ff1d52c9fd6b7c1966ed3` |
| L5 | literal `0` | `22fec75bfbe7f7c5d20d19818c4d7c41bf1ebaf9a94569374228ba9a13376779` |
| L6 | literal `1` | `833ceccc8eed0661d0edc4b5f1ccb5e6df51da22a979423327b2aeba83bb8366` |
| E11 | `x > 0` | `68fb5483171f2b690b74efb9c20a9eee7d3b366d544ea201e402d77e59d9284e` |
| E12 | `x - 1` | `1b6f75d7ed4c5b25ce7addecd26dfc9f93e3265f9218ad812d62c943955bf63a` |
| E13 | `x - 1` narrowed into `Int[0, 9]` | `f1b8c8bc5d8f3487a5c39d09e5e8ab2da5e089ebb3bc3de1bdea3cd61e5b695e` |
| G1 | an `option` node over itself, a one-member group, ordinal 0 | `7b2e6632de9e716f1f7b6a3155ea4e6a36129ea1e4473f539d52a75001ae5e31` |
| G2 | `record List { next?: List; }`, ordinal 0 | `4471223e43af6f4377ab4e8a057b54a92020015e79944b9978ce90253f7e9636` |
| G3 | `Option<List>`, ordinal 1 | `96901f2222cafaed9d09389407a1c38b35e99111f00abf38e3fb6a8aa84c9cae` |
| G4 | function `f`, ordinal 1 | `23cdc2faddac19360e414395846f07003b83144a5da98cc50351485d4238570d` |
| G5 | `if x > 0 then f(x - 1) else true`, ordinal 0 | `3d8af00b18a2c9f774c89ad6da55ae8ff8aa06ecd1f3d7216ae202ab822c34fc` |
| G6 | `f(x - 1)`, ordinal 2 | `8cd0eadcbc67918d0ba72b19553898a4fa9693bb79db7dc76f266486fe07d840` |
| G7 | `record Tree { kids: Sequence<Tree>[0, 3]; }`, ordinal 0 | `5dd245d1f95885f7c628dfa8424f4b4ddb616a218369984e488731a5e3061c64` |
| G8 | `Sequence<Tree>`, ordinal 1 | `5dab1d21b36212959d997c17b88ab44822bf4078e35ae1b993c17db591425756` |
| G9 | `Sequence<Tree>[0, 3]`, ordinal 2 | `6bd6dd7f2b655affb65a4b49dfcff572129d960f9d45766bf954cfc8ce6a4028` |
| G10 | function `ping`, ordinal 2 | `e491fbeae992168fef1e681476a9a20e4794eac40f377174e6700189450cf345` |
| G11 | function `pong`, ordinal 3 | `7fd05a5af745fd388daa29067d9e7fc15ef4122bc6ffac7afde2dfdb11aaa399` |
| G12 | `if x > 0 then pong(x - 1) else true`, in `ping`, ordinal 0 | `0c10a4780d09fec3b279bbbe1775d14af5dac2d3b3b61695d14b6d192d352830` |
| G13 | `if x > 0 then ping(x - 1) else true`, in `pong`, ordinal 1 | `ac6436d7164016e2ab8ccb1926ae820ca7e6625b50909d289b479cf1835d49ea` |
| G14 | `ping(x - 1)`, in `pong`, ordinal 4 | `2dc8b60b64805f36d8cb9d363ef4c090fc037251212f433153398cf15bdc638f` |
| G15 | `pong(x - 1)`, in `ping`, ordinal 5 | `df9d5492f79e107bc06affbb920b96ff562d7b976580f0f831858e9c185d82ea` |

**T1**: Boolean

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"boolean","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa`

**T2**: Integer

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"integer","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32`

**T3**: Text scalar

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"text","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659`

**T4**: Int[0, 9]

```json
{"body":{"members":[{"name":"min","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}},{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"9","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"integer_range","semantic_type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477`

**T5**: Option<Int[0, 9]>

```json
{"body":{"members":[{"target":{"digest":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":null,"semantic_form":"option","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `7bacf8b16f079a2352aac1a83b88a5925ed3f8c00e3a02735e8d3c646bb6461e`

**T6**: Sequence<Int[0, 9]>

```json
{"body":{"members":[{"target":{"digest":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":null,"semantic_form":"sequence","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `27355f7b7768c0596876c2173262651d3d36946f3e8e53cef42d51551c804956`

**T7**: Sequence<Int[0, 9]>[0, 5]

```json
{"body":{"members":[{"name":"min","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}},{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"5","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"collection_bounds","semantic_type":{"digest":"27355f7b7768c0596876c2173262651d3d36946f3e8e53cef42d51551c804956","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `515cb5664eae047ff9a2568809e02e6936cc6c8ab63fe54864f1e8f546e5f2c2`

**T8**: Text[0, 64; nfc]

```json
{"body":{"members":[{"name":"min","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}},{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"64","value_kind":"integer"}},{"name":"text_profile","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"nfc","value_kind":"text"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"text_bounds","semantic_type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `bc0a6c39a41f825ff618b1c046130085607d7003db84df32c4c5059ebf57b297`

**T9**: Rational scalar

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"rational","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `9587ede6610978546ca75deb5ff42a874e1b623c9c16f25678062ecc32c24ae9`

**T10**: Rational[-9, 9; 1, 9]

```json
{"body":{"members":[{"name":"numerator_min","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"-9","value_kind":"integer"}},{"name":"numerator_max","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"9","value_kind":"integer"}},{"name":"denominator_min","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"}},{"name":"denominator_max","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"9","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"rational_range","semantic_type":{"digest":"9587ede6610978546ca75deb5ff42a874e1b623c9c16f25678062ecc32c24ae9","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `07d677acac076563319745323e09e3a6b3e83d658f0c1ea7e572e08f33732b9e`

**T11**: Decimal scalar

```json
{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"decimal","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `61e127863156fc90ecc98bf5563dc20687f4e481cfa6752c25cffb654c61ef2d`

**T12**: Decimal[-100000, 100000; 2, 2; nearest-even]

```json
{"body":{"members":[{"name":"coefficient_min","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"-100000","value_kind":"integer"}},{"name":"coefficient_max","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"100000","value_kind":"integer"}},{"name":"scale_min","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"2","value_kind":"integer"}},{"name":"scale_max","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"2","value_kind":"integer"}},{"name":"rounding","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"nearest-even","value_kind":"text"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"decimal_range","semantic_type":{"digest":"61e127863156fc90ecc98bf5563dc20687f4e481cfa6752c25cffb654c61ef2d","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `b5616d031335fd951ca74ed52996b665a5673769662dfcc6ee7194a0fb5dc9cc`

**D1**: record Point, owner (a, u)

```json
{"body":{"members":[{"name":"x","term":"binding","value":{"target":{"digest":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"y","term":"binding","value":{"target":{"digest":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["Point"]},"node_tag":"composite_type","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":null,"semantic_form":"record","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `45ff50317a846ffbc0853f4a4837507f244a3d302d0fa8605a7edf36da532ae4`

**D2**: record Point, owner (a, w)

```json
{"body":{"members":[{"name":"x","term":"binding","value":{"target":{"digest":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"y","term":"binding","value":{"target":{"digest":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["Point"]},"node_tag":"composite_type","owner":{"authority":"a","identity":"w","kind":"source"},"recursion":null,"semantic_form":"record","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `c6894922cc1c3d8df23386b264075d71b1c62d1e9890b86b58b04200ad72dd88`

**D3**: record Opt { a: Int[0, 9]; b?: Int[0, 9]; }, owner (a, u)

```json
{"body":{"members":[{"name":"a","term":"binding","value":{"target":{"digest":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"b","term":"binding","value":{"name":"optional","term":"binding","value":{"target":{"digest":"7bacf8b16f079a2352aac1a83b88a5925ed3f8c00e3a02735e8d3c646bb6461e","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}}],"term":"aggregate"},"declaration":{"qualified_name":["Opt"]},"node_tag":"composite_type","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":null,"semantic_form":"record","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `05067eac40eb72b83e944e9d7a97b285c47a569a0404c2d973862d46aafc91f6`

**D4**: record Opt { a: Int[0, 9]; b: Option<Int[0, 9]>; }, owner (a, u)

```json
{"body":{"members":[{"name":"a","term":"binding","value":{"target":{"digest":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"b","term":"binding","value":{"target":{"digest":"7bacf8b16f079a2352aac1a83b88a5925ed3f8c00e3a02735e8d3c646bb6461e","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["Opt"]},"node_tag":"composite_type","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":null,"semantic_form":"record","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `11e8d0d336ec22793fc88addb6b493975d8064dcd2de4c2184f15a1302925600`

**D5**: tuple Pair(Int[0, 9], Int[0, 9]), owner (a, u)

```json
{"body":{"members":[{"target":{"digest":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":{"qualified_name":["Pair"]},"node_tag":"composite_type","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":null,"semantic_form":"tuple","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `e519e1b5b543cfa0cf021c9e489d6d5bac5d13b6545a124e8247200195aed560`

**P1**: parameter a: Boolean, level 0

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"a","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `838088fb2300dd016cf10707e297afbd2f6209eb7e20c24757d515fda8ee6cf1`

**P2**: parameter b: Boolean, level 1

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"b","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `555416913f6f787765eeb816c83af4650d7a0e122b8e3202f8b8d13b144fef59`

**P3**: let binder y: Boolean, level 1

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"y","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `2acded6f19f94b965e38043f06716f9b775e790cecd3f9557baa4f4b17073365`

**P4**: parameter x: Int[0, 9], level 0

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"x","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda`

**L1**: literal true

```json
{"body":{"term":"literal","type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"value":true,"value_kind":"boolean"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"literal","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `03a8898fec9612e8a5faac4927eef10568cd62179e71acc17c86eddb12a792e9`

**L2**: literal 7

```json
{"body":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"7","value_kind":"integer"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"literal","semantic_type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `9f42fd255e4db2821dffdd2da201755df67dec54e8b0814f8160cab40a44b29b`

**L3**: literal rational(1, 2) of type Rational[-9, 9; 1, 9]

```json
{"body":{"term":"literal","type":{"digest":"07d677acac076563319745323e09e3a6b3e83d658f0c1ea7e572e08f33732b9e","domain":"quire.checked-semantic-node/v1"},"value":"1/2","value_kind":"rational"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"literal","semantic_type":{"digest":"07d677acac076563319745323e09e3a6b3e83d658f0c1ea7e572e08f33732b9e","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `2ada675e3655813b63b63a9d926d9a3969c7950e53bb6d78b72dd64948911c71`

**F1**: function f() -> Boolean { true }, owner (a, u)

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[],"term":"aggregate"}},{"name":"body","term":"binding","value":{"target":{"digest":"03a8898fec9612e8a5faac4927eef10568cd62179e71acc17c86eddb12a792e9","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["f"]},"node_tag":"function","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":null,"semantic_form":"pure_function","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `dbd06f242fc36f1ed1b5773a7e59fb89ebc862494d8512b44e84942bea153e79`

**E1**: a and b

```json
{"body":{"arguments":[{"target":{"digest":"838088fb2300dd016cf10707e297afbd2f6209eb7e20c24757d515fda8ee6cf1","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"555416913f6f787765eeb816c83af4650d7a0e122b8e3202f8b8d13b144fef59","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.boolean.and","laws":[],"leaves":[],"member":null,"mode":null},"operator":"binary","result_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"binary","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `a98896386ccee595ae1cc04f4f83c8792fc04e1027e779ac7def428592c8b21e`

**F2**: function both(a: Boolean, b: Boolean): Boolean { a and b }, owner (a, u)

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"838088fb2300dd016cf10707e297afbd2f6209eb7e20c24757d515fda8ee6cf1","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"555416913f6f787765eeb816c83af4650d7a0e122b8e3202f8b8d13b144fef59","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"target":{"digest":"a98896386ccee595ae1cc04f4f83c8792fc04e1027e779ac7def428592c8b21e","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["both"]},"node_tag":"function","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":null,"semantic_form":"pure_function","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `2597b9bf514c3dd93654daca8fbea64d0a4622ea8bc5002888ce72ecb2520454`

**E2**: both(a, true)

```json
{"body":{"arguments":[{"target":{"digest":"2597b9bf514c3dd93654daca8fbea64d0a4622ea8bc5002888ce72ecb2520454","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"838088fb2300dd016cf10707e297afbd2f6209eb7e20c24757d515fda8ee6cf1","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"03a8898fec9612e8a5faac4927eef10568cd62179e71acc17c86eddb12a792e9","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.function.call","laws":[],"leaves":[],"member":null,"mode":null},"operator":"call","result_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"call","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `d5af48cd20c8ebceb650b0137d834b4ae764dd99ae910214d33a3f3cd2785b36`

**E3**: let y = a in y

```json
{"body":{"arguments":[{"name":"y","term":"binding","value":{"target":{"digest":"838088fb2300dd016cf10707e297afbd2f6209eb7e20c24757d515fda8ee6cf1","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"target":{"digest":"2acded6f19f94b965e38043f06716f9b775e790cecd3f9557baa4f4b17073365","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.control.let","laws":[],"leaves":[],"member":null,"mode":null},"operator":"let","result_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"let","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `faa9bf455e92d25b5622dd31501c1341fe447b1e641407dae5522ee6dd5e8a4a`

**F3**: function m(x: Int[0, 9]): Boolean decreases(x) { true }, owner (a, u)

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"target":{"digest":"03a8898fec9612e8a5faac4927eef10568cd62179e71acc17c86eddb12a792e9","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"decreases","term":"binding","value":{"target":{"digest":"ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["m"]},"node_tag":"function","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":null,"semantic_form":"pure_function","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `e13010a50a476f31b5955ef7ac6e008d83a7af661b0ff1d52c9fd6b7c1966ed3`

#### Recursion-group vectors

L5, L6 and E11 to E13 are nodes outside any group that the recursive
functions below name. G1 to G15 are members of recursion groups, and every
declared member is under owner (`a`, `u`):

- G1: a one-member group, a `composite_type`/`option` node whose body
  references itself. No QSL source forms a one-member group: a function
  names its body's root expression, which is never the function, and a
  record that names itself directly (`record R { next: R; }`) is a cycle
  that FR-143 refuses, since it passes no optional field, `Option` or
  minimum-zero collection. TC-413 keys G1
  through the key function directly.
- G2 and G3: `record List { next?: List; }`, a group of two.
- G4 to G6: `function f using v(x: Int[0, 9]): Boolean pure decreases(x) { if x > 0 then f(x - 1) else true }`,
  a self-recursive function, a group of three. `x - 1` is `Integer` (E12),
  and the call narrows it into the parameter's `Int[0, 9]` (E13).
- G7 to G9: `record Tree { kids: Sequence<Tree>[0, 3]; }`, a group of three
  whose `collection_bounds` member names another member at its
  `semantic_type`.
- G10 to G15: `function ping using v(x: Int[0, 9]): Boolean pure decreases(x) { if x > 0 then pong(x - 1) else true }`
  and `function pong`, the same body calling `ping`, a mutually recursive
  pair whose bodies differ only in which function they call, a group of six.

Each group's signatures, in group order, and its group digest are below.
The anonymous pass orders G4 to G6, G2 and G3 and G7 to G9 completely. For
G10 to G15 it leaves three pairs of equal anonymous signatures, and the full
pass orders each pair by the names `ping` and `pong`.

**L5**: literal `0`

```json
{"body":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"literal","semantic_type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `22fec75bfbe7f7c5d20d19818c4d7c41bf1ebaf9a94569374228ba9a13376779`

**L6**: literal `1`

```json
{"body":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"literal","semantic_type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `833ceccc8eed0661d0edc4b5f1ccb5e6df51da22a979423327b2aeba83bb8366`

**E11**: `x > 0`

```json
{"body":{"arguments":[{"target":{"digest":"ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"22fec75bfbe7f7c5d20d19818c4d7c41bf1ebaf9a94569374228ba9a13376779","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.integer.gt","laws":[],"leaves":[],"member":null,"mode":null},"operator":"binary","result_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"binary","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `68fb5483171f2b690b74efb9c20a9eee7d3b366d544ea201e402d77e59d9284e`

**E12**: `x - 1`

```json
{"body":{"arguments":[{"target":{"digest":"ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"833ceccc8eed0661d0edc4b5f1ccb5e6df51da22a979423327b2aeba83bb8366","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.integer.sub","laws":[],"leaves":[],"member":null,"mode":null},"operator":"binary","result_type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"binary","semantic_type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `1b6f75d7ed4c5b25ce7addecd26dfc9f93e3265f9218ad812d62c943955bf63a`

**E13**: `x - 1` narrowed into `Int[0, 9]`

```json
{"body":{"arguments":[{"target":{"digest":"1b6f75d7ed4c5b25ce7addecd26dfc9f93e3265f9218ad812d62c943955bf63a","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.numeric.narrow","laws":[],"leaves":[],"member":{"declaration":{"digest":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477","domain":"quire.checked-semantic-node/v1"},"kind":"type_argument"},"mode":null},"operator":"convert","result_type":{"digest":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"conversion","semantic_type":{"digest":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `f1b8c8bc5d8f3487a5c39d09e5e8ab2da5e089ebb3bc3de1bdea3cd61e5b695e`

**G1**: an `option` node over itself, a one-member group, ordinal 0

```json
{"body":{"members":[{"ordinal":0,"term":"group_reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":{"group":"4a005f58e201e284473264dd016bbcc0a1cfcd8a26031428f8dac6a969e9b14b","ordinal":0,"size":1},"semantic_form":"option","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `7b2e6632de9e716f1f7b6a3155ea4e6a36129ea1e4473f539d52a75001ae5e31`

**G2**: `record List { next?: List; }`, ordinal 0

```json
{"body":{"members":[{"name":"next","term":"binding","value":{"name":"optional","term":"binding","value":{"ordinal":1,"term":"group_reference"}}}],"term":"aggregate"},"declaration":{"qualified_name":["List"]},"node_tag":"composite_type","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":{"group":"c79c9c74f6849f66fcf9d2d2af6c4fddd157c567785083171cfa5c7acabfc660","ordinal":0,"size":2},"semantic_form":"record","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `4471223e43af6f4377ab4e8a057b54a92020015e79944b9978ce90253f7e9636`

**G3**: `Option<List>`, ordinal 1

```json
{"body":{"members":[{"ordinal":0,"term":"group_reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":{"group":"c79c9c74f6849f66fcf9d2d2af6c4fddd157c567785083171cfa5c7acabfc660","ordinal":1,"size":2},"semantic_form":"option","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `96901f2222cafaed9d09389407a1c38b35e99111f00abf38e3fb6a8aa84c9cae`

**G4**: function `f`, ordinal 1

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"ordinal":0,"term":"group_reference"}},{"name":"decreases","term":"binding","value":{"target":{"digest":"ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["f"]},"node_tag":"function","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":{"group":"0b9e8d18320d0ce587699e40ac33a25fd41c4a640226bda4b8b1521edc5e4c50","ordinal":1,"size":3},"semantic_form":"recursive_function","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `23cdc2faddac19360e414395846f07003b83144a5da98cc50351485d4238570d`

**G5**: `if x > 0 then f(x - 1) else true`, ordinal 0

```json
{"body":{"arguments":[{"target":{"digest":"68fb5483171f2b690b74efb9c20a9eee7d3b366d544ea201e402d77e59d9284e","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"ordinal":2,"term":"group_reference"},{"target":{"digest":"03a8898fec9612e8a5faac4927eef10568cd62179e71acc17c86eddb12a792e9","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.control.if","laws":[],"leaves":[],"member":null,"mode":null},"operator":"conditional","result_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":{"ordinal":0,"size":3},"semantic_form":"conditional","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `3d8af00b18a2c9f774c89ad6da55ae8ff8aa06ecd1f3d7216ae202ab822c34fc`

**G6**: `f(x - 1)`, ordinal 2

```json
{"body":{"arguments":[{"ordinal":1,"term":"group_reference"},{"target":{"digest":"f1b8c8bc5d8f3487a5c39d09e5e8ab2da5e089ebb3bc3de1bdea3cd61e5b695e","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.function.call","laws":[],"leaves":[],"member":null,"mode":null},"operator":"call","result_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":{"ordinal":2,"size":3},"semantic_form":"call","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `8cd0eadcbc67918d0ba72b19553898a4fa9693bb79db7dc76f266486fe07d840`

**G7**: `record Tree { kids: Sequence<Tree>[0, 3]; }`, ordinal 0

```json
{"body":{"members":[{"name":"kids","term":"binding","value":{"ordinal":2,"term":"group_reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["Tree"]},"node_tag":"composite_type","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":{"group":"098352bbb5f6c2dd2836b1dc59c7bbc6fdf93076c767bf4c358304da0b98ca6b","ordinal":0,"size":3},"semantic_form":"record","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `5dd245d1f95885f7c628dfa8424f4b4ddb616a218369984e488731a5e3061c64`

**G8**: `Sequence<Tree>`, ordinal 1

```json
{"body":{"members":[{"ordinal":0,"term":"group_reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":{"group":"098352bbb5f6c2dd2836b1dc59c7bbc6fdf93076c767bf4c358304da0b98ca6b","ordinal":1,"size":3},"semantic_form":"sequence","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `5dab1d21b36212959d997c17b88ab44822bf4078e35ae1b993c17db591425756`

**G9**: `Sequence<Tree>[0, 3]`, ordinal 2

```json
{"body":{"members":[{"name":"min","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}},{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"3","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":{"group":"098352bbb5f6c2dd2836b1dc59c7bbc6fdf93076c767bf4c358304da0b98ca6b","ordinal":2,"size":3},"semantic_form":"collection_bounds","semantic_type":{"ordinal":1,"term":"group_reference"},"version":"quire.structural-node/v1"}
```

Key: `6bd6dd7f2b655affb65a4b49dfcff572129d960f9d45766bf954cfc8ce6a4028`

**G10**: function `ping`, ordinal 2

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"ordinal":0,"term":"group_reference"}},{"name":"decreases","term":"binding","value":{"target":{"digest":"ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["ping"]},"node_tag":"function","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":{"group":"8383f625c29862ff9fe9bc66d7a03140f76a54e39153e9158c4f40cecd2597aa","ordinal":2,"size":6},"semantic_form":"recursive_function","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `e491fbeae992168fef1e681476a9a20e4794eac40f377174e6700189450cf345`

**G11**: function `pong`, ordinal 3

```json
{"body":{"members":[{"name":"parameters","term":"binding","value":{"members":[{"target":{"digest":"ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"}},{"name":"body","term":"binding","value":{"ordinal":1,"term":"group_reference"}},{"name":"decreases","term":"binding","value":{"target":{"digest":"ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda","domain":"quire.checked-semantic-node/v1"},"term":"reference"}}],"term":"aggregate"},"declaration":{"qualified_name":["pong"]},"node_tag":"function","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":{"group":"8383f625c29862ff9fe9bc66d7a03140f76a54e39153e9158c4f40cecd2597aa","ordinal":3,"size":6},"semantic_form":"recursive_function","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `7fd05a5af745fd388daa29067d9e7fc15ef4122bc6ffac7afde2dfdb11aaa399`

**G12**: `if x > 0 then pong(x - 1) else true`, in `ping`, ordinal 0

```json
{"body":{"arguments":[{"target":{"digest":"68fb5483171f2b690b74efb9c20a9eee7d3b366d544ea201e402d77e59d9284e","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"ordinal":5,"term":"group_reference"},{"target":{"digest":"03a8898fec9612e8a5faac4927eef10568cd62179e71acc17c86eddb12a792e9","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.control.if","laws":[],"leaves":[],"member":null,"mode":null},"operator":"conditional","result_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":{"ordinal":0,"size":6},"semantic_form":"conditional","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `0c10a4780d09fec3b279bbbe1775d14af5dac2d3b3b61695d14b6d192d352830`

**G13**: `if x > 0 then ping(x - 1) else true`, in `pong`, ordinal 1

```json
{"body":{"arguments":[{"target":{"digest":"68fb5483171f2b690b74efb9c20a9eee7d3b366d544ea201e402d77e59d9284e","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"ordinal":4,"term":"group_reference"},{"target":{"digest":"03a8898fec9612e8a5faac4927eef10568cd62179e71acc17c86eddb12a792e9","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.control.if","laws":[],"leaves":[],"member":null,"mode":null},"operator":"conditional","result_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":{"ordinal":1,"size":6},"semantic_form":"conditional","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `ac6436d7164016e2ab8ccb1926ae820ca7e6625b50909d289b479cf1835d49ea`

**G14**: `ping(x - 1)`, in `pong`, ordinal 4

```json
{"body":{"arguments":[{"ordinal":2,"term":"group_reference"},{"target":{"digest":"f1b8c8bc5d8f3487a5c39d09e5e8ab2da5e089ebb3bc3de1bdea3cd61e5b695e","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.function.call","laws":[],"leaves":[],"member":null,"mode":null},"operator":"call","result_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":{"ordinal":4,"size":6},"semantic_form":"call","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `2dc8b60b64805f36d8cb9d363ef4c090fc037251212f433153398cf15bdc638f`

**G15**: `pong(x - 1)`, in `ping`, ordinal 5

```json
{"body":{"arguments":[{"ordinal":3,"term":"group_reference"},{"target":{"digest":"f1b8c8bc5d8f3487a5c39d09e5e8ab2da5e089ebb3bc3de1bdea3cd61e5b695e","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.function.call","laws":[],"leaves":[],"member":null,"mode":null},"operator":"call","result_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":{"ordinal":5,"size":6},"semantic_form":"call","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `df9d5492f79e107bc06affbb920b96ff562d7b976580f0f831858e9c185d82ea`


G1, group digest `4a005f58e201e284473264dd016bbcc0a1cfcd8a26031428f8dac6a969e9b14b`:

| Ordinal | Member | Anonymous signature | Full signature |
|---|---|---|---|
| 0 | G1 | `3ae296ebb5c73914192b56dcb1ed43464dc277339747b6840db238a5d65f51e4` | `3ae296ebb5c73914192b56dcb1ed43464dc277339747b6840db238a5d65f51e4` |

G2-G3, group digest `c79c9c74f6849f66fcf9d2d2af6c4fddd157c567785083171cfa5c7acabfc660`:

| Ordinal | Member | Anonymous signature | Full signature |
|---|---|---|---|
| 0 | G2 `List` | `597649119d3c999ef649f87ed744b3d744ca4e00804501844b2e03b5afae0ee1` | `9d185dc6e7992d708d2399672036b0c37d3904a42995bdad24d7eb8734216018` |
| 1 | G3 `Option<List>` | `5e72aa9ed1dbb764923d3ab79e2bd94a4f47354dc8b81156904f2757a08d9e1e` | `3fd8713484ef742d2d10a182ad17f1d6c195d5c9341332c7d6b40d6f9e7fab5f` |

G4-G6, group digest `0b9e8d18320d0ce587699e40ac33a25fd41c4a640226bda4b8b1521edc5e4c50`:

| Ordinal | Member | Anonymous signature | Full signature |
|---|---|---|---|
| 0 | G5 conditional | `4d44301461c0af30b4de0d11ceb8d03db50c3248cb144bd50b7111111d22e24b` | `4d44301461c0af30b4de0d11ceb8d03db50c3248cb144bd50b7111111d22e24b` |
| 1 | G4 `f` | `4eab5ac7dd3f75eced5c6e5dbf48b230a5be7ce400eb28bc2ba2e28a8f551924` | `c9fd4dba85eed73a0de9e90dd1fa4e1a6cc1704649447b02b74c1711bb9d2cad` |
| 2 | G6 call | `8732b7655fc2c7103dcccfa754ec175365601bbe145d29db1a5acb8f700b1359` | `2f6d9adc0049dfa1bc615e7aff49df9f59c6aad3d915f7a2890779339fb50ba4` |

G7-G9, group digest `098352bbb5f6c2dd2836b1dc59c7bbc6fdf93076c767bf4c358304da0b98ca6b`:

| Ordinal | Member | Anonymous signature | Full signature |
|---|---|---|---|
| 0 | G7 `Tree` | `1a9e910b451eedeb1dbaaa173d871637655c112bca6cbf8d9f75b099bb60c737` | `f5a34062f94b31dfb8aa4b9f751fec3b21f9b3cac747530cdbe17b07f4932f17` |
| 1 | G8 `Sequence<Tree>` | `3775af7b9c7af621ec1d1298105962edd74e00cbc9c2d93b5418d01839bec5a4` | `7e043c029fe763c3e825ac4a798dc3fc8e0ecd2b9c0193072a47e744b8369b77` |
| 2 | G9 `Sequence<Tree>[0, 3]` | `fa0d0a0301803db710840ce7bdee7d49e317c2839559694bb1ea65be5d3e2a2d` | `fa0d0a0301803db710840ce7bdee7d49e317c2839559694bb1ea65be5d3e2a2d` |

G10-G15, group digest `8383f625c29862ff9fe9bc66d7a03140f76a54e39153e9158c4f40cecd2597aa`:

| Ordinal | Member | Anonymous signature | Full signature |
|---|---|---|---|
| 0 | G12 conditional in `ping` | `4d44301461c0af30b4de0d11ceb8d03db50c3248cb144bd50b7111111d22e24b` | `a12abd67c2361ee759d6a3e66779721a619ddf8d54a1b6a7258924dc06cf9c65` |
| 1 | G13 conditional in `pong` | `4d44301461c0af30b4de0d11ceb8d03db50c3248cb144bd50b7111111d22e24b` | `a4c91c06d164df255f535982d9ac8dc4ef99c51abe79180795bd1f1df815fe48` |
| 2 | G10 `ping` | `4eab5ac7dd3f75eced5c6e5dbf48b230a5be7ce400eb28bc2ba2e28a8f551924` | `2a3e7409858e5570380cf5ccef819d73da685f9b1aa39e30aed8a6e1433f9395` |
| 3 | G11 `pong` | `4eab5ac7dd3f75eced5c6e5dbf48b230a5be7ce400eb28bc2ba2e28a8f551924` | `8158c700f25d06bb06974e7c7f9ffa1587ef05d2e835373aec4b65501c38ba99` |
| 4 | G14 `ping(x - 1)` | `8732b7655fc2c7103dcccfa754ec175365601bbe145d29db1a5acb8f700b1359` | `dbc24afb445fa39f541b71fb0ae972a84864891316f185aaa5006cf0c6c2e431` |
| 5 | G15 `pong(x - 1)` | `8732b7655fc2c7103dcccfa754ec175365601bbe145d29db1a5acb8f700b1359` | `f989ebfae08ce217678d4973bafc13c5c0cc10252a204a8f5913b2e52fed4b27` |

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
| FR-092-AC-7 | With the check stage's depth limit set to 4 (`CheckingLimits`), a parameter typed `Option<Option<Option<Option<Boolean>>>>` is keyed, and one typed with five nested `Option`s refuses with `resource_exhausted`/`insufficient-next-charge` naming the depth limit, and yields no key. The recursive `f` of vectors G4 to G6 and the same declaration under the name `g`, calling `g`, in one unit: the preimages of the conditionals of `f` and `g` both hash to G5, and the package refuses with `unknown_required_feature`/`unsupported-feature` naming the regions of `f` and `g`, with no key for any member of either group. On a 2 MiB stack, at the default limits, a chain of 30 records each holding an optional field of the next, the last into a text field, checks; a chain of 1,000 refuses naming the depth limit. | Test (TC-413) |
| FR-092-AC-8 | An enum declaration is keyed by `quire.enum-declaration-node/v1` and its member by `quire.enum-member-node/v1`, never by `quire.structural-node/v1`: for QSpec's `enum-status` and `enum-status-ready` preimages in `node-identity-vectors.json`, the minted keys equal the recorded `sha256`. | Test (TC-413) |
| FR-092-AC-9 | `Rational[-9, 9; 1, 9]` and its base key to T10 and T9, `Decimal[-100000, 100000; 2, 2; nearest-even]` and its base to T12 and T11, and `tuple Pair(Int[0, 9], Int[0, 9]);` under (`a`, `u`) to D5. `record Opt { a: Int[0, 9]; b?: Int[0, 9]; }` keys to D3 and `record Opt { a: Int[0, 9]; b: Option<Int[0, 9]>; }` to D4, which differs. `rational(1, 2)` as a `Rational[-9, 9; 1, 9]` literal keys to L3, spelled `"1/2"`, and `rational(2, 4)` keys to L3 too. | Test (TC-413) |
| FR-092-AC-10 | `function m using v(x: Int[0, 9]): Boolean pure decreases(x) { true }` keys `x`'s parameter node to P4 and `m` to F3, whose body binds `decreases` to a `reference` to P4. | Test (TC-414) |
| FR-092-AC-11 | Each recursion group of the Recursion-group vectors checks and keys to its vectors' preimage bytes and keys: `f` to G4, G5 and G6 over L5, L6 and E11 to E13; `List` to G2 and G3; `Tree` to G7, G8 and G9, whose G9 preimage writes its `semantic_type` as `{term: "group_reference", ordinal: 1}`; and `ping` and `pong` to G10 to G15. The key function keys G1. Declaring `pong` before `ping` gives the same keys as declaring `ping` first. Each in-group application node's preimage has `recursion` `{size, ordinal}` and no `group` member, and each structural one's `recursion.group` equals its group's digest. In `function h using v(x: Int[0, 9]): Boolean pure decreases(x) { if x > 0 then h(x - 1) and h(x - 1) else true }`, the two calls are one node, and `h`'s group has `size` 4: `h`, the conditional, the conjunction and the call. | Test (TC-413) |
| FR-092-AC-12 | `record Point { x: Int[0, 9]; y: Int[0, 9]; }` under (`a`, `u`), declared once through a `CompositeDeclaration` whose key is 32 bytes of `0x11` and once through one whose key is 32 bytes of `0x22`, keys to D1 both times, and its checked type node's id is D1 both times. No preimage, checked-graph node or checked type node holds either supplied key. | Test (TC-413) |

## Dependencies

- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  O-04 and OQ-G (owner scope; builtin and anonymous type nodes carry no
  owner), §2 (the one RFC 8785 implementation), QC-18 and QC-24 (the QSpec
  requests this preimage and node shapes make).
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §2.2 E3 (node identity is minted at E3), §3 FB-13 (only `check` mints a
  `NodeKey`), §2.3 (the depth limit), and the M-6a OQ-7 ruling.
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
- QSpec FR-143 and FR-146: the recursive records and recursive functions
  that form recursion groups.
- `SourceOwner`'s `authority` and `identity` are those of the unit's
  `RawSourceRef` ([FR-001](FR-001-read-exact-source.md); ADR-013 §7 slice
  S-4b, QSL-233).
- QSpec. `quire.structural-node/v1`, the `value`/`parameter` semantic form
  and the function node body shape are QSL proposals (ADR-013 QC-24). QSL
  keys its nodes by them now and conforms to QSpec's arm once QSpec publishes
  one. The group order, the structural `recursion.group` member and the
  `group_reference` at a `semantic_type` position are QSL proposals too
  (QC-24). FR-322 and IR-242's reader derive an in-group ordinal from the
  writer's graph order, which QSL's emission sets to the group order
  (FR-093).

## Status

Specified under QSL-208; recursion groups and declared composite handles
specified under QSL-211. Implemented on the QSL-156 slice A4b branch,
pending merge: `check` keys every node by this requirement's preimages in
`qsl-semantics/src/check/node_key/` and `qsl-semantics/src/check/lowering.rs`,
recursion groups included (`node_key::group_keys`, which names members by
handle, and the lowering's drafts, keyed in dependency order), and gives a
declared composite's checked type node its key. TC-413 and TC-414 back AC-1
to AC-12 and CON-2 there, and the FR-146 recursive-function tests of
`qsl-eval/tests/it/total_functions.rs` pass. Keying a group is charged to the
checking stage's work budget. An option or collection type whose node is a
member of a recursion group is that member wherever it is named after the
group is keyed (FR-093's P15 and P16 are typed at G17).

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
