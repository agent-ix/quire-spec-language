---
id: FR-091
title: "Produce Value-family parsed forms from the CST and assemble them into package declarations"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-014
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-067
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: traces_to
---
# FR-091: Produce Value-family parsed forms from the CST and assemble them into package declarations

## Description

This requirement owns two pieces of the source-to-check path.

1. **The `Value` family's S2 production** (ADR-011 §7.3 M-3b for `Value`).
   The `Value` family form builder turns the S1 output of a complete-V1 unit
   into `Value`-family parsed forms, through the `forms` core's dispatch
   table (ADR-011 §2.1 E2 owner: "QSL `forms`; family form builders").
2. **The forms assembler.** The assembler turns the parsed forms of one unit
   into the `PackageDeclarations` value that `PackageDeclarations::check`
   consumes. It is the entry of E3 and belongs to the layer-3 `check` core
   (ADR-011 §2.1 E3 owner: QSL `check`).

With both in place, complete-V1 source reaches `PackageDeclarations::check`
through S1, S2 and E3.

The requirement carries testable criteria for decisions already taken:

- ADR-011 §2.1 E2 and E3 rows: the admitted input and owner of each edge.
- ADR-011 §2.2. E2 row: each form holds the span of its CST node; identity
  none; edition and the unit's profile, import and model selections
  carried; declared bounds and extents carried as syntax. E3 row: QSL keys
  the source map by occurrence key; bounds and extents typed. E9 row: the
  packet's occurrence key resolves to a nested span.
- ADR-011 OBS-007: S2 `forms` is the only producer of check-stage input from
  source.
- ADR-011 §2.3. E2 refuses with diagnostics and builds no partial form. E3
  refuses with every error diagnostic and yields no partial output. Every
  stage entry takes explicit limits, and recursive stages (S2 among them)
  bound their depth by an explicit limit.
- ADR-011 §1: name binding is a phase inside S3.
- ADR-011 §3 FB-01: no stage after S1 reads source text, CST or token text
  to recover meaning. FB-13: only `check` calls the kernel `NodeKey`
  constructor.
- ADR-011 §6.1. Layer 2 (`forms` core < family form builders) depends on
  "1, F, K", and the column is an exhaustive allow-list. A family module
  depends on its layer's core only. The `check` core is layer 3.
- ADR-012 §1 and §3: `Value` covers scalar and composite values, types,
  expressions and function application. Its parsing forms are literals,
  operators, `let`, `if`, calls, records, collections and function
  declarations. `StateModel` covers model population and lookup.
- ADR-012 §3 and §4.3. The parser composes families through one closed entry
  table keyed by the leading-token kind, and each entry makes one call into
  one family production. The one `Expression` enum is defined in the
  `forms` core. Each variant's owning family is the family whose hook its
  arm calls, and that family adds and removes the variant. `Deref` belongs
  to `StateModel` and `Pre` to `ProtocolClause`.
- ADR-013 O-11: the parsed-forms stage produces qualified names, and the
  check stage resolves them. Unresolved or ambiguous names refuse at check
  with a catalog code.
- ADR-013 R-07: a conversion never drops a declared value. A target that
  cannot represent a source value refuses.
- ADR-013 T-4 and T-5. A stage result is its output, a refusal with typed
  causes, a limit refusal (`LimitExceeded`: limit kind, configured bound,
  locus) or an internal fault. S0 to S2 name a location with
  `Locus::Region`.
- ADR-013 O-04: a source-owned node's identity preimage names its owner as
  `SourceOwner{authority, identity}`.
- QSpec (`ix://agent-ix/quire-specification`, cited by reference, never
  copied). `proposals/quire-v1/shared-grammar.md`: `using` selects a
  declared profile alias for every native declaration, with no default, and
  a floating type's `[rounding-mode]` is optional. FR-322 makes a floating
  type's rounding mode part of the type. FR-148 has the evaluator use the
  declared mode, and reads an omitted rounding spelling as `exact`. FR-145: `map` and `collect` are one
  operation. `proposals/checked-package-v2/README.md` and
  `node-identity-preimage.schema.json`: a nominal enum, dimension or unit
  node's preimage names its owner, and a source owner is
  `SourceOwner{kind: "source", authority, identity}`. The
  `quire.application-node/v1` preimage has no owner member. Aliases within
  one source unit are unique
  (`shared-grammar.md`).
  `proposals/quire-v1/definitions/native-diagnostics.md` revision
  `1-draft.6`: the catalog codes and causes this requirement names.
  `spec/functional/type-model/FR-141-evaluate-text-and-enumerations.md`:
  an enum declaration's key hashes
  `{version: "quire.enum-declaration-node/v1", owner, qualified_declaration,
  ordered, members}`, whose `members` are in declaration order for an
  `ordered enum` and sorted by case name otherwise; a member's key hashes
  `{version: "quire.enum-member-node/v1", declaration_node_id, case}`; the
  optional display string of a member enters neither preimage and never
  takes part in equality or ordering.
  `spec/functional/type-model/FR-142-evaluate-quantities-and-units.md`: a
  dimension's key hashes `{version: "quire.dimension-node/v1", owner,
  qualified_declaration, terms}` over base-dimension terms with nonzero
  exponents; a unit's hashes `{version: "quire.unit-node/v1", owner,
  qualified_declaration, dimension_node_id, target_unit_node_id, scale,
  offset}` with reduced rationals; an edge means `target_value = scale ×
  source_value + offset`; a dimension's units have one targetless root with
  scale one and offset zero, and no target cycle or cross-dimension target.
  `shared-grammar.md`: a predicate
  reads only its explicit parameters and pure lexical values. The
  checked-package-v2 `schema.json` `FunctionNode` admits the
  `semantic_form` values `pure_function`, `predicate` and
  `recursive_function`.
- The rulings on FR-091-OQ-1 to OQ-5 and OQ-7, recorded under Rulings.

The complete-V1 grammar is QSpec's `proposals/quire-v1/shared-grammar.md`,
which `qsl-cst` transcribes (`qsl-cst/src/grammar.rs`). It is the authority
for the syntax this requirement maps.

## Inputs

- The S1 output of one complete-V1 unit, `qsl_cst::ParsedSource`, whose CST
  is a `LosslessCst`.
- An explicit S2 limit set, which holds at least a nesting-depth bound.
- For the assembler: the parsed unit that S2 returned for that source, and
  the unit's source owner `SourceOwner{authority, identity}`, taken from the
  source reference the unit was compiled from. It is a required input:
  E3 has no default owner.

## Outputs

- From S2: one parsed unit. It holds the header's edition; the unit's
  profile, import and model selections as spelled, each with its alias,
  its definition reference and its span; and, in source order, one
  `Value`-family parsed form per declaration.
- From S2 on refusal: a refusal with a typed cause and the byte span it
  concerns, and no parsed unit.
- From S2 at a limit: a limit refusal that names the limit kind, the
  configured bound and the byte span where the limit was reached, and no
  parsed unit.
- From the assembler: a `PackageDeclarations` value.
- From the assembler on refusal: a refusal that carries every error found in
  the unit, each with a typed cause and the byte span of the form it
  concerns, and no `PackageDeclarations` value.

## Behavior

### S2 walks the unit's declarations and dispatches each one

The S2 entry admits a `ParsedSource` only when `ParsedSource::is_admissible()`
holds. A source whose CST carries a recovery refuses with cause
`RecoveringCst`. A source that carries a diagnostic and no recovery refuses
with a cause of its own, distinct from `RecoveringCst`, that holds the first
diagnostic's code. Such a source comes from `ParsedSource::prepend_diagnostic`,
for example a profile refusal. S2 returns no parsed unit in either case
(ADR-011 §2.3 E2).

For an admissible source, S2 visits the unit's `Declaration` nodes in source
order. The first significant token of each declaration selects that
declaration's entry in the `forms` core's dispatch table (ADR-012 §3). This
refines FR-067's "root construct's leading token". A complete unit's root
begins with `language`, so each declaration is a construct of its own. The
`Value` family has these entries:

| Leading token | Declaration production | `Value` parsed form |
|---|---|---|
| `function` | `FunctionDeclaration` | the `forms` `FunctionDeclaration` |
| `type` | `AliasDeclaration` | alias form |
| `record` | `RecordDeclaration` | record form |
| `tuple` | `TupleDeclaration` | tuple form |
| `enum` | `EnumDeclaration` | enum form, not ordered |
| `ordered` | `EnumDeclaration` | enum form, ordered |
| `predicate` | `Predicate` | the `forms` `FunctionDeclaration`, kind `Predicate` |
| `dimension` | `DimensionDeclaration` | dimension form |
| `unit` | `UnitDeclaration` | unit form |

Each entry makes exactly one call into the `Value` family's production
function (FR-067-CON-4, verified by inspection). When this table lands, the
`record` entry replaces the M-3a no-entry fixture token FR-067-AC-3's test
uses, so that test moves to a spelling no family claims.

When a declaration's leading token has no entry, S2 refuses the whole unit.
The cause is `NoDispatchEntry`, and the refusal names the token spelling and
the declaration's span. S2 returns no form for any declaration of that unit.

The `Value` family owns the `enum` (including `ordered enum`), `dimension`,
`unit` and `predicate` declaration productions. No other family has a
dispatch entry, production function or parsed-form type for them. Enum,
dimension and unit declarations are value types (QSpec FR-322 classes them
as `scalar_type` nodes), and a `predicate` is a function form with a
`Boolean` result. An `ordered enum` declaration's first significant token is
`ordered`, so `ordered` is its entry's leading token. Only `check` mints an
enum declaration's or member's, a dimension's or a unit's `NodeKey`
(FB-13).

### The unit's selections

The parsed unit carries the unit's profile, import and model selections in
source order, each as spelled: its alias, its definition reference and its
span. S2 resolves none of them (ADR-011 §2.2 E2 row).

### What a `Value` parsed form carries

Every `Value` parsed form carries the span of its `Declaration` CST node and
the unit's edition. It carries no semantic identity (FR-067-AC-2).

- **Function form.** This is the `forms` core's one `FunctionDeclaration`
  type. It carries the declared name; a `using` field holding the alias as
  written and its span; each
  parameter's name and type form, in order; the result type form; the
  `decreases` measure expression when one is written; and the body
  expression.
- **Alias form.** It carries the declared name and the aliased type form.
- **Record form.** It carries the declared name and each field's name, type
  form and optional marker `?`, in order.
- **Tuple form.** It carries the declared name and each element type form,
  in order.
- **Enum form.** It carries the declared name and its span; whether the
  declaration is `ordered`; and each member, in source order, with its case
  name, the span of that name, and its display string as spelled with the
  span of the string literal when `= "text"` is written.
- **Predicate form.** This is the `forms` `FunctionDeclaration` with
  declaration kind `Predicate`. Every other `FunctionDeclaration` has kind
  `Function`. It carries the declared name; the `using` field; each
  parameter's name and type form, in order; a result type form with head
  `Boolean` and the span of the `Boolean` token; no measure, because the
  `Predicate` production has no `decreases` clause; and the body
  expression, which the expression mapping below builds from the
  predicate's block exactly as it builds a function body.
- **Dimension form.** It carries the declared name and its span, and its
  terms, in source order, when `=` is written. Each term carries the
  operator that precedes it (none for the first term, else `*` or `/`), its
  qualified name as written with its span, and its exponent when `^` is
  written, as a `quire_exact::Integer` built from the signed-integer
  tokens, with its span. A form with no `=` has no terms and declares a
  base dimension.
- **Unit form.** It carries the declared name and its span; the dimension's
  qualified name after `:`, with its span; the scale, the exact number
  after `=`; the target's qualified name after `*`, with its span, when
  written; and the offset, the exact number after `+`, when written. An
  **exact number** carries its spelling kind (`rational` or `decimal`), its
  two parts as `quire_exact::Integer` values built from their tokens
  (numerator and denominator, or coefficient and scale), and its span.

A **type form** is a type reference carried as syntax: the head (a
constructor keyword or a qualified name) as written, every declared bound,
scale, rounding mode and text profile as spelled, the element type forms,
and its own span. It holds no `ValueType` and no `NodeKey` (ADR-011 §2.2 E2
row: "Declared bounds and extents carried as syntax"; identity "none"). Every
type reference in a `Value` form is a type form. That includes each
`FunctionDeclaration` parameter and result, the target of `convert<T>(e)`,
and the named types of `fold<A>`, `reduce<A>`, `count<N>` and `sum<N>`.
`forms` has no second function type that holds `ValueType`, and the
`allInstances<T>` target `T` is a type form too.

A floating type's `[mode]` is optional (QSpec `shared-grammar.md`), so
`qsl-cst` admits `Float32` and `Float64` with or without it. The type form of a floating type written without a
mode carries no mode, and E3 reads it as rounding mode `exact` (QSpec
FR-148).

### Every expression node carries its span

Each `Expression` node in a `Value` form carries the span of the CST node it
maps from. `(e)` maps to `e`'s node and carries `e`'s span. E3 keys the
source map by occurrence key and E9 resolves an occurrence key to a nested
span, while FB-01 forbids any stage after S1 from reading the CST. S2 is
therefore the only source of a nested node's region (ADR-011 §2.2 E2, E3
and E9 rows; §3 FB-01).

### Expression mapping

The `Value` production maps each CST expression construct to one
`Expression` variant:

| CST construct | `Expression` |
|---|---|
| `true`, `false` | `Boolean` |
| unsigned integer literal | `Integer` |
| `rational(n, d)` | `Rational(n, d)` |
| identifier, qualified name, enum value `E::m` | `Name`, spelled as written with `::` separators |
| `let x = v in b` | `Let` |
| `if c then a else b` | `If` |
| `implies` | `Binary{Implies}`, right-associative |
| `or`, `and` | `Binary{Or}`, `Binary{And}`, left-associative |
| `=`, `!=`, `<`, `<=`, `>`, `>=` | `Binary{Equal, NotEqual, Less, LessOrEqual, Greater, GreaterOrEqual}` |
| `+`, `-`, `*`, `/` | `Binary{Add, Subtract, Multiply, Divide}`, left-associative |
| `not e` | `Not` |
| unary `-e` | `Negate` |
| `e.f` | `Field` |
| `present(e)`, `value(e)` | `Present`, `Value` |
| `name(args)`, tuple value `Q(args)` | `Call`, with arguments in source order |
| record value `R { f: e, g: null }` | `Record`; a field whose whole value is `null` is `FieldInitializer::Null` |
| `sequence[..]`, `set[..]`, `bag[..]`, `orderedSet[..]` | `Collection` with kind `Sequence`, `Set`, `Bag`, `OrderedSet` |
| `map`, `collect` `(x in c: e)` | `Query` with `BinderQuery::Map` |
| `filter`, `flatMap`, `forall`, `exists` `(x in c: e)` | `Query` with `BinderQuery` `Filter`, `FlatMap`, `Forall`, `Exists` |
| `flatten(e)` | `Flatten` |
| `fold<A>(acc, x in c: s, identity: i)`, `reduce<A>(acc, x in c: s)` | `Accumulate`, with the identity when written |
| `count<N>(x in c: p)`, `sum<N>(x in c: e)` | `Count`, `Sum` |
| `size(e)` | `Size` |
| `contains(c, v)` | `Contains` |
| `convert<T>(e)` | `Convert`, with `T` carried as a type form |
| `deref(e)` | `Deref` |
| `pre(e)` | `Pre` |
| `allInstances<T>(e)` | `AllInstances`, with `T` carried as a type form |
| `(e)` | the mapping of `e` |

Grouping and precedence come from the CST's own nesting. The production
never re-derives them from token order.

`map` and `collect` are one operation (QSpec FR-145), so both spellings map
to `BinderQuery::Map`.

`Deref` and `AllInstances` belong to `StateModel`, and `Pre` to
`ProtocolClause` (ADR-012 §1, §3, §4.3). The variants are defined in the
`forms` core, so the `Value` production builds them where they are nested in
a `Value` declaration and names no other family module. E3 decides whether
the construct is valid there, and refuses it with the owning family's
catalogued cause, never `unsupported_construct`. In a function body or
measure:

- `pre(e)` refuses at check with `wrong_snapshot`/`forbidden-pre-read`,
  the `ProtocolClause` cause (FR-090).
- `deref(e)` whose value is not projected by `.f` refuses at check with
  `ill_typed`/`type-mismatch`: only `deref(r).f` is a value.
- `allInstances<T>(e)` whose `T` names no declaration of an admitted domain
  package refuses in the assembler with the unresolved-type-name error.

A construct that no family's variant represents refuses the whole unit with
cause `UnrepresentedConstruct`, naming the construct's CST production and
span. These constructs are text literals, `decimal(..)`, `float32(bits: ..)`
and `float64(bits: ..)`, `null` outside a record field value, `none`,
`self`, `result`, `div`, `rem`, `mod`, indexing `e[i]`, `size<T>(e)` and
`reaches(..)`. S2 returns no form for any declaration of that unit.
`reaches(..)` is a `StateModel` construct (model-graph reachability), and
its variant is `StateModel`'s to add (ADR-012 §4.3).

### S2 depth limit

S2 bounds its recursion by the nesting-depth bound `L` in its limit set, not
by the native stack (ADR-011 §2.3 Limits). Depth counts `Expression` nodes.
The root expression of a function body or measure is at depth 1, and each
child is one deeper than its parent. A body whose deepest node is at depth
`L` builds. A body with a node at depth `L + 1` gives a limit refusal that
names limit kind nesting depth, the bound `L`, and the span of the first node
at depth `L + 1` in source order. It is never a truncated form and never a
`NoDispatchEntry` or `UnrepresentedConstruct` refusal. The S2 bound applies
to sources that S1 admits. S1's delimiter-nesting ceiling
([NFR-001](../non-functional/NFR-001-bound-syntax-work.md) "Nesting level")
counts delimiter pairs, a different quantity, and is enforced independently.

### S2 layering

The `Value` family form builder is a module under `forms`, in the
`qsl-forms` crate (`qsl-forms/src/`). ADR-011 §6.1 allows layer 2 to depend
on layer 1 (`qsl-cst`), F (`qsl-foundation`) and K (`quire-exact`), and on
its own layer's `forms` core. `qsl-forms`'s `[dependencies]` name exactly
those three crates, so no shipped path in the builder can reach layers 3
to 6. The builder depends on no other family module and on nothing in
layers 3 to 6. No `Value` parsed
form, type form, parsed-unit selection or `Expression` variant in the
mapping table has a field of type `ValueType` or `NodeKey`.

`Expression::Integer` and `Expression::Rational` carry `quire_exact::Integer`
values, and `Expression::Collection` carries a `quire_exact::CollectionKind`.
The builder constructs those values from the literal and keyword tokens, so
E3 reads the value and never token text (ADR-011 §3 FB-01). K is the only
kernel edge the builder has, and it builds kernel values without calling a
kernel operation (ADR-013 OQ-A).

### The assembler builds `PackageDeclarations` from parsed forms

The assembler is a module of the layer-3 `check` core. Its non-test code
reads the parsed unit only. It reads no `qsl_cst` type, no source text and
no token text (FB-01). It fills `PackageDeclarations` as follows:

- `aliases`: one entry per alias form, in source order, holding the declared
  name and the resolved `ValueType`.
- `functions`: one entry per `FunctionDeclaration`, function or predicate,
  in source order, each name-callable with clause kind `Body` and keeping
  its declaration kind. Each entry carries a check-owned resolved
  signature: the parameter types and the result type that check resolved
  from the type forms, and the profile selection its `using` alias
  resolves to. The resolved signature is a `check` type, not a `forms`
  type.
- `types`: one composite declaration per record form and tuple form. Each
  has a declaration key that `check` mints (FB-13, ADR-013 O-04).
- `enums`: one `EnumBinding` per enum form, in source order, holding the
  declared name, the admitted `EnumDeclaration` and its admitted members
  in FR-141 canonical order (see "Enum declarations").
- `declared_type_spans`: the span of each record's, tuple's and enum's
  declared name, by that name (FR-096).
- `units`: the `UnitGraph` that `UnitGraph::admit` admits over every
  dimension and unit form of the unit (see "Dimension and unit
  declarations"). A unit with no dimension or unit form has the empty
  graph.
- `nominal_spans`: the span of each dimension's and unit's declared name,
  by its node key (FR-096).

### Enum declarations

For each enum form, the assembler:

1. Refuses a form that names one case more than once with a
   duplicate-enum-member error, naming the enum, the case and the span of
   each member that declares it. It admits nothing for that form.
2. Builds the `quire.enum-declaration-node/v1` preimage:
   - `owner`: the unit's owner as a source owner,
     `{kind: "source", authority, identity}`, the same `SourceOwner` that
     keys the unit's records, tuples and functions;
   - `qualified_declaration`: the declared name's `::`-separated segments,
     which for a declaration in source is the one segment of its name, the
     same segments the lowered node's `declaration.qualified_name` carries
     ([FR-092](FR-092-key-type-parameter-and-declared-nodes.md));
   - `ordered`: whether the form is `ordered`;
   - `members`: the case names in source order when `ordered`, and sorted
     by case name (byte order) otherwise.
3. Mints the declaration key through the one `check::node_key` function
   that mints a nominal node's key: the SHA-256 of the preimage's RFC 8785
   bytes, the digest `value::enumeration` computes.
4. Admits the declaration with `EnumDeclaration::admit(preimage, key,
   owners)`, where `owners` holds exactly the unit's source owner.
5. For each case in the preimage's `members` order, builds the
   `quire.enum-member-node/v1` preimage over the declaration key and the
   case, mints its key through the same `check::node_key` function, and
   admits it with `EnumDeclaration::admit_member`.

`value::enumeration` gives each preimage a constructor from its parts,
beside `from_json`. The constructor applies the schema checks `from_json`
applies and refuses with `NonCanonicalPreimage`. A member's display string
enters no preimage and no `PackageDeclarations` field (QSpec FR-141).

Steps 2 to 5 cannot refuse a form that step 1 admits: the grammar gives
every enum at least one member and every case an identifier, step 1 makes
the cases distinct, step 2 sorts an unordered enum, and step 3 mints the
key that step 4 recomputes. A refusal from a constructor, `admit` or
`admit_member` is therefore a nominal-admission fault, a broken invariant of
`check`, which the refusal carries with the declaration's span.

### Dimension and unit declarations

The rulings on FR-091-OQ-11 (Rulings) fix this section. Dimension names
and unit names are two namespaces of their own: a dimension term and a
unit's `:` name look up dimension forms only, and a unit's `*` target looks
up unit forms only. A type form looks up neither, so a type form that names
a dimension or a unit is an unresolved type name: complete-V1 source has no
quantity type reference (STD-113), and a source-declared unit has no use in
source.

The assembler refuses, reporting every error found:

- **Duplicate dimension or unit name.** A name declared by more than one
  dimension form, or by more than one unit form. The error names the name
  and the span of each declaration.
- **Unresolved dimension or unit name.** A dimension term or a unit's `:`
  name that names no dimension form, or a target that names no unit form.
  The error names the name and its span.
- **Dimension or unit cycle.** A dimension whose terms reach that
  dimension again through derived dimensions, or a unit whose target chain
  reaches that unit again. The error names the cycle's dependency edges.
- **Zero denominator.** An exact number `rational(n, 0)`. The error names
  its span.
- **Unit-graph topology.** The following each name the declarations they
  concern: a derived dimension whose normalized terms are empty; a unit
  whose scale is zero; a targetless unit whose scale is not one or whose
  offset is not zero (a non-identity root); a unit whose target has a
  different dimension; and a dimension whose units have two targetless
  roots.

With no error, the assembler builds each preimage, owner and
`qualified_declaration` as for an enum:

1. **Exact numbers.** `rational(n, d)` is the reduced rational `n / d`,
   with a positive denominator, and `decimal(c, s)` is the reduced rational
   `c / 10^s`. The reduced value is the value written (ADR-013 R-07); an
   unreduced spelling is not refused.
2. **Dimension terms.** A dimension form with no `=` has empty `terms`.
   For a derived dimension, the assembler normalizes: each term contributes
   its dimension's base map (a base dimension's map is itself with exponent
   one, a derived dimension's map is its own normalized terms) multiplied
   by its exponent (one when none is written), negated when the term
   follows `/`. It adds the contributions per base dimension, drops zero
   exponents, and sorts the terms ascending by base-dimension key. `terms`
   thus names base dimensions only, each once, as QSpec FR-142 and
   `UnitGraph::admit` require.
3. **Dimension keys.** It mints each base dimension's key, then each
   derived dimension's, over its `quire.dimension-node/v1` preimage,
   through the same `check::node_key` function that mints an enum's key.
4. **Unit preimages.** A unit's `quire.unit-node/v1` preimage has its
   dimension's key as `dimension_node_id`, its target's key as
   `target_unit_node_id` (`null` when no target is written), the reduced
   scale as `scale`, and the reduced offset, zero when no `+` is written,
   as `offset`. The unit's edge is `target_value = scale × source_value +
   offset` (FR-142).
5. **Unit keys.** It mints each unit's key after its target's, roots first,
   through the same function.
6. **Admission.** It calls `UnitGraph::admit` with every dimension and unit
   preimage and its key, and `owners` holding exactly the unit's source
   owner. The result is `units`.

`value::unit` gives `DimensionPreimage` and `UnitPreimage` a constructor
from their parts, with node keys for the nodes they reference, beside
`from_json`. The checks above leave `UnitGraph::admit` nothing to refuse,
so a refusal from a constructor or from `admit` is a nominal-admission
fault, carried with the span of the first dimension or unit form.

The assembler stops at the admitted `UnitGraph`. Keeping each dimension's
and unit's preimage through `Unit` and `UnitGraph`, and lowering the
`quire.dimension-node/v1` and `quire.unit-node/v1` nodes, is QSL-238's unit
half; lowering quantity type nodes is QSL-247.

**Dimension and unit key vectors.** Owner
`{kind: "source", authority: "a", identity: "u"}`, each
`qualified_declaration` the declared name. `dimension_node_id` and
`target_unit_node_id` are `{domain: "quire.checked-semantic-node/v1",
digest}`, and a rational is `{numerator, denominator}` in decimal strings.
The method reproduces QSpec's `dimension-velocity` (`7296d20b…da53`) and
`unit-degree-celsius` (`2ff8829f…bf40`) vectors.

| Vector | Source | Preimage content | Key |
|---|---|---|---|
| Q1 | `dimension Length;` | `terms: []` | `ff856699d6710bab9b9a893db4b7cf079f203258f543f02f990191d0e4834028` |
| Q2 | `dimension Time;` | `terms: []` | `e277e2dae0f9bae883f80b677af21b3ab95e9ae4c5406864d1dd307d0024f927` |
| Q3 | `dimension Speed = Length / Time;` | `terms: [{Q2, "-1"}, {Q1, "1"}]` | `3843d2d5c9906e126f8310cf148fa9b8f7366e0d5bdb762db7b8567e0968c10e` |
| Q4 | `dimension Accel = Speed / Time;` | `terms: [{Q2, "-2"}, {Q1, "1"}]` | `7d3bb56f6ff61b76783c608273c6c9b773376958449b3b1ace348ff582fa08d7` |
| Q5 | `dimension Temperature;` | `terms: []` | `3e5ab5ca1538a74d998648ec764c1dc6081acbc0dcdce0c8e81011fd7603730d` |
| Q6 | `unit m : Length = rational(1, 1);` | dimension Q1, target `null`, scale 1/1, offset 0/1 | `4c5aa30d2878d6197f7292492907082ccc857c60da88979b0a99eee5a5aecb0b` |
| Q7 | `unit km : Length = rational(2000, 2) * m;` | dimension Q1, target Q6, scale 1000/1, offset 0/1 | `700256858942ebde86f19abdfa12423cfc5f7e58d830ba3c5903c2d189cd2edd` |
| Q8 | `unit cm : Length = decimal(1, 2) * m;` | dimension Q1, target Q6, scale 1/100, offset 0/1 | `83b004fb06bbc9fbe56999ac263d2f3eafe87a25c3182c90acbf1131c0d260ee` |
| Q9 | `unit K : Temperature = rational(1, 1);` | dimension Q5, target `null`, scale 1/1, offset 0/1 | `5cc7dfb7cedf23b3dff8012718e3cea37586ed90c6b824affb6fafee10db49db` |
| Q10 | `unit C : Temperature = rational(1, 1) * K + decimal(27315, 2);` | dimension Q5, target Q9, scale 1/1, offset 5463/20 | `12b65987f369f8908be0d690d372e2b3fc3bae7623672a83a942a950663c67fa` |

### Predicates

A predicate is a function whose declaration kind is `Predicate`. The
assembler resolves its `using` alias, parameter type forms and body
exactly as it resolves a function's, and its result type form resolves to
`Boolean`. `check` checks and calls a predicate as it checks and calls a
function (FR-151's acyclic call graph included), and
[FR-092](FR-092-key-type-parameter-and-declared-nodes.md) keys its node with
`semantic_form` `predicate`.

The assembler also resolves every type form inside a body or measure: the
targets of `convert<T>` and `allInstances<T>` and the named types of
`fold<A>`, `reduce<A>`, `count<N>` and `sum<N>`. Their errors are
assembler errors like a signature's.

**Using aliases.** The assembler resolves each form's `using` alias to the
one profile selection of the unit that declares that alias, and records the
resolved selection in the declaration's resolved signature. An alias that
names no profile selection of the unit refuses with an undeclared-alias
error, naming the alias and the span of its `using` field. There is no
default profile. Profile, import and model aliases are unique within a
unit (QSpec `shared-grammar.md`): a unit that declares one alias twice
refuses with a duplicate-alias error naming the alias and both selection
spans.
Resolving a selection's definition reference against the library lock, and
building `ieee_profile` from it, are E3 steps outside this requirement.

**Node key owner.** `check` mints the node keys of record, tuple and
function declarations over the unit's `SourceOwner{authority, identity}`
(ADR-013 O-04). The source revision and digest are package lock evidence
and are not part of the preimage, so an unchanged declaration keeps its key
across revisions of its source. The owner is a required E3 input, and
`check` has no constant package identity:
`DEFAULT_PACKAGE_IDENTITY` does not exist. The record, tuple and function
nodes are keyed by the `quire.structural-node/v1` preimage, which carries the
owner ([FR-092](FR-092-key-type-parameter-and-declared-nodes.md)). An enum
declaration is keyed by QSpec's owner-bearing
`quire.enum-declaration-node/v1` preimage over the same owner, and its
members by `quire.enum-member-node/v1` over the declaration key. QSpec
publishes owner scope for nominal enum, dimension and unit nodes; the
structural-node preimage is a QSL proposal to QSpec (Dependencies).

**Enum key vectors.** Each key is the SHA-256 of the RFC 8785 bytes of the
preimage shown, owner `{kind: "source", authority: "a", identity: "u"}`
unless stated. The method reproduces QSpec's `enum-status` vector in
`node-identity-vectors.json` (`7928f1e1…1562`).

| Vector | Source | Preimage (other members as above) | Key |
|---|---|---|---|
| N1 | `ordered enum Status { READY, DONE }` | `qualified_declaration: ["Status"]`, `ordered: true`, `members: ["READY", "DONE"]` | `e5e7c1d5b51c76e84c928b616d266d45a570e8211404c302b47dbec62ae00d27` |
| N2 | member `READY` of N1 | `{version: "quire.enum-member-node/v1", declaration_node_id: {domain: "quire.checked-semantic-node/v1", digest: N1}, case: "READY"}` | `499f4989da1b6790fcda8c1e6e64ed04041d8cd304f336133c48f58c424d65ec` |
| N3 | `enum Color { RED, BLUE = "Blue" }` | `qualified_declaration: ["Color"]`, `ordered: false`, `members: ["BLUE", "RED"]` | `0757650a7514f2f86e2101a0d02e5055d1152c36fc7e01ea2dfc8eabc21dae62` |
| N4 | N1's source under owner identity `w` | N1's, with `identity: "w"` | `239e86987c45808a71cb0d23e7b25f8f70d6adb426288280c7eaf566f0f145e9` |

Type resolution is the E3 name-binding phase (ADR-011 §1; ADR-013 O-11).
The resolution `match` has one explicit arm per type-form head:

- a built-in constructor (`Boolean`, `Integer`, `Int[..]`, `Rational[..]`,
  `Decimal[..]`, `Text[..]`, `Option<..>`, `Sequence`/`Set`/`Bag`/
  `OrderedSet<..>[..]`) resolves to its `ValueType` over the declared bounds
  the type form carries;
- `Float32` and `Float64`, with or without a written mode, refuse with the
  floating-type error. A floating type's rounding mode is part of the type
  (QSpec FR-322, FR-148), and a `ValueType::Float` that holds only an
  `IeeeWidth` would drop it, which R-07 forbids. The refusal names the
  floating type (width and rounding mode, `exact` when none is written),
  the profile selection that the declaration's `using` alias names, and
  the type form's span;
- `Reference<Q>` resolves `Q` against the admitted domain packages (I1). With
  no admitted domain package, `Q` is an unresolved name;
- a qualified name that names exactly one alias form of the unit resolves to
  that alias's resolved type;
- a qualified name that names exactly one record or tuple form of the unit
  resolves to `ValueType::Composite` of that declaration's key;
- a qualified name that names exactly one enum form of the unit resolves to
  `ValueType::Enum` of that enum's `EnumBinding` shape.

An enum member written as an expression, `E::m`, is resolved by `check`
against `enums`, as FR-141 and the check stage already do: a case that `E`
does not declare refuses there with `missing_declaration`/`missing-name`.

The assembler refuses in these cases:

- **Unresolved type name.** A qualified name names no alias, record, tuple
  or enum form of the unit and nothing in an admitted domain package. The
  refusal names the name and the span of the type form.
- **Ambiguous type name.** A qualified name names more than one alias,
  record, tuple or enum form of the unit (ADR-013 O-11). The refusal names
  the name, the span of the referencing type form and the span of each
  candidate declaration.
- **Duplicate enum member.** As stated under "Enum declarations".
- **Nominal-admission fault.** As stated under "Enum declarations" and
  "Dimension and unit declarations".
- **Dimension and unit errors.** As stated under "Dimension and unit
  declarations".
- **Ill-formed scalar bounds.** The value type rejects a built-in
  constructor's declared bounds. Examples are an `Int` interval whose lower
  bound is above its upper bound, a `Rational` denominator interval that
  reaches below one, and a `Text` minimum above its maximum. The refusal
  carries that value type's own cause and the type form's span.
- **Floating type.** As stated above.
- **Alias cycle.** An alias's resolution reaches that alias again, through
  other aliases or through a type constructor's element type forms (for
  example `type A = Option<A>;`). The refusal names the aliases on the
  cycle, as the cycle's dependency edges.
- **Undeclared `using` alias.** As stated above.
- **Duplicate alias.** As stated above.

An `import` reaches the assembler only after spine `compile`'s S4 source
resolution has bound it to a supplied library ([FR-099](FR-099-compile-against-supplied-libraries.md),
ADR-015 D-1). An import no library supplies is refused there, at stage
`intake`, before assembly, and is never dropped from the package.

A refusal carries every error the assembler found in the unit, not only the
first (ADR-011 §2.3 E3). The assembler returns no `PackageDeclarations`
value alongside a refusal. The assembler runs before check mints any
occurrence key, so each assembler error names its location as a
`Locus::Region` over the type form's or declaration's span (ADR-013 T-5).

### Catalog codes

Each S2 and assembler cause has one exhaustive `catalog_code()` (ADR-013
O-17). The codes come from the catalog revision `1-draft.6`, written as
code or code/cause:

| Cause | Code |
|---|---|
| `RecoveringCst` | `invalid_syntax` |
| diagnosed source (diagnostic, no recovery) | the first diagnostic's own code |
| `NoDispatchEntry`, `UnrepresentedConstruct` | `unsupported_construct` |
| unresolved type name | `missing_declaration`/`missing-name` |
| ambiguous type name | `ambiguous_declaration` |
| ill-formed scalar bounds | `ill_typed` |
| floating type | `unknown_required_feature`/`unsupported-feature` |
| undeclared `using` alias | `missing_declaration`/`missing-selection` |
| duplicate alias | `ambiguous_declaration`/`ambiguous-name` |
| alias cycle | `invalid_package`/`definition-cycle` (catalog extension proposed, Dependencies) |
| duplicate enum member | `ambiguous_declaration`/`ambiguous-name` |
| nominal-admission fault | `runtime_invariant`/`established-invariant-broken` |
| duplicate dimension or unit name | `ambiguous_declaration`/`ambiguous-name` |
| unresolved dimension or unit name | `missing_declaration`/`missing-name` |
| dimension or unit cycle | `invalid_package`/`definition-cycle` |
| zero denominator | `undefined_expression`/`unproved-nonzero` |
| unit-graph topology | the STD-112 cause, once published (FR-091-OQ-12) |
| S2 nesting-depth limit | `stage_limit_exceeded`/`nesting-depth-exceeded` |

The floating-type row: no catalog code names a well-formed type that the
producer does not represent. `unsupported_construct` is the wrong code,
because the catalog reserves it for a form that the selected profile
prohibits, and a floating type is not prohibited.
`unknown_required_feature`/`unsupported-feature` is the nearest code. Its
meaning is a feature outside the consumer's declared support, and the
catalog states that a recognized name is not implementation support.

The alias-cycle row uses `definition-cycle`, whose payload is dependency
edges: the alias chain is those edges. The check stage uses the same code
for an FR-151 call-graph cycle (`qsl-semantics/src/check/refusal.rs`,
`CheckCause::DefinitionCycle`). The duplicate-alias row follows
`complete::package`, which refuses a duplicate selection alias as
`ambiguous-name`. The duplicate-enum-member row follows the record rule,
where a duplicate field name is `ambiguous-name`
(`DeclarationCause::DuplicateMember`): the catalog's `ambiguous-name`
retains the visible name and the distinct conflicting declarations and
loci. The nominal-admission-fault row follows the assembler's other broken
invariants (the declared-type handle, a domain package's object type).
The duplicate dimension or unit name row follows the duplicate-enum-member
row, the cycle row follows the alias-cycle row (FR-091-OQ-7), and the
zero-denominator row follows the check stage's own refusal of
`rational(n, 0)` in an expression (`Obligation::Nonzero`). The unit-graph
topology row has no catalog cause yet: `ill_typed` retains an operator
locus and expected and actual types, and `invalid_package`/`invalid-value`
a field path in a package document. STD-112 asks QSpec for the causes
(FR-091-OQ-12).

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-091-CON-1 | The `Value` family form builder and the assembler are the only producers of a `PackageDeclarations` value from complete-V1 source. The assembler's parsed-form input is S2 output. Its other E3 inputs are those ADR-011 §2.1's E3 row admits (admitted domain packages, library lock) and the unit's `SourceOwner`. | Design | Inspection |
| FR-091-CON-2 | The dispatch table, the `Value` production's expression match and the assembler's type-form resolution match have no `_` or catch-all arm (ADR-012 §5.1). | Design | Test (TC-398) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-091-AC-1 | S2 returns a parsed unit with exactly four forms, in the order alias, function, record and tuple, for an admissible S1 output whose header records edition `1-draft` and which holds one profile selection with alias `v` and then, in order, one `type`, one `function`, one `record` and one `tuple` declaration. Each form's span equals the span of its `Declaration` CST node, and the unit's edition reads `1-draft`. The unit's selections hold exactly one profile selection, with alias `v`, the definition reference as written and the span of its `Profile` CST node, and no import or model selection. | Test (TC-392) |
| FR-091-AC-2 | The `forms` `FunctionDeclaration` that S2 builds from `function inc using v(x: Int[0, 9], y: Digit): Int[0, 10] pure decreases(x) { x + 1 }` reads: name `inc`; `using` field alias `v`, with the span of that `v`; parameter `x`, a type form with head `Int` and bounds spelled `0` and `9`; parameter `y`, a type form with qualified-name head `Digit`; result, a type form with head `Int` and bounds `0` and `10`; measure `Name("x")`; body `Binary{Add, Name("x"), Integer(1)}`. No field of `FunctionDeclaration` or of the type form, and no accessor return type on either, is `ValueType` or `NodeKey`. | Test (TC-393) |
| FR-091-AC-3 | For each row of the expression-mapping table, a function body that holds that construct maps to the listed `Expression` variant, with operands in source order. `a or b and c` maps to `Or(a, And(b, c))`. `a - b - c` maps to `Subtract(Subtract(a, b), c)`. `a implies b implies c` maps to `Implies(a, Implies(b, c))`. `(a + b) * c` maps to `Multiply(Add(a, b), c)`. `map(x in c: x)` and `collect(x in c: x)` both map to `Query` with `BinderQuery::Map`. `allInstances<M::T>(p)` maps to `AllInstances` whose target is a type form with qualified-name head `M::T`. | Test (TC-394) |
| FR-091-AC-4 | S2 refuses a `ParsedSource` whose CST carries a recovery with cause `RecoveringCst`. It refuses an admissible parse, to which `prepend_diagnostic` has added one diagnostic, with the diagnosed-source cause holding that diagnostic's code. Neither refusal returns a parsed unit. | Test (TC-395) |
| FR-091-AC-5 | S2 refuses the whole unit with cause `NoDispatchEntry` for an admissible unit holding a valid `function` declaration followed by `invariant Positive using v on M::T at current { true }`. The refusal names the spelling `invariant` and that declaration's span, and S2 returns no form for the `function` declaration. | Test (TC-395) |
| FR-091-AC-6 | S2 builds a parsed unit with one form for an admissible unit whose only declaration is `enum Color { RED }`, `ordered enum Level { LOW }`, `predicate P using v(x: Boolean): Boolean { x }`, `dimension Length;` or, after `dimension Length;`, `unit m : Length = rational(1, 1);`. None of these refuses with `NoDispatchEntry`. No module under `forms` other than the `Value` family form builder names the `EnumDeclaration`, `EnumMember`, `Predicate`, `DimensionDeclaration` or `UnitDeclaration` CST production. | Test (TC-395) |
| FR-091-AC-7 | S2 refuses the whole unit with cause `UnrepresentedConstruct`, naming the construct's CST production and span and returning no form, for a function body that holds exactly one of: a text literal, `decimal(1, 2)`, `a mod b`, `xs[0]`, `none`, `reaches(a, b, M::R)`. | Test (TC-396) |
| FR-091-AC-8 | For each of `function f using v(x: Int[0, 9]): Boolean pure { B }` with body `B` equal to `deref(x)`, `allInstances<M::T>(x)` or `pre(x)`, and `function g using v(x: Int[0, 9]): Boolean pure decreases(pre(x)) { true }`, each in a unit with no admitted domain package, S2 returns a parsed unit that holds `Deref`, `AllInstances` or `Pre` where the construct is written. Check refuses `pre(x)`, in the body and in the measure, with `wrong_snapshot`/`forbidden-pre-read`, the `ProtocolClause` cause, and `deref(x)` with `ill_typed`/`type-mismatch`. The assembler refuses `allInstances<M::T>(x)` with an unresolved-type-name error naming `M::T` (`missing_declaration`/`missing-name`). None of these refusals has code `unsupported_construct`. | Test (TC-396) |
| FR-091-AC-9 | With S2 nesting-depth bound `L = 8`, S2 builds a function body `not`×7 `a`, whose deepest node is at depth 8. It refuses `not`×8 `a` with a limit refusal that names limit kind nesting depth, bound `8`, and the span of the node at depth 9, and returns no parsed unit. It refuses `not`×20 `a` in the same way. S1 admits all three sources, so each refusal comes from S2. | Test (TC-397) |
| FR-091-AC-10 | Every `Expression` node in a `Value` form carries the span of the CST node it maps from. For the body `if a then b else c + d`, the `If` node's span covers the whole body, the `Add` node's span covers `c + d`, and the `Name("d")` node's span covers `d`. For `(a + b) * c`, the `Add` node's span covers `a + b`, without the parentheses. | Test (TC-403) |
| FR-091-AC-11 | The `Value` family form builder module has `use` edges and inline paths only to the `forms` core, `qsl_cst`, `qsl_foundation` and `quire_exact`. It has none to another family module or to anything under `crate::check`, `crate::value` or `crate::model`. No field of a `Value` parsed form, of the type form, of a parsed-unit selection, or of an `Expression` variant in the mapping table has type `ValueType` or `NodeKey`. The dispatch `match`, the expression `match` and the assembler's resolution `match` have no `_` arm. | Test (TC-398) |
| FR-091-AC-12 | The assembler returns a `PackageDeclarations` value for the S2 output of source that declares `type Digit = Int[0, 9];`, then `function inc using v(x: Digit): Int[0, 10] pure { x + 1 }`, then `function two using v(): Int[0, 10] pure { inc(1) }`. Its `aliases` is `[("Digit", Int[0..9])]`. Its `functions` are `inc` and then `two`. `inc`'s check-owned resolved signature has parameter type `Int[0..9]` and result type `Int[0..10]`. | Test (TC-399) |
| FR-091-AC-13 | Calling `two` with no arguments through `CheckedPackage::call`, on the package that `PackageDeclarations::check` admits and links from the AC-12 assembler output, returns a completed outcome with integer value `2`. | Test (TC-399) |
| FR-091-AC-14 | The assembler returns one refusal holding two unresolved-type-name errors for a unit with `function f using v(x: Missing): Boolean pure { true }` and `function g using v(y: Absent): Boolean pure { true }`. One error names `Missing` with its type form's span, and the other names `Absent` with its type form's span. It returns no `PackageDeclarations`. | Test (TC-400) |
| FR-091-AC-15 | The assembler refuses with an ambiguous-type-name error for a unit with `type A = Int[0, 1];`, `type A = Int[0, 2];` and `function f using v(x: A): Boolean pure { true }`. The error names `A`, the span of `x`'s type form and the spans of both `type A` declarations. | Test (TC-400) |
| FR-091-AC-16 | The assembler returns one refusal with three errors for parameters typed `Int[9, 0]`, `Rational[0, 1; 0, 5]` and `Text[5, 1; nfc]`, one per function. Each error carries the owning value type's own rejection cause and its type form's span. It returns no `PackageDeclarations`. | Test (TC-400) |
| FR-091-AC-17 | The assembler refuses with an alias-cycle error, code `invalid_package`/`definition-cycle`, naming both `A` and `B` for `type A = B;` and `type B = A;`, and returns no `PackageDeclarations`. It refuses `type C = Option<C>;` with the same code, naming `C`. | Test (TC-400) |
| FR-091-AC-18 | For `record Point { x: Int[0, 9]; y: Int[0, 9]; }`, `tuple Pair(Int[0, 9], Int[0, 9]);` and `function px using v(p: Point): Int[0, 9] pure { p.x }`, assembled under source owner authority `a`, identity `u`, the assembler's `types` holds one record declaration `Point` and one tuple declaration `Pair`, each with a key that `check` minted over that owner. `px`'s resolved parameter type is `ValueType::Composite` of `Point`'s key, and `PackageDeclarations::check` admits the package. Assembling and checking the same source again under (`a`, `u`) gives the same `Point` and `Pair` keys and the same checked node id for `px`. Under (`a`, `w`) it gives a different key for each of the three. No item named `DEFAULT_PACKAGE_IDENTITY` exists under `src/`. | Test (TC-401) |
| FR-091-AC-19 | The assembler refuses a parameter typed `Float64[nearest-even]` with a floating-type error, code `unknown_required_feature`/`unsupported-feature`, that names width `Float64`, rounding mode `nearest-even`, the declaration's profile selection `v` and the type form's span. It refuses a parameter typed `Float64` with no mode with the same error naming rounding mode `exact`. It refuses a parameter typed `Reference<M::T>`, in a unit with no admitted domain package, with an unresolved-type-name error naming `M::T`. None of the three returns a `PackageDeclarations`. | Test (TC-405) |
| FR-091-AC-20 | The assembler module is under the layer-3 `check` core. Its non-test code has no `use` edge or inline path to `qsl_cst`. Its `#[cfg(test)]` code may reach `qsl_cst` only to run S1 and S2. | Test (TC-402) |
| FR-091-AC-21 | `catalog_code()` on each S2 and assembler cause returns the code in the Catalog codes table, and matches every cause with no `_` arm. The diagnosed-source cause returns its diagnostic's own code. The floating-type cause returns `unknown_required_feature`, the undeclared-alias cause `missing_declaration`, the duplicate-alias cause `ambiguous_declaration`, the alias-cycle cause `invalid_package`, the duplicate-enum-member cause `ambiguous_declaration`/`ambiguous-name`, the nominal-admission-fault cause `runtime_invariant`/`established-invariant-broken`, the duplicate dimension or unit name cause `ambiguous_declaration`/`ambiguous-name`, the unresolved dimension or unit name cause `missing_declaration`/`missing-name`, the dimension or unit cycle cause `invalid_package`/`definition-cycle`, and the zero-denominator cause `undefined_expression`/`unproved-nonzero`. The unit-graph topology cause returns the STD-112 cause once QSpec publishes it (FR-091-OQ-12). S2's nesting-depth limit refusal reports `stage_limit_exceeded`/`nesting-depth-exceeded`. | Test (TC-406) |
| FR-091-AC-22 | For a unit with one profile selection, alias `v`, and `function f using v(): Boolean pure { true }`, the assembler records `f`'s `using` alias as resolved to that selection. With `function g using w(): Boolean pure { true }` added, it returns one refusal holding an undeclared-alias error, code `missing_declaration`/`missing-selection`, that names `w` and the span of `g`'s `using` field, and no `PackageDeclarations`. A unit that declares two profile selections with alias `v` refuses with a duplicate-alias error, code `ambiguous_declaration`/`ambiguous-name`, naming `v` and both selection spans. | Test (TC-412) |
| FR-091-AC-23 | S1 admits a unit whose function parameter is typed `Float32` or `Float64` with no `[mode]`, and S2 builds that parameter's type form with head `Float32` or `Float64` and no rounding mode. | Test (TC-405) |
| FR-091-AC-24 | Spine `compile` of a unit that declares `import "test/units" version "2" digest "<64 lowercase hex>" as u;`, with no library supplied as `test/units`, refuses at stage `intake`, before assembly, with `missing_import`/`missing-selection` naming `test/units` at the import's identity string (FR-099, ADR-015 D-1), and emits no package. | Test (TC-405) |
| FR-091-AC-25 | S2 builds, for `ordered enum Level { LOW, HIGH = "High", }`, an enum form with name `Level` and the span of `Level`, ordered, and the members `LOW` with no display string and then `HIGH` with display string `"High"` and the span of that literal, each member with the span of its case name. For `enum Color { RED, BLUE }` it builds an enum form that is not ordered, with members `RED` and then `BLUE`, in source order. | Test (TC-480) |
| FR-091-AC-26 | The `FunctionDeclaration` that S2 builds from `predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }` reads: kind `Predicate`; name `Positive`; `using` field alias `v`, with the span of that `v`; parameter `x`, a type form with head `Int` and bounds spelled `0` and `9`; result, a type form with head `Boolean` and the span of the `Boolean` token; no measure; body `Binary{Greater, Name("x"), Integer(0)}`. The `FunctionDeclaration` that S2 builds from a `function` declaration has kind `Function`. | Test (TC-480) |
| FR-091-AC-27 | The assembler returns, for `ordered enum Status { READY, DONE }` and then `enum Color { RED, BLUE = "Blue" }` under source owner authority `a`, identity `u`, `enums` holding `Status` and then `Color`. `Status`'s declaration key is vector N1 and its members are `READY` then `DONE`, with `READY`'s member key N2. `Color`'s declaration key is N3 and its members are `BLUE` then `RED`. Assembling `enum Color { RED, BLUE }` under (`a`, `u`) also gives N3. Assembling the `Status` source under (`a`, `w`) gives N4. `declared_type_spans` holds the span of each of `Status` and `Color`. | Test (TC-481) |
| FR-091-AC-28 | On the package that `PackageDeclarations::check` admits and links from the assembler output for a unit with one profile selection `v`, `ordered enum Status { READY, DONE }`, `function isReady using v(s: Status): Boolean pure { s = Status::READY }`, `function ok using v(): Boolean pure { isReady(Status::READY) }` and `function later using v(): Boolean pure { Status::READY < Status::DONE }`, `isReady`'s resolved parameter type is `ValueType::Enum` of `Status`'s shape, and calling `ok` and `later` through `CheckedPackage::call` each return a completed outcome with value `true`. With `enum Color { RED, BLUE }` and `function bad using v(): Boolean pure { Color::RED < Color::BLUE }`, check refuses with `ill_typed`. With `function gone using v(): Boolean pure { Status::GONE = Status::READY }`, check refuses with `missing_declaration`/`missing-name` naming `Status::GONE`. | Test (TC-481) |
| FR-091-AC-29 | The assembler returns one refusal, and no `PackageDeclarations`, for a unit with `enum E { A, B, A }`, `enum F { X }`, `record F { y: Boolean; }`, `function f using v(p: F): Boolean pure { true }` and `function g using v(p: Shade): Boolean pure { true }`. It holds a duplicate-enum-member error, code `ambiguous_declaration`/`ambiguous-name`, naming `E`, `A` and the spans of both `A` members; an ambiguous-type-name error naming `F`, the span of `p`'s type form in `f` and the spans of both `F` declarations; and an unresolved-type-name error naming `Shade`. | Test (TC-481) |
| FR-091-AC-30 | The assembler returns, for `predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }` and then `function three using v(): Boolean pure { Positive(3) }`, `functions` holding `Positive`, kind `Predicate`, and then `three`, kind `Function`. `Positive`'s resolved signature has parameter type `Int[0..9]` and result type `Boolean`, and its `using` alias resolves to the selection `v`. Calling `three` through `CheckedPackage::call` on the checked package returns a completed outcome with value `true`. `predicate Q using w(x: Boolean): Boolean { x }`, in a unit with no selection `w`, refuses with an undeclared-alias error naming `w`. `predicate R using v(x: Boolean): Boolean { R(x) }` refuses at check with the FR-146 `missing-measure` obligation, as a recursive function written without `decreases` does. | Test (TC-481) |
| FR-091-AC-31 | S2 builds, for `dimension Length;`, a dimension form named `Length` with no terms. For `dimension Accel = Length * Time^-2 / Mass;` it builds terms `Length` (no operator, no exponent), `Time` (`*`, exponent `-2`) and `Mass` (`/`, no exponent), each with the span of its name and of its exponent when written. For `unit C : Temperature = rational(1, 1) * K + decimal(27315, 2);` it builds a unit form named `C` with dimension name `Temperature`, scale `rational` with parts `1` and `1`, target `K` and offset `decimal` with parts `27315` and `2`, each with its span. For `unit m : Length = rational(1, 1);` the unit form has no target and no offset. | Test (TC-482) |
| FR-091-AC-32 | The assembler returns, for vectors Q1 to Q10's sources in one unit under source owner (`a`, `u`), `units` holding dimensions keyed Q1 to Q5 and units keyed Q6 to Q10. Q3's and Q4's terms are the base terms the vectors list, so `Accel = Speed / Time` normalizes to `Length^1 Time^-2`. `km`'s edge has scale `1000` (from `rational(2000, 2)`), `cm`'s scale `1/100` (from `decimal(1, 2)`), and `C`'s offset `5463/20`; `C` is affine and `km` is not. `nominal_spans` holds the span of each declared name by its key. A unit with no dimension or unit form has the empty `units`. | Test (TC-483) |
| FR-091-AC-33 | A parameter typed `m`, in a unit that declares `dimension Length;` and `unit m : Length = rational(1, 1);`, refuses with an unresolved-type-name error naming `m`. | Test (TC-483) |
| FR-091-AC-34 | The assembler returns one refusal, and no `PackageDeclarations`, for a unit with `dimension Length;`, `dimension Mass;`, `dimension Mass;`, `dimension Area = Width^2;`, `dimension P = Q; dimension Q = P;`, `unit a : Length = rational(1, 0);`, `unit b : Length = rational(2, 1) * c;` and `unit c : Length = rational(1, 2) * b;`. It holds a duplicate-name error naming `Mass` and both spans (`ambiguous_declaration`/`ambiguous-name`); an unresolved-name error naming `Width` (`missing_declaration`/`missing-name`); a cycle error with edges `P`→`Q` and `Q`→`P`, and a cycle error with edges `b`→`c` and `c`→`b` (`invalid_package`/`definition-cycle`); and a zero-denominator error at `rational(1, 0)` (`undefined_expression`/`unproved-nonzero`). | Test (TC-483) |
| FR-091-AC-35 | The assembler refuses each of these units with one unit-graph topology error naming the declarations it concerns, and returns no `PackageDeclarations`: `dimension L; dimension N = L / L;` (empty normalized terms); `dimension L; unit r : L = rational(1, 1); unit z : L = rational(0, 1) * r;` (zero scale); `dimension L; unit r : L = rational(2, 1);` (non-identity root); `dimension L; dimension T; unit r : L = rational(1, 1); unit s : T = rational(1, 1) * r;` (cross-dimension target); and `dimension L; unit r : L = rational(1, 1); unit q : L = rational(1, 1);` (two roots). Each error's catalog code is the STD-112 cause (FR-091-OQ-12). | Test (TC-483) |

## Dependencies

- **Upstream:** [FR-067](FR-067-add-s2-forms-and-retire-seam-5.md) built the
  `forms` core: the dispatch table, `ParsedForm`'s span, edition and
  declared-extent accessors, and the `RecoveringCst` and `NoDispatchEntry`
  causes. This requirement adds the `Value` family's entries and production
  (M-3b), which FR-067-CON-2 leaves to the family, and refines FR-067's
  dispatch key to each declaration's leading token.
  [FR-068](FR-068-split-expression-checking-into-check-stage.md) placed
  `PackageDeclarations` and its `check` in the layer-3 `check` core.
- QSL-180's K5 slice replaces the `ValueType` fields of the `forms` types
  with a syntactic type form, and gives `PackageDeclarations` a check-owned
  resolved signature. This requirement states the same direction.
- [FR-065](FR-065-migrate-function-application-to-checked-family.md) takes
  a family's parsed forms as its input. This requirement produces them from
  source for the `Value` family.
- ADR-013 T-4's `Staged`, `StageFailure` and `LimitExceeded` are #213 S-5b's
  (QSL-160, [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)).
  T-5's `Locus` and O-12's `SourceRegion` are #213 S-4's. The
  criteria above observe a refusal's cause, limit kind, bound and byte span,
  whichever carrier holds them.
- The unit's `SourceOwner` is the `authority` and `identity` of the unit's
  `RawSourceRef`, which the caller names and S0 mints
  ([FR-001](FR-001-read-exact-source.md), ADR-013 §7 slice S-4b,
  QSL-233).
- Record, tuple and function keys over `SourceOwner` use QSL's
  `quire.structural-node/v1` preimage
  ([FR-092](FR-092-key-type-parameter-and-declared-nodes.md)). QSpec
  publishes an owner-bearing preimage for nominal enum, dimension and unit
  nodes only; the structural-node preimage is proposed to
  `ix://agent-ix/quire-specification` under ADR-013 QC-18 and QC-24.
- The enum entries rely on `value::enumeration` computing preimage digests
  and never minting a key (QSL-131 K4, ADR-011 FB-13), and on lowering
  building an admitted `EnumBinding`'s declaration and member nodes from
  its preimage (FR-092 rule 1, QSL-238's enum half).
- The dimension and unit hand-off: the assembler ends at an admitted
  `UnitGraph` in `PackageDeclarations::units`; keeping each unit's and
  dimension's preimage through `Unit` and `UnitGraph` and lowering the
  `quire.dimension-node/v1` and `quire.unit-node/v1` nodes is QSL-238's
  unit half; lowering quantity type nodes is QSL-247.
- STD-112 (QSpec catalog causes for unit-graph topology errors) and
  STD-113 (a quantity type reference in complete-V1 source).
- Resolving a selection's definition reference against the library lock is
  the M-4 lock evidence (QSL-6).
- FR-091-AC-17's code needs the QSpec catalog to name an alias cycle under
  `definition-cycle`. Revision `1-draft.6` scopes that cause to definition
  dependencies and to FR-151 dispatch cycles, and has no row for a type
  alias. The one-sentence catalog extension is proposed to
  `ix://agent-ix/quire-specification`.
- **Downstream:** the ADR-011 §5 stage driver calls S1, S2, the assembler
  and `check` in that order.

## Status

The `Value` slice is implemented (QSL-141). `qsl_forms::build_unit` walks a
unit's declarations and dispatches `function`, `type`, `record` and `tuple`
to the `Value` builder (`qsl-forms/src/value.rs`), which maps every row of
the expression table with each node's span, refuses the listed
unrepresented constructs, and bounds its depth. The forms
`FunctionDeclaration` carries its `using` alias. The assembler records each
resolved selection in `PackageDeclarations::function_selections`
(FR-091-AC-22). A bare `Float32` or `Float64` is admitted by S1 and builds a
type form with no rounding mode (FR-091-AC-23). No `match` over `Production`
in `qsl-forms` has a `_` arm (FR-091-AC-11). The assembler is
`PackageDeclarations::assemble` (`qsl-semantics/src/check/assemble.rs`). It
resolves aliases, records, tuples and function signatures, mints each
record's and tuple's handle over the unit's `SourceOwner`, records the span
of each declared type's name for FR-096, and reports every error it finds.

Remaining work:

- FB-13, QSL-275: the `enum`, `ordered` and `predicate` entries, the enum
  and predicate forms, the assembler's enum admission and the `predicate`
  function node (FR-091-AC-6 and AC-25 to AC-30, FR-092-AC-13) are
  specified and not implemented. Today every one of these declarations
  refuses `NoDispatchEntry`, and the only callers of
  `EnumDeclaration::admit` are tests.
- FB-13, QSL-275: the `dimension` and `unit` entries and forms and the
  assembler's `UnitGraph` admission (FR-091-AC-31 to AC-35) are specified
  and not implemented; today both refuse `NoDispatchEntry`, and the only
  callers of `UnitGraph::admit` are tests. AC-35's catalog code waits on
  STD-112 (FR-091-OQ-12). The hand-off to QSL-238 and QSL-247 is under
  Dependencies.
- The other families' parsed-form types are not built: the state family's
  under QSL-67, and the others under QSL-45, QSL-44, QSL-43, QSL-42,
  QSL-40, QSL-39 and QSL-36.
- `ValueType::Float` still holds only an `IeeeWidth`; the assembler refuses
  every floating type (FR-091-AC-19) rather than resolving one.

## Rulings

Lead rulings of 2026-09-22 on the open questions below. Each row is
decided; the last column names the fact that reopens it.

| Question | Ruling | Reason | Reopen when |
| --- | --- | --- | --- |
| FR-091-OQ-1, nested constructs | S2 builds the `Deref`, `Pre` and `AllInstances` variants that the `forms` core defines. The check stage refuses each with the owning family's catalogued cause, never `unsupported_construct`. `pre(e)` in a function body is `wrong_snapshot`/`forbidden-pre-read`, a `ProtocolClause` cause. | The catalog forbids a producer from choosing a broader listed code to discard a distinction it knows. FR-090 makes `forbidden-pre-read` a `ProtocolClause` cause. | ADR-012 §3's rule that a family owns its grammar productions is ruled to cover nested sub-expressions. |
| FR-091-OQ-1, declarations | `Value` owns the `enum`, `dimension`, `unit` and `predicate` declaration productions. The `enum`, `dimension` and `unit` entries need their key minting inside `check` (FB-13, QSL-131). | QSpec FR-322 classes enum, dimension and unit as `scalar_type` nodes. `predicate` is a function form with a `Boolean` result. | QSpec makes an enum a `union-decl` case rather than a `scalar_type`. |
| FR-091-OQ-2 | S2 carries each unit's profile, import and model selections, and each form keeps its `using` alias. E3 resolves every alias to a declared profile selection, or refuses with `missing_declaration`/`missing-selection`. ADR-011 §2.2's E2 Version cell reads "Edition and the unit's profile, import and model selections carried". Resolution against the library lock is the M-4 lock evidence (QSL-6). The forms `FunctionDeclaration` has a `using` field. | ADR-011 OBS-007 makes S2 the only source of check-stage input from source. QSpec requires `using` to name a declared alias, with no default. | QSpec makes the compile request's input inventory, not each unit, the source of selections. S2 still carries each unit's selections for the driver to compare, and only the lock assembly moves. |
| FR-091-OQ-3 | Record, tuple and function node keys are minted over `SourceOwner{authority, identity}`. ADR-013 O-04 and ADR-012 §2 state the owner reading of QC-18. `check` has no `DEFAULT_PACKAGE_IDENTITY`. | Neither the source grammar nor the v2 wire carries a package name, so replay could not rebuild a `name@version` key. QSpec's `proposals/checked-package-v2/README.md` publishes the owner reading for nominal enum, dimension and unit nodes; for `composite_type` and `function` nodes QSL keys by its proposed `quire.structural-node/v1` preimage, which carries the owner (FR-092). | QSpec adds a package name to source or to the v2 wire, or a declaration must keep its key when it moves between the units of one package. |
| FR-091-OQ-4 | `ValueType::Float` carries the rounding mode, in `quire-exact` and in QSL, and the evaluator applies it. The assembler's floating-type refusal (FR-091-AC-19) has code `unknown_required_feature`/`unsupported-feature`: no catalog code names a well-formed type the producer does not represent, and `unsupported_construct` is reserved for profile-prohibited forms. A bare `Float32` or `Float64` is strict `exact`, and `qsl-cst` accepts it. | QSpec FR-322 makes the rounding mode part of the type, and FR-148 requires the evaluator to use it. | QSpec FR-322 moves float rounding off `type_pinned_modes`. |
| FR-091-OQ-5 | `collect` maps to `Query{Map}`. `reaches` is a `StateModel` construct with no variant; it and the other listed constructs are refused with `UnrepresentedConstruct`. A variant for one of them comes with its checker and evaluator arms. `div`/`rem` depend on the unit's div/rem selection (FR-091-OQ-2), float literals on a mode-carrying `ValueType::Float` (FR-091-OQ-4), and `e[i]` on QSpec semantics. | QSpec FR-145 makes `map` and `collect` one operation. QSL FR-008-AC-20's duplicate-output `collect` refusal belongs to the native lane, not to complete-V1; QSpec FR-008-AC-5 refuses `collect` only where `quire.state.queries/v1` is selected without `quire.value.complete/v1`. | M-6a must compile existing native programs that use the refused constructs. |
| FR-091-OQ-7 | An alias cycle is `invalid_package`/`definition-cycle` at the check stage. | The check stage uses the same code for an FR-151 dispatch call-graph cycle (`qsl-semantics/src/check/refusal.rs`, `CheckCause::DefinitionCycle`), and the cause's payload is dependency edges. | QSpec declines the catalog extension and keeps `definition-cycle` to `DefinitionRef` closures. |
| FR-091-OQ-11 | Source dimensions and units (team lead ruling, 2026-09-26). (a) Dimension terms are normalized: derived terms expand to base terms, repeated exponents add, zeros drop, terms sort by key, and a derived dimension that cancels to no terms refuses. (b) `rational`/`decimal` numbers are reduced, not refused; a zero denominator refuses as `undefined_expression`/`unproved-nonzero`. (c) A dimension or unit cycle is `invalid_package`/`definition-cycle` and an unresolved name `missing_declaration`/`missing-name`; the topology errors take the STD-112 causes (FR-091-OQ-12). (d) Units are admitted with no source use until STD-113 settles a quantity type reference. | (a) matches FR-142's normalized base-dimension map, and a derived term is how acceleration is written from speed. (b) the reduced rational is the value written (ADR-013 R-07). (c) follows FR-091-OQ-7 and the catalog's `missing-name`; no existing cause fits the topology errors. (d) the complete-V1 `type-ref` has no quantity form, and reading a unit's name as a type would invent syntax meaning. | QSpec publishes STD-112's causes or STD-113's quantity type reference. |

## Open Questions

- **FR-091-OQ-9: Do type-form bounds carry kernel values or spelled text?**
  A type form carries every declared bound "as spelled", and the ADR-011
  §2.2 E2 row carries declared bounds and extents "as syntax". Expression
  literals carry `quire_exact::Integer` values so that E3 never reads token
  text (ADR-011 §3 FB-01; ADR-013 OQ-A). Resolving a spelled bound at E3
  reads token text in the same way. FR-091-AC-2 and AC-16 observe bounds as
  spelled.
- **FR-091-OQ-10: Is `unsupported_construct` the right code for
  `NoDispatchEntry` and `UnrepresentedConstruct`?** The catalog's
  `unsupported_construct` retains "the exact selected profile that prohibits"
  the form. A declaration with no dispatch entry and a construct with no
  `Expression` variant are forms the complete-V1 profile admits and the
  producer does not represent, which is the reason the floating-type
  cause is `unknown_required_feature`/`unsupported-feature`. The Catalog
  codes table keeps `unsupported_construct` for these two causes.
- **FR-091-OQ-12: Which catalog causes code the unit-graph topology
  errors?** STD-112 asks QSpec for a cause for each of: a unit whose target
  has a different dimension, a dimension with two roots, a zero scale and a
  non-identity root. A derived dimension whose normalized terms are empty
  needs one as well, and STD-112 does not list it yet. Until QSpec answers,
  FR-091-AC-35's code is unbacked. A missing root cannot arise from source
  that passes the other checks.
