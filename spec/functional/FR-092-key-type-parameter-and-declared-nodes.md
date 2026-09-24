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
- The node's recursion group, when it is in one (FR-322 `recursion_group`).

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
| `owner` | Present exactly when `declaration` is not `null`: the declaring node's QSpec `Owner` (ADR-013 O-04). A `Value` declaration compiled from source carries its unit's `SourceOwner`, `{kind: "source", authority, identity}`. Absent otherwise. |
| `node_tag` | The node's FR-322 `node_tag`. |
| `semantic_form` | The node's FR-322 `semantic_form`. |
| `semantic_type` | The `NodeId` (`{domain: "quire.checked-semantic-node/v1", digest}`) of the node's semantic type, or `null` when the node's semantic type is the node itself. |
| `declaration` | `{qualified_name}` for a node with a `declaration` source occurrence (FR-322 `declaration`), else `null`. |
| `recursion` | `null`, or `{size, ordinal}` of the node in its recursion group, as `quire.application-node/v1` defines it. |
| `body` | The node's FR-322 body term. A `reference` to a member of the node's own recursion group is written `{term: "group_reference", ordinal}`, as `quire.application-node/v1` writes it. |

The members, the `group_reference` rule and the term shapes are those of
`quire.application-node/v1`, with three differences: the `version`, the
`owner` member of a declared node, and a `null` `semantic_type` for a
self-typed node. A `scalar_type` and a `composite_type` node are their own
semantic type, so this `null` keeps their preimage acyclic.

A number never appears in a preimage as a JSON number except `recursion`'s
`size` and `ordinal`, a `group_reference` `ordinal` and an `operation.member`
`position`. A literal's value
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
limit (`CheckCause::ResourceExhausted`), and yields no key.

A declared node's qualified name, a binding name and a `semantic_form` are
never empty: S2 builds identifiers from non-empty tokens. A preimage that
would hold an empty one is an internal fault (ADR-013 T-4), and yields no
key.

### Recursion groups

A node's recursion group is its strongly connected component in the graph of
`reference` edges, when that component has more than one node or a
self-edge; its members' order is FR-322's graph order. Two in-group nodes of
different groups can have equal preimages, because a `group_reference`
names a position, not a node: two recursive functions `f` and `g` with the
same body shape give their in-group expression nodes one preimage.
`check` SHALL refuse a package in which two distinct nodes have equal
preimages and are not the same node, with `unknown_required_feature`/
`unsupported-feature` naming both nodes' source regions, and yield no key for
either (FR-092-OQ-1).

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

A type alias introduces no type node. A type form that names an alias lowers
to the node of the alias's resolved type (FR-091 resolves the alias to that
type).

An enum type is its QSpec nominal node (rule 1). A quantity type is its unit's
nominal node (rule 1). `Reference<T>` and `Population<T>[N]` type nodes are
the `StateModel` family's.

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
| FR-092-AC-7 | With the check stage's depth limit set to 4 (`CheckingLimits`), a parameter typed `Option<Option<Option<Option<Boolean>>>>` is keyed, and one typed with five nested `Option`s refuses with `resource_exhausted`/`insufficient-next-charge` naming the depth limit, and yields no key. Two recursive functions `function f using v(x: Int[0, 9]): Boolean pure decreases(x) { if x = 0 then true else f(0) }` and the same body under the name `g`, in one unit, refuse with `unknown_required_feature`/`unsupported-feature` naming both functions' in-group node regions. | Test (TC-413) |
| FR-092-AC-8 | An enum declaration is keyed by `quire.enum-declaration-node/v1` and its member by `quire.enum-member-node/v1`, never by `quire.structural-node/v1`: for QSpec's `enum-status` and `enum-status-ready` preimages in `node-identity-vectors.json`, the minted keys equal the recorded `sha256`. | Test (TC-413) |
| FR-092-AC-9 | `Rational[-9, 9; 1, 9]` and its base key to T10 and T9, `Decimal[-100000, 100000; 2, 2; nearest-even]` and its base to T12 and T11, and `tuple Pair(Int[0, 9], Int[0, 9]);` under (`a`, `u`) to D5. `record Opt { a: Int[0, 9]; b?: Int[0, 9]; }` keys to D3 and `record Opt { a: Int[0, 9]; b: Option<Int[0, 9]>; }` to D4, which differs. `rational(1, 2)` as a `Rational[-9, 9; 1, 9]` literal keys to L3, spelled `"1/2"`, and `rational(2, 4)` keys to L3 too. | Test (TC-413) |
| FR-092-AC-10 | `function m using v(x: Int[0, 9]): Boolean pure decreases(x) { true }` keys `x`'s parameter node to P4 and `m` to F3, whose body binds `decreases` to a `reference` to P4. | Test (TC-414) |

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
- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md)
  §5.1: no catch-all arm in a family dispatch.
- `SourceOwner`'s `authority` on QSL's source identity is #213 S-4
  (QSL-159).
- QSpec. `quire.structural-node/v1`, the `value`/`parameter` semantic form
  and the function node body shape are QSL proposals (ADR-013 QC-24). QSL
  keys its nodes by them now and conforms to QSpec's arm once QSpec publishes
  one. A recursion group's graph order is FR-322's; the IR reader's recursion
  preimage is IR-242.

## Status

Specified under QSL-208. Not implemented. `check` mints each function identity
from a length-prefixed preimage in `qsl-semantics/src/check/family.rs` that
includes the package identity, and builds no type, parameter or value node
key. QSL-156 slice A4b implements this requirement (see FR-093 for the
ownership decision). TC-413 and TC-414 are planned.

## Open Questions

- **FR-092-OQ-1: How does an in-group node's key tell two recursion groups
  apart?** FR-322's `group_reference` names a member by its ordinal, so an
  expression node inside the group of `f` and the matching node inside the
  group of `g` have equal preimages when `f` and `g` have the same body
  shape, and a self-referential record's anonymous `Option<R>` has the same
  preimage for every such record. The `quire.application-node/v1` preimage
  has no member that could carry the group's declared members. Until QSpec
  decides it with the recursion preimage (IR-242, ADR-013 QC-24), `check`
  refuses the collision (Recursion groups) rather than merge the nodes.
