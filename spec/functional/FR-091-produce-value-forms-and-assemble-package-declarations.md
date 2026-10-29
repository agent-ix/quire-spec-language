---
id: FR-091
title: "Produce Value-family parsed forms from the CST and assemble them into package declarations"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-014
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-142
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
  - target: ix://agent-ix/quire-specification/FR-147
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-149
    type: references
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
  stage entry takes explicit resource limits. ADR-030 D-1 removes depth as a
  limit: S2 walks over explicit heap stacks and is bounded by S1's node
  limit (FR-257).
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
  `proposals/quire-v1/definitions/native-diagnostics.md`: the catalog codes and causes this requirement names.
  `spec/functional/expressions/FR-147-evaluate-integer-division-domains.md`:
  `div` and `rem` use exactly one selected integer-division law under
  `quire.value.complete/v1`; `mod` is independently Euclidean; absent facet
  and an integer-division reading of `/` remain refused; FR-149 permits exact
  integer-to-rational promotion for rational `/` with an expected rational type
  only under `quire.value.complete/v1`.
  `spec/functional/type-model/FR-141-evaluate-text-and-enumerations.md` owns
  enum declaration/member identity, display-string exclusion and enum order;
  QSL maps those identities into `VariantId` and the kernel enum shape below.
  `spec/functional/type-model/FR-142-evaluate-quantities-and-units.md` owns
  dimension and unit identity and unit-graph semantics; QSL's lowering and
  kernel mapping are specified here. FR-142-AC-8 owns the compound-unit
  identity vectors used by the QSL conformance checks.
  `shared-grammar.md`: a predicate
  reads only its explicit parameters and pure lexical values. The
  checked-package-v2 `schema.json` `FunctionNode` admits the
  `semantic_form` values `pure_function`, `predicate` and
  `recursive_function`.
- The rulings on FR-091-OQ-1 to OQ-5 and OQ-7, recorded under Rulings.

The complete-V1 grammar is QSpec's `proposals/quire-v1/shared-grammar.md`,
which `qsl-cst` transcribes (`qsl-cst/src/grammar.rs`). It is the authority
for the syntax this requirement maps.

A domain package's native `Float32`/`Float64` model field resolves to
`ValueType::Float` of its width with the strict `exact` rounding mode, as a
function parameter, result and body type of that spelling does; a floating
type is admitted as a parameter, result, body and model field type.

## Inputs

- The S1 output of one complete-V1 unit, `qsl_cst::ParsedSource`, whose CST
  is a `LosslessCst`.
- For the assembler: the parsed unit that S2 returned for each source unit of
  the package, in request order (FR-480), each with its source owner
  `SourceOwner{authority, identity}`, taken from the source reference the
  unit was compiled from. The owner is a required input: E3 has no default
  owner. A package of one unit passes one parsed unit.
- For the assembler: an explicit limit set holding the decimal-scale bound
  (default 4096; "Dimension and unit declarations").

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
| `dimension` | `DimensionDeclaration` or `DimensionEquivalence` | dimension form, or dimension-equivalence form when a `~` follows the first name |
| `unit` | `UnitDeclaration` | unit form |

Each entry makes exactly one call into the `Value` family's production
function (FR-067-CON-4, verified by inspection). When this table lands, the
`record` entry replaces the M-3a no-entry fixture token FR-067-AC-3's test
uses, so that test moves to a spelling no family claims.

When a declaration's leading token has no entry, S2 refuses the whole unit.
The cause is `NoDispatchEntry`, and the refusal names the token spelling and
the declaration's span. S2 returns no form for any declaration of that unit.

The `Value` family owns the `enum` (including `ordered enum`), `dimension`,
`unit` and `predicate` declaration productions. The `dimension` production
reads the token and a qualified name: a `~` after that name makes the
declaration a dimension-equivalence form (the shared grammar's
`dimension-equivalence`, `dimension R::Time ~ F::Time by 1 R::s = 1000 F::ms;`,
whose `by` clause states the ratio between the two canonical roots), and otherwise the
name is the single-segment identifier of a dimension form. No other family has a
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
  name, the span of that name, and its display string as spelled, quotes
  included, with the span of the string literal when `= "text"` is written.
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
- **Dimension-equivalence form.** It carries the two dimension names as
  written, in source order, each with its span; the two **ratio terms** of the
  `by` clause in source order, each holding a ratio number, a unit name and the
  term's span; and the span of the declaration. A ratio number is an unsigned
  integer, `rational(n, d)` or `decimal(c, s)` and carries its spelling kind
  (`integer`, `rational` or `decimal`), its two parts as `quire_exact::Integer`
  values built from their tokens (an integer is its value over 1) and its
  span. A unit name is a qualified name as written, with its span. The form
  resolves, reorders and reduces nothing: `dimension R::Time ~ F::Time by 1
  R::s = 1 F::s;` and `dimension F::Time ~ R::Time by 1 F::s = 1 R::s;` are two
  forms.
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
| `decimal(c, s)` | `DecimalValue{coefficient, scale}`, the signed integer `c` and the unsigned integer `s` exactly as written |
| `float32(bits: 0xHHHHHHHH)`, `float64(bits: 0xHHHHHHHHHHHHHHHH)` | `FloatBits{width, bits}`, the width `32` or `64` and the bit pattern as an unsigned integer |
| quoted text `"..."` | `String`, holding the decoded Unicode scalar sequence |
| identifier, qualified name, enum value `E::m` | `Name`, spelled as written with `::` separators |
| `let x = v in b` | `Let` |
| `if c then a else b` | `If` |
| `implies` | `Binary{Implies}`, right-associative |
| `or`, `and` | `Binary{Or}`, `Binary{And}`, left-associative |
| `=`, `!=`, `<`, `<=`, `>`, `>=` | `Binary{Equal, NotEqual, Less, LessOrEqual, Greater, GreaterOrEqual}` |
| `+`, `-`, `*`, `/` | `Binary{Add, Subtract, Multiply, Divide}`, left-associative |
| integer `div`, `rem`, `mod` | `Binary{IntegerDiv, IntegerRem, IntegerMod}` respectively, left-associative at product precedence |
| `not e` | `Not` |
| unary `-e` | `Negate` |
| `e.f` | `Field` |
| `present(e)`, `value(e)` | `Present`, `Value` |
| `name(args)`, tuple value `Q(args)` | `Call`, with arguments in source order; `UUID("...")`, `Timestamp(n)`, `Bytes("...")` and `JsonObject("...")` are such calls, and check resolves them ("Native scalar literals") |
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

S2 preserves `div`, `rem` and `mod` as distinct operator forms with their CST
spans even if the containing expression is unreachable. It does not decide
whether the declaration's profile admits them. At check, the declaration's
resolved `using` selection admits them only with `quire.value.complete/v1`;
otherwise each receives a located `unsupported_construct`. The checked
package's exactly-one integer-division definition selects the `div`/`rem`
law, while `mod` uses the independent Euclidean remainder. `/` never denotes
integer division; rational `/` follows QSpec FR-044, with FR-149's integer-operand
promotion only under `quire.value.complete/v1`. Evaluation follows QSpec
FR-147 and cannot infer the division law from the spelling or a backend.

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
span. These constructs are `null` outside a record field value, `none`,
`self`, `result`, indexing `e[i]`, `size<T>(e)` and
`reaches(..)`. S2 returns no form for any declaration of that unit.
`reaches(..)` is a `StateModel` construct (model-graph reachability), and
its variant is `StateModel`'s to add (ADR-012 §4.3).

### S2 depth

S2 builds a form for every body S1 admits, whatever its depth. It builds at
most one form node per CST node, so S1's node limit
([NFR-001](../non-functional/NFR-001-bound-syntax-work.md)) bounds it, and
S2 takes no limit set of its own. S2 walks the CST and builds `Expression`
over explicit heap stacks
([FR-257](FR-257-build-forms-at-any-depth.md), ADR-030 D-4.2).

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
  dimension, dimension-equivalence and unit form of the unit (see "Dimension
  and unit declarations"). A unit with no dimension, dimension-equivalence or
  unit form has the empty graph.
- `nominal_spans`: the span of each dimension's and unit's declared name
  and of each dimension equivalence's declaration, by its node key (FR-096).

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
3. Mints the declaration key with `check::node_key::nominal_key(&impl
   NodeIdentityPreimage) -> Result<NodeKey, InvalidSemanticGraph>`, the one function that mints a nominal
   node's key: the structural key, the preimage value itself (its RFC 8785
   byte string), compared by equality and used directly as a map key; there
   is no hash. It is separate from
   `check::node_key::node_key`, which keys structural and application
   nodes and never a nominal preimage.
4. Admits the declaration with `EnumDeclaration::admit(preimage, key,
   owners)`, where `owners` holds exactly the unit's source owner.
5. For each case in the preimage's `members` order, builds the
   `quire.enum-member-node/v1` preimage over the declaration key and the
   case, mints its key with `nominal_key`, and
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

The rulings on FR-091-OQ-11 and FR-091-OQ-14 (Rulings) fix this section.
Dimension names and unit names are two namespaces of their own: a dimension
term, a unit's `:` name and each dimension name of a dimension equivalence look up
dimension forms only (a bare name) or the dimensions of a selected domain
package (`M::Name`), and a unit's `*` target and each unit name of a dimension
equivalence's `by` clause look up unit forms only (a bare name) or the units of a
selected domain package (`M::name`). A type form looks up neither, except `Quantity<u>`, which looks up the unit
namespace (QSpec FR-142 "Quantity type reference"; FR-091-AC-44): any other type form that names a dimension
or a unit is an unresolved type name. A source-declared dimension and unit are
used by name outside a type form: FR-230 reads them as the time dimension and
its time units.

The assembler refuses, reporting every error it finds among the following.
It runs the unit-graph topology check below only when none of them is
found, so a topology error is never reported beside a duplicate-name,
unresolved-name, zero-denominator or limit error:

- **Duplicate dimension or unit name.** A name declared by more than one
  dimension form, or by more than one unit form. The error names the name
  and the span of each declaration.
- **Unresolved unit dimension name.** A unit's `:` name, or a dimension
  term, that is a bare name no dimension form of the unit declares. A bare
  name reaches the unit's own dimension forms only, so a bare name that an
  admitted domain package declares (FR-056, QSpec FR-154) is unresolved
  here. The error names the name and its span. A dimension of an admitted
  domain package is reached as `M::Name` only. A qualified
  name whose first segment is a model alias of the unit (`M::Time`, `M::s`)
  is not looked up here: it names a dimension or unit of the selected domain
  package, which S3 resolves (QSpec FR-142 "Namespaces"). A source dimension
  and a package dimension, or the dimensions of two packages, that carry one
  name are distinct dimensions, so the assembler reports neither an unresolved
  name for a qualified name nor a duplicate against a source dimension or unit
  of the same last segment.
- **Zero denominator.** An exact number `rational(n, 0)`. The error names
  its span.
- **Decimal scale limit.** An exact number `decimal(c, s)` whose `s` is
  above the assembler's decimal-scale bound. The assembler takes this bound
  as an explicit limit (ADR-011 §2.3), default 4096, because forming
  `10^s` costs work that grows with `s`. The limit refusal names limit kind
  work budget, the bound, the actual `s` and the exact number's span.

**Unit-graph topology.** The assembler checks the unit's dimensions and
units by QSpec FR-142 "Dimension and conversion algebra", with FR-142's
rules, order and causes and none of its own:

- It decides each dimension-equivalence form by QSpec FR-142 "Dimension
  equivalence" steps 1 to 7, the first match winning, with those steps' codes
  and causes. A dimension name resolves as a dimension term does: a bare name to a
  dimension form of the unit, `M::Name` to the dimension `Name` of the
  document of the admitted package that the model alias `M` selects; a unit name
  of the `by` clause resolves as a unit's `*` target does, a bare name to a unit
  form of the unit and `M::name` to the unit `name` of that package's document.
  Step 4 reads which dimension node a unit belongs to (its `:` name, or the package
  unit's dimension); step 5 forms the unit's composed offset and scale to its canonical
  root as the product of the reduced scales and the sum of the scaled offsets along its
  target chain (a source unit's from its forms, a package unit's from its document), and
  refuses a unit whose composed offset is not zero and a ratio number that is not
  positive. Step 6 forms the **root ratio** of the pair, `(y * b) / (x * a)` reduced
  (`x U = y V`, `a` and `b` the composed scales of `U` and `V`), and the ratio each base
  dimension's root has against its class representative's root from the base pairs. Step 7
  walks the base pairs in source order, assigns each node a root size against the first node
  of its component, and refuses the first declaration whose ratio disagrees with the sizes
  the earlier declarations fixed. It takes the base pairs (both names resolve to a
  dimension that has no terms) first, steps 1 to 5 in source order and then step 7 over
  them, before the first dimension turn, and every other pair, steps 1 to 6 in source
  order, after the last turn; a pair that step 1 refuses is not classified and is refused
  in the first phase. A unit whose target chain a row 1, 2, 3 or 6 defect breaks fixes no
  scale: the equivalence naming it is decided up to step 4 and the dimension's turn reports
  the defect. An
  equivalence form is not a node of the turn list and mentions no dimension
  for its walk, so a package dimension that only an equivalence names takes
  its turn with the package dimensions not yet reached. A refusal is located
  at the equivalence form's span and retains what the step names.
- Equal pairs are decided on node identity, so `dimension Length ~ Length by 1 m = 1 m;`
  is step 2, a pair written in the other order, with any ratio, is step 3, and a source
  dimension and a package dimension that carry one name are two nodes.
- The base-dimension rule at a base dimension's turn applies to a base
  dimension related by an equivalence exactly as to any other: an equivalence
  names a unit of each of its two dimensions (step 4), so it never anchors a base
  dimension that has none. The base classes (QSpec FR-142 "Dimension
  equivalence") are the components of the base pairs admitted by the first phase.
- It takes the dimension nodes in FR-142's turn-list order, built by
  walking the declarations in source order. A dimension of an admitted
  domain package is an imported node of that list. A source declaration
  that mentions it gives it its turn when it is first reached, before the
  declaration that reached it and before the nodes reached through its
  terms. Every package dimension not yet reached takes its turn after the
  last source declaration's turn, in the order of its selection in the unit and
  then in its authored order within the package.
- An imported node's definition, terms and units count as admitted and are
  not re-checked. At its turn, rows 1 to 6 apply to the local units typed by
  it, with the package's units of that dimension as admitted members of the
  graph: a local targetless unit beside the package's root is a second root
  (row 4, retaining the package root and the local unit), a local cycle is
  row 6, a local zero scale is row 1, and a local unit that targets a
  package unit of its own node adds no cycle and no targetless unit. Row 5
  applies only to a local root of a node with no package root. The
  base-dimension rule applies to a package base dimension at its turn as to
  a local one. A refusal of a package node is located at the `model_root` node of its
  selected package and names the node's FCD identity.
- At each node's turn it runs FR-142's step (-1) over a local dimension's
  own terms (a term that names no dimension; a dimension in a dimension
  cycle or with a self-term, its edges from the cycle's dimension declared
  first), then step (0)'s base-dimension rule, then rows 1 to 3 over the
  node's local units in source order, then rows 4 to 6 over the node. Row 2
  is a unit target that names no unit of the package, neither a unit form
  nor a unit of an admitted domain package; row 6 is a unit target cycle,
  its edges retained from the cycle's least unit by source position.
- It reports the first defect found and no other: a node is refused by the
  earliest row that applies to its earliest local unit, and a later node is
  not examined once an earlier one is refused. The error carries the code
  and cause FR-142's step, rule or row gives and names the declarations it
  retains.
- A derived dimension whose normalized terms are empty, such as
  `dimension N = L / L;`, is the dimensionless dimension, and the assembler
  admits it. Its units form `N`'s own graph under rows 1 to 6, and its empty
  map creates no base-dimension obligation.

With no error, the assembler builds each preimage, owner and
`qualified_declaration` as for an enum:

1. **Exact numbers.** `rational(n, d)` is the reduced rational `n / d`,
   with a positive denominator, and `decimal(c, s)` is the reduced rational
   `c / 10^s`. The reduced value is the value written (ADR-013 R-07).
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
   with `nominal_key`, the function that mints an enum's key. It then mints
   each dimension equivalence's key over its
   `quire.dimension-equivalence-node/v1` preimage `{version, owner,
   dimension_node_ids, root_scale}`, `owner` the unit's `SourceOwner`,
   `dimension_node_ids` the keys of the two related dimension nodes, distinct
   and in ascending key order (byte order of the RFC 8785 key strings), and `root_scale` the reduced root ratio of step 6 as
   `{numerator, denominator}` decimal strings, stated for the node listed first (the
   reciprocal when the declaration names the dimensions in descending order), so a pair
   written in either order, with either unit spelling of one ratio, has one key. An equivalence rewrites no dimension: every `terms` list and every
   dimension key is the one the dimensions alone give.
4. **Unit preimages.** A unit's `quire.unit-node/v1` preimage has its
   dimension's key as `dimension_node_id`, its target's key as
   `target_unit_node_id` (`null` when no target is written), the reduced
   scale as `scale`, and the reduced offset, zero when no `+` is written,
   as `offset`. The unit's edge is `target_value = scale × source_value +
   offset` (FR-142).
5. **Unit keys.** It mints each unit's key after its target's, roots first,
   with `nominal_key`.
6. **Admission.** It calls `UnitGraph::admit` with every dimension,
   dimension-equivalence and unit preimage and its key, and `owners` holding
   exactly the unit's source owner. The result is `units`, which holds the
   equivalence nodes beside the dimension nodes and derives the equivalence
   classes, base classes and class roots from them (QSpec FR-142 "Dimension
   equivalence", "Effect"), so the checker and the replay facade read a class
   from the graph and never from the source.

`value::unit` gives `DimensionPreimage` and `UnitPreimage` a constructor
from their parts, with node keys for the nodes they reference, beside
`from_json`. The checks above leave `UnitGraph::admit` nothing to refuse,
so a refusal from a constructor or from `admit` is a nominal-admission
fault, carried with the span of the first dimension or unit form.

The assembler stops at the admitted `UnitGraph`, which keeps each
dimension's and unit's preimage bytes; `UnitTable::declared` carries them
to lowering, which builds the `quire.dimension-node/v1` and
`quire.unit-node/v1` nodes a quantity type names (FR-094).

**Dimension and unit key vectors.** Owner
`{kind: "source", authority: "a", identity: "u"}`, each
`qualified_declaration` the declared name. `dimension_node_id` and
`target_unit_node_id` are the structural keys of the nodes they name, and a rational is `{numerator, denominator}` in decimal strings.
The method reproduces QSpec's `dimension-velocity` and
`unit-degree-celsius` vectors.

| Vector | Source | Preimage content | Key |
|---|---|---|---|
| Q1 | `dimension Length;` | `terms: []` | the preimage value itself |
| Q2 | `dimension Time;` | `terms: []` | the preimage value itself |
| Q3 | `dimension Speed = Length / Time;` | `terms: [{Q2, "-1"}, {Q1, "1"}]` | the preimage value itself |
| Q4 | `dimension Accel = Speed / Time;` | `terms: [{Q2, "-2"}, {Q1, "1"}]` | the preimage value itself |
| Q5 | `dimension Temperature;` | `terms: []` | the preimage value itself |
| Q6 | `unit m : Length = rational(1, 1);` | dimension Q1, target `null`, scale 1/1, offset 0/1 | the preimage value itself |
| Q7 | `unit km : Length = rational(2000, 2) * m;` | dimension Q1, target Q6, scale 1000/1, offset 0/1 | the preimage value itself |
| Q8 | `unit cm : Length = decimal(1, 2) * m;` | dimension Q1, target Q6, scale 1/100, offset 0/1 | the preimage value itself |
| Q9 | `unit K : Temperature = rational(1, 1);` | dimension Q5, target `null`, scale 1/1, offset 0/1 | the preimage value itself |
| Q10 | `unit C : Temperature = rational(1, 1) * K + decimal(27315, 2);` | dimension Q5, target Q9, scale 1/1, offset 5463/20 | the preimage value itself |
| Q11 | `dimension Distance;` | `terms: []` | the preimage value itself |
| Q12 | `unit dist : Distance = rational(1, 1);` and `dimension Distance ~ Length by 1 dist = 1 km;` | `dimension_node_ids: [Q11, Q1]` (ascending key order), `root_scale` 1000/1 (`km` is 1000 roots of `Length`, `dist` 1 root of `Distance`) | the preimage value itself |
| Q13 | `dimension Length ~ Distance by 1 cm = 2 dist;` | `dimension_node_ids: [Q11, Q1]`, `root_scale` 1/200 (`cm` is 1/100 of a root of `Length`, so one root of `Length` is 200 roots of `Distance` and one root of `Distance`, listed first, is 1/200 root of `Length`) | the preimage value itself |

Q12's preimage is `{version: "quire.dimension-equivalence-node/v1", owner,
dimension_node_ids, root_scale}` with `owner` the source owner above. The same method
reproduces QSpec's `dimension-equivalence-rpc-flow-time`
(`root_scale` 1/1), `dimension-equivalence-rpc-clock-time`
(1000/1) and `dimension-equivalence-flow-clock-time`
(1000/1) from owner `{kind: "source", authority: "agent-ix",
identity: "example"}` and the keys of QSpec's `model-dimension-acme-rpc-time`,
`model-dimension-acme-flow-time` and `model-dimension-acme-clock-time`.

### Predicates

A predicate is a function whose declaration kind is `Predicate`. The
assembler resolves its `using` alias, parameter type forms and body
exactly as it resolves a function's, and its result type form resolves to
`Boolean`. `check` checks and calls a predicate as it checks and calls a
function, including QSpec FR-146's total-function rule that refuses a
recursive group with no measure, and
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
The `ieee_profile` selection and its build are
[FR-397](FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md)'s.

**Node key owner.** `check` mints the node keys of record, tuple and
function declarations over the unit's `SourceOwner{authority, identity}`
(ADR-013 O-04). The source revision is package lock evidence
and is not part of the preimage, so an unchanged declaration keeps its key
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

**Enum key vectors.** Each key is the RFC 8785 byte string of the
preimage shown, compared by equality, owner `{kind: "source", authority: "a", identity: "u"}`
unless stated.

| Vector | Source | Preimage (other members as above) | Key |
|---|---|---|---|
| N1 | `ordered enum Status { READY, DONE }` | `qualified_declaration: ["Status"]`, `ordered: true`, `members: ["READY", "DONE"]` | the preimage value itself |
| N2 | member `READY` of N1 | `{version: "quire.enum-member-node/v1", declaration_node_id: N1, case: "READY"}` | the preimage value itself |
| N3 | `enum Color { RED, BLUE = "Blue" }` | `qualified_declaration: ["Color"]`, `ordered: false`, `members: ["BLUE", "RED"]` | the preimage value itself |
| N4 | N1's source under owner identity `w` | N1's, with `identity: "w"` | the preimage value itself |

Type resolution is the E3 name-binding phase (ADR-011 §1; ADR-013 O-11).
The resolution `match` has one explicit arm per type-form head:

- a built-in constructor (`Boolean`, `Integer`, `Int[..]`, `Rational[..]`,
  `Decimal[..]`, `Text[..]`, `Option<..>`, `Sequence`/`Set`/`Bag`/
  `OrderedSet<..>[..]`) resolves to its `ValueType` over the declared bounds
  the type form carries;
- `Float32[mode]` and `Float64[mode]` resolve to `ValueType::Float`, which
  holds the width and the rounding mode (QSpec FR-322, FR-148); a bare
  `Float32` or `Float64` is strict `exact`. All six spellings (`exact`,
  `toward-zero`, `toward-positive`, `toward-negative`, `nearest-even`,
  `nearest-away`) are admitted. A mode the grammar does not spell is
  `ill-formed scalar bounds`. A domain package's native `Float32`/`Float64`
  model field carries no mode spelling and resolves with `exact`;
- `Reference<Q>` resolves `Q` against the admitted domain packages (I1). With
  no admitted domain package, `Q` is an unresolved name;
- the unreserved names `String`, `UUID` and `Timestamp` resolve to the native
  `ValueType::String`, `ValueType::Uuid` and `ValueType::Timestamp` (FR-056),
  take no bounds or arguments, and need no domain package, and are available as a parameter, result, field
  or alias target type of the unit. They are not keywords: for these three spellings the native type is a
  candidate alongside the unit's alias, record, tuple and enum forms, so a unit
  that also declares a form of the same name makes the use an ambiguous type
  name, below, and the exactly-one arms below apply only when the name is not
  one of these spellings;
- a qualified name that names exactly one alias form of the unit resolves to
  that alias's resolved type;
- a qualified name that names exactly one record or tuple form of the unit
  resolves to `ValueType::Composite` of that declaration's key;
- a qualified name that names exactly one enum form of the unit resolves to
  `ValueType::Enum` of that enum's `EnumBinding` shape.

An enum member written as an expression, `E::m`, is resolved by `check`
against `enums`, as FR-141 and the check stage already do: a case that `E`
does not declare refuses there with `missing_declaration`/`missing-name`. A
bare member `m` is a `Name` the assembler does not resolve; `check` resolves
it to `E::m` where its position supplies the expected enum type `E`
(FR-317 "Bare enum members").

The assembler refuses in these cases:

- **Unresolved type name.** A qualified name names no alias, record, tuple
  or enum form of the unit and nothing in an admitted domain package, other
  than the spellings `String`, `UUID` and `Timestamp`, which resolve to the
  native types. The refusal names the name and the span of the type form.
- **Ambiguous type name.** A qualified name names more than one candidate: the
  unit's alias, record, tuple and enum forms and, for `String`, `UUID` and
  `Timestamp`, the native type of that spelling (ADR-013 O-11). The refusal
  names the name, the span of the referencing type form and the span of each
  candidate declaration; the native candidate has no declaration span and is
  reported as the native type of that name, without a span.
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
- **Integer outside i128.** An `Int[lo, hi]` bound lies outside
  `i128::MIN..=i128::MAX` (stated under "Integer bounds and literals up to
  i128" below).
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

### Integer bounds and literals up to i128

Every bound of an `Int[lo, hi]` type form and every integer literal of
integer type in an expression is an exact integer, with no narrower width
anywhere from S2 through check, field refinement, lowering, the IR wire and
node keys. This rule caps nothing else: the integer arguments of
`rational(n, d)`, a unit's scale and the other constructors' bounds keep
the rules of their own requirements. The
end-to-end ceiling is i128: each such integer lies in
`i128::MIN..=i128::MAX` (-170141183460469231731687303715884105728 to
170141183460469231731687303715884105727). That range holds every value of
the native `i8` to `i64`, `u8` to `u64` and `i128` types, so `Int[0,
18446744073709551615]` (a full `u64`) and `Int[0, 9223372036854775808]`
(`i64::MAX + 1`) are ordinary types. A `u128` value above `i128::MAX` is
out of range.

A source literal is unsigned, so `i128::MIN` has one literal spelling,
folded the way Rust folds `-128i8`. A literal is *directly negated* when, in
the CST, it is the whole operand of a unary `-`: the `-` token is followed,
with only layout (whitespace and comments) between them, by the integer
literal token itself, and no parenthesis, operator or other token stands
between the two. `-170141183460469231731687303715884105728` is directly
negated; `-(170141183460469231731687303715884105728)`, `- (170141183460469231731687303715884105728)`
and `-(x + 170141183460469231731687303715884105728)` are not.

- A directly negated literal whose value is exactly 2^127
  (170141183460469231731687303715884105728) is folded: S2 builds the single
  `Integer` literal `i128::MIN` in place of the `Negate`, so check types,
  keys and lowers it as that one literal (FR-092 vector L7), with no
  negation node, and it reaches the IR wire as one `IntegerLiteral`.
- Every other negation keeps its `Negate` form, so no existing key or wire
  form changes: `-170141183460469231731687303715884105727` is a negation of
  the literal 2^127-1.
- Every literal that is not folded is in range when it is at most
  `i128::MAX`. So a 2^127 that is not directly negated refuses, and so does
  `-170141183460469231731687303715884105729`, whose literal 2^127+1 is
  above `i128::MAX`.

Check SHALL refuse an integer bound or literal outside that range with the
typed cause `IntegerOutsideI128`, a member of the ill-formed-scalar-bounds
family the `Int` interval's own refusal (`EmptyInterval`) belongs to. The
cause names:

- `site`: the bound it is, `lower` or `upper`, or `literal`;
- `value`: the integer as written, as its canonical decimal string;
- `limit`: the i128 limit it crosses, `i128::MIN` for a value below the
  range and `i128::MAX` for a value above it, as its decimal string;

and the span of the type form or the literal. Like the other assembler
errors, a bound refusal is reported with every other error of the unit. A
refused integer never reaches field refinement, keying, lowering or the IR:
no `PackageDeclarations` and no checked package holds it.

Runtime values are not type bounds. The evaluator's `ScalarLimits` budget
the size of the values a run computes; they never narrow a declared bound
or refuse a literal that this rule admits.

### Native scalar literals

The unit language writes a constant of each native value type `String`,
`UUID`, `Timestamp`, `Bytes` and `JsonObject` with one literal form. The forms are the only way to
write such a constant, and each has exactly one checked type. They add no
production, no token and no reserved word: the quoted text token is already
read by the lexer, and `UUID("...")`, `Timestamp(n)`, `Bytes("...")` and
`JsonObject("...")` are ordinary calls that the shared complete-V1 grammar
already parses.

| Literal | Syntax | Checked type | Value |
|---|---|---|---|
| `"..."` | the quoted text token | `String` | the decoded Unicode scalar sequence |
| `UUID("...")` | a call of the name `UUID` with one quoted text argument | `UUID` | the 128-bit payload of the canonical text |
| `Timestamp(n)` | a call of the name `Timestamp` with one integer argument | `Timestamp` | the signed 128-bit nanosecond count `n` on the POSIX epoch scale |
| `Bytes("...")` | a call of the name `Bytes` with one quoted text argument | `Bytes` | the octet sequence of the canonical hexadecimal text |
| `JsonObject("...")` | a call of the name `JsonObject` with one quoted text argument | `JsonObject` | the JSON value of the canonical RFC 8785 text |

**String.** The quoted text token is the JSON-compatible quoted text the
lexer already reads: the escapes `\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`,
`\t` and `\uXXXX` (a surrogate pair is one scalar) decode as JSON decodes
them. S2 builds `Expression::String` holding the decoded scalar sequence. The
sequence is kept exactly, as FR-056 states for the native String: the empty
sequence, a NUL (`"\u0000"`), a multibyte scalar and UUID-looking text are
all admitted, and no normalization, case folding or truncation applies, so
`"\u00e9"` and `"é"` are one value and differ from `"e\u0301"`. A quoted
literal has the type `String` in every position. It is never `Text`, never
`UUID` and never a variant or enum value, and it takes no profile, bound or
expected type. A malformed escape, a lone surrogate escape, an unterminated
literal and a raw control character below U+0020 inside the quotes are lexical
errors: `invalid_syntax`/`invalid-escape` for the first two and
`invalid_syntax`/`invalid-token` for the others, at the literal, before S2.

**UUID.** `UUID("...")` takes one argument, which must be a quoted text
literal (not a `String`-typed expression such as a parameter or a `let`
binding). Its text must be the canonical UUID text of FR-056's K2 canonical
native payloads: 32 lowercase hexadecimal digits grouped `8-4-4-4-12` with
four hyphens, and no other spelling. A case variant, a hyphenless, braced or
`urn:uuid:` spelling, text of another length, a non-hexadecimal digit and the
empty text are each malformed. Check admits the text as the 128-bit payload
through the same K2 admission FR-056 applies to a runtime input, and refuses a
malformed text with the typed cause `MalformedUuidLiteral`, naming the
argument text as written and the span of the call, code
`ill_typed`/`type-mismatch`. The refusal is reported with every other error of
the unit, and no checked package holds the literal.

**Timestamp.** `Timestamp(n)` takes one argument, which must be an integer
literal or a directly negated integer literal (the forms of "Integer bounds
and literals up to i128"), and the value is the nanosecond count. No calendar,
timezone, offset or precision form exists, and the count is not converted. The
value is the integer value, so `Timestamp(-0)` and `Timestamp(0)` are one
value. A count above `i128::MAX` or below `i128::MIN` is the argument literal's
own `IntegerOutsideI128` refusal, which that section states, and needs no
cause of its own; `Timestamp(-170141183460469231731687303715884105728)` is the
folded `i128::MIN` literal and is admitted. A leading-zero integer such as
`007` is refused by the lexer as `invalid_syntax`/`invalid-token` before any
form exists. An `Integer`-typed expression never converts to a `Timestamp`.

**Bytes.** `Bytes("...")` takes one argument, which must be a quoted text
literal (not a `String`-typed expression). Its decoded text must be the
canonical Bytes text of FR-056's "Native Bytes, JsonObject and Duration":
lowercase hexadecimal digits, two per octet, the empty text for the empty
sequence. `Bytes("")` and `Bytes("00ff")` are admitted; `Bytes("00FF")`,
`Bytes("0")`, `Bytes("0x00")` and `Bytes("00 ff")` are malformed. Check admits
the text through the same K2 admission FR-056 applies to a runtime input and
refuses a malformed text with the typed cause `MalformedBytesLiteral`, naming
the argument text as written and the span of the call, code
`ill_typed`/`type-mismatch`. Two `Bytes("00ff")` in one unit are one value.

**JsonObject.** `JsonObject("...")` takes one argument, which must be a quoted
text literal. Its decoded text must be the canonical RFC 8785 text of one JSON
value as FR-056 states (`JsonObject("{\"a\":1}")`, `JsonObject("[]")`,
`JsonObject("null")` are admitted; `JsonObject("{\"a\": 1}")`,
`JsonObject("{\"b\":1,\"a\":1}")`, `JsonObject("1.0")` and `JsonObject("")`
are malformed). Check admits the text through the same K2 admission and refuses
a malformed text with the typed cause `MalformedJsonObjectLiteral`, naming the
argument text as written and the span of the call, code
`ill_typed`/`type-mismatch`. A `Duration` has no literal form of its own: a
quantity literal over the unit `s` (`3 s`, `rational(3, 2) s`) is a Duration.

**Name resolution.** `UUID`, `Timestamp`, `Bytes` and `JsonObject` in a call position resolve with the native
literal constructor of that spelling as a candidate beside the unit's function,
predicate and tuple declarations of that name, and exactly one candidate is
required. This is new behavior: a call otherwise resolves a declared function
before a type, and a call headed by a native type refuses
`ill_typed`/`operator-ineligible`; the literal constructors are the only calls
that take the native type name. A unit that also
declares a function, predicate or tuple `UUID` (or `Timestamp`, `Bytes`, `JsonObject`) makes the call
an ambiguous name, `ambiguous_declaration`/`ambiguous-name`, naming the use,
the span of each declaration and the native constructor as a candidate without
a span, as the ambiguous type-name rule above does. The native operations `length(b)` and `concat(a, b)` of FR-056's "Native Bytes, JsonObject and Duration" resolve by the same rule and the same ambiguity refusal. A qualified spelling such
as `M::UUID("...")` names no native constructor and resolves as any qualified
call does, refusing `missing_declaration`/`missing-name` when it names
nothing. A call whose argument is not the form above (`UUID(x)`, `UUID(5)`,
`Timestamp(x)`, `Timestamp("5")`, `Timestamp(1 + 2)`, `Bytes(x)`, `Bytes(5)`,
`JsonObject(x)`), or that has no argument
or two, refuses `ill_typed`/`type-mismatch` at the argument, or at the call
when the argument count is not one, naming the expected argument form. The `UUID` and
`Timestamp` type names are the predeclared type names above and are unaffected.

The five literals have no operator of their own. They take part in `=` and
`!=` under FR-056, and a `Bytes` literal in `length` and `concat`, as arguments, `let` bindings, record field values,
conditional branches, collection elements and bodies, wherever a value of the
literal's type is accepted. A literal at a slot of another type, including
`"abc"` at a `Text[0, 8; nfc]` slot, `"00112233-4455-6677-8899-aabbccddeeff"`
at a `UUID` slot, `"00ff"` at a `Bytes` slot and the integer `5` at a `Timestamp` slot, refuses
`ill_typed`/`type-mismatch` at the literal as every typed mismatch does, with
no coercion. FR-093 lowers the five literals.

### Decimal constructor

The unit language writes an exact decimal constant with the constructor of QSpec
`shared-grammar.md`: `decimal(c, s)`, whose `c` is a signed integer and `s` an
unsigned integer, both in the `signed-int` and `uint` spellings (`0` or a digit
string with no leading zero, and `-` only before a nonzero digit string). It is
a keyword-headed form rather than a call: `decimal` is a reserved word, no name
resolution applies, and a unit declaration of that spelling is not a
candidate. The split between the two syntax errors is
by what S1 reads at the argument position. An argument position that holds a
numeric token (an optional `+` or `-` sign, digits and an optional fractional
part, read as one token) whose spelling is not the argument's `signed-int` or
`uint` spelling is `invalid_syntax`/`invalid-token` at that token: a `+` sign,
a leading zero, `-0`, a fractional part, and a `-` sign on the scale (`-2` is a
signed token where a `uint` is required). Every other malformation of the form
is `invalid_syntax`/`unexpected-token` at the offending token or the constructor:
a missing or extra argument, a missing comma, a missing parenthesis, a
non-numeric argument and `decimal` followed by anything but `(`. Both refuse at
the constructor, before S2. The same spelling in the magnitude of a unit
declaration or a quantity literal (the assembler's exact numbers and FR-093's quantity
literals) is an exact number of that position and builds that position's form.

S2 builds `DecimalValue{coefficient, scale}` for each value-position
constructor, keeping the coefficient and the scale as written, with the span of
the constructor. It reduces nothing, raises no power of ten and refuses no
scale, so `decimal(1, 4294967295)` builds. Typing, the bound on the scale,
lowering and evaluation are
[FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)'s "Decimal
constructors".

### Float bit constructors

The unit language writes a float constant with the bit constructors of QSpec
`shared-grammar.md`: `float32(bits: 0xHHHHHHHH)` and
`float64(bits: 0xHHHHHHHHHHHHHHHH)`. The digits are exactly eight (`float32`)
or sixteen (`float64`) lowercase hexadecimal digits after `0x`, so signed zero
and every NaN payload survive parsing. They are keyword-headed forms rather
than calls: `float32` and `float64` are reserved words, no name resolution
applies, and a unit declaration of that spelling is not a candidate. A form
with another digit count, an uppercase or non-hexadecimal digit, no `0x`, no
`bits` label or a missing colon is a lexical or syntax error,
`invalid_syntax`/`invalid-token`, at the constructor, before S2.

S2 builds `FloatBits{width, bits}` for each constructor, keeping the bit
pattern as an unsigned 32-bit or 64-bit integer and the span of the
constructor. Typing, lowering and evaluation are
[FR-397](FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md)'s.

### Catalog codes

Each S2 and assembler cause has one exhaustive `catalog_code()` (ADR-013
O-17). The codes come from the catalog, written as
code or code/cause:

| Cause | Code |
|---|---|
| `RecoveringCst` | `invalid_syntax` |
| diagnosed source (diagnostic, no recovery) | the first diagnostic's own code |
| `NoDispatchEntry`, `UnrepresentedConstruct` | `unsupported_construct` |
| unresolved type name | `missing_declaration`/`missing-name` |
| ambiguous type name | `ambiguous_declaration` |
| ill-formed scalar bounds | `ill_typed` |
| integer outside i128 (`IntegerOutsideI128`), a bound or a literal | `ill_typed`/`type-mismatch` |
| malformed UUID literal (`MalformedUuidLiteral`) | `ill_typed`/`type-mismatch` |
| undeclared `using` alias | `missing_declaration`/`missing-selection` |
| duplicate alias | `ambiguous_declaration`/`ambiguous-name` |
| alias cycle | `invalid_package`/`definition-cycle` (catalog extension proposed, Dependencies) |
| duplicate enum member | `ambiguous_declaration`/`ambiguous-name` |
| nominal-admission fault | `runtime_invariant`/`established-invariant-broken` |
| duplicate dimension or unit name | `ambiguous_declaration`/`ambiguous-name` |
| unresolved unit dimension name | `missing_declaration`/`missing-name` |
| zero denominator | `undefined_expression`/`unproved-nonzero` |
| decimal-scale limit | `stage_limit_exceeded`/`work-budget-exceeded` |
| type-environment allocator denial | `resource_exhausted`/`allocation-failed` |
| type-environment reservation size arithmetic or representation overflow | `resource_exhausted`/`reservation-size-overflow` |
| unit-graph topology | the code and cause QSpec FR-142 gives the defect: `missing_declaration`/`missing-name` (an unknown term, an unresolved equivalence dimension or unit name, the base-dimension rule, row 2), `invalid_package`/`definition-cycle` (a dimension cycle, an equivalence of one node, row 6), `invalid_package`/`invalid-value` (rows 1 and 5, an equivalence's affine unit or non-positive ratio number), `invalid_package`/`incompatible-definition` (an equivalence whose maps or ratio differ or whose unit is of another dimension, row 3), `invalid_package`/`conflicting-definition` (a repeated equivalence pair or a base cycle whose ratios do not multiply to one, row 4) |
| unresolved state clause operation (FR-104) | `missing_declaration`/`missing-name` |
| ambiguous state clause operation (FR-104) | `ambiguous_declaration`/`ambiguous-name` |
| ambiguous state clause population (FR-104) | `ambiguous_declaration`/`ambiguous-name` |

The alias-cycle row uses `definition-cycle`, whose payload is dependency
edges: the alias chain is those edges. The check stage uses the same code
for an FR-151 call-graph cycle (`qsl-semantics/src/check/refusal.rs`,
`CheckCause::DefinitionCycle`). The duplicate-alias row follows
the retired `complete::package` resolver, which refused a duplicate selection alias as
`ambiguous-name`. The duplicate-enum-member row follows the record rule,
where a duplicate field name is `ambiguous-name`
(`DeclarationCause::DuplicateMember`): the catalog's `ambiguous-name`
retains the visible name and the distinct conflicting declarations and
loci. The nominal-admission-fault row follows the assembler's other broken
invariants (the declared-type handle, a domain package's object type).
The duplicate dimension or unit name row follows the duplicate-enum-member
row, and the zero-denominator row follows the check stage's own refusal of
`rational(n, 0)` in an expression (`Obligation::Nonzero`). The unit-graph
topology row takes QSpec FR-142's causes, so one defect has one code in
every checker of the language.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-091-CON-1 | The `Value` family form builder and the assembler are the only producers of a `PackageDeclarations` value from complete-V1 source. The assembler's parsed-form input is S2 output. Its other E3 inputs are those ADR-011 §2.1's E3 row admits (admitted domain packages) and the unit's `SourceOwner`. | Design | Inspection |
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
| FR-091-AC-7 | S2 refuses the whole unit with cause `UnrepresentedConstruct`, naming the construct's CST production and span and returning no form, for a function body that holds exactly one of: `xs[0]`, `none`, `reaches(a, b, M::R)`. | Test |
| FR-091-AC-8 | For each of `function f using v(x: Int[0, 9]): Boolean pure { B }` with body `B` equal to `deref(x)`, `allInstances<M::T>(x)` or `pre(x)`, and `function g using v(x: Int[0, 9]): Boolean pure decreases(pre(x)) { true }`, each in a unit with no admitted domain package, S2 returns a parsed unit that holds `Deref`, `AllInstances` or `Pre` where the construct is written. Check refuses `pre(x)`, in the body and in the measure, with `wrong_snapshot`/`forbidden-pre-read`, the `ProtocolClause` cause, and `deref(x)` with `ill_typed`/`type-mismatch`. The assembler refuses `allInstances<M::T>(x)` with an unresolved-type-name error naming `M::T` (`missing_declaration`/`missing-name`). None of these refusals has code `unsupported_construct`. | Test (TC-396) |
| FR-091-AC-9 | S2 builds a form for each of the function bodies `not`×8 `a`, `not`×20 `a` and `not`×129 `a`, each admitted by S1 at its default limits, and each form's expression tree is as deep as its body. S2 takes no limit set, and no S2 outcome names a depth. | Test (TC-397) |
| FR-091-AC-10 | Every `Expression` node in a `Value` form carries the span of the CST node it maps from. For the body `if a then b else c + d`, the `If` node's span covers the whole body, the `Add` node's span covers `c + d`, and the `Name("d")` node's span covers `d`. For `(a + b) * c`, the `Add` node's span covers `a + b`, without the parentheses. | Test (TC-403) |
| FR-091-AC-11 | The `Value` family form builder module has `use` edges and inline paths only to the `forms` core, `qsl_cst`, `qsl_foundation` and `quire_exact`. It has none to another family module or to anything under `crate::check`, `crate::value` or `crate::model`. No field of a `Value` parsed form, of the type form, of a parsed-unit selection, or of an `Expression` variant in the mapping table has type `ValueType` or `NodeKey`. The dispatch `match`, the expression `match` and the assembler's resolution `match` have no `_` arm. | Test (TC-398) |
| FR-091-AC-12 | The assembler returns a `PackageDeclarations` value for the S2 output of source that declares `type Digit = Int[0, 9];`, then `function inc using v(x: Digit): Int[0, 10] pure { x + 1 }`, then `function two using v(): Int[0, 10] pure { inc(1) }`. Its `aliases` is `[("Digit", Int[0..9])]`. Its `functions` are `inc` and then `two`. `inc`'s check-owned resolved signature has parameter type `Int[0..9]` and result type `Int[0..10]`. | Test (TC-399) |
| FR-091-AC-13 | Calling `two` with no arguments through `CheckedPackage::call`, on the package that `PackageDeclarations::check` admits and links from the AC-12 assembler output, returns a completed outcome with integer value `2`. | Test (TC-399) |
| FR-091-AC-14 | The assembler returns one refusal holding two unresolved-type-name errors for a unit with `function f using v(x: Missing): Boolean pure { true }` and `function g using v(y: Absent): Boolean pure { true }`. One error names `Missing` with its type form's span, and the other names `Absent` with its type form's span. It returns no `PackageDeclarations`. | Test (TC-400) |
| FR-091-AC-15 | The assembler refuses with an ambiguous-type-name error for a unit with `type A = Int[0, 1];`, `type A = Int[0, 2];` and `function f using v(x: A): Boolean pure { true }`. The error names `A`, the span of `x`'s type form and the spans of both `type A` declarations. | Test (TC-400) |
| FR-091-AC-16 | The assembler returns one refusal with three errors for parameters typed `Int[9, 0]`, `Rational[0, 1; 0, 5]` and `Text[5, 1; nfc]`, one per function. Each error carries the owning value type's own rejection cause and its type form's span. It returns no `PackageDeclarations`. | Test (TC-400) |
| FR-091-AC-17 | The assembler refuses with an alias-cycle error, code `invalid_package`/`definition-cycle`, naming both `A` and `B` for `type A = B;` and `type B = A;`, and returns no `PackageDeclarations`. It refuses `type C = Option<C>;` with the same code, naming `C`. | Test (TC-400) |
| FR-091-AC-18 | For `record Point { x: Int[0, 9]; y: Int[0, 9]; }`, `tuple Pair(Int[0, 9], Int[0, 9]);` and `function px using v(p: Point): Int[0, 9] pure { p.x }`, assembled under source owner authority `a`, identity `u`, the assembler's `types` holds one record declaration `Point` and one tuple declaration `Pair`, each with a key that `check` minted over that owner. `px`'s resolved parameter type is `ValueType::Composite` of `Point`'s key, and `PackageDeclarations::check` admits the package. Assembling and checking the same source again under (`a`, `u`) gives the same `Point` and `Pair` keys and the same checked node id for `px`. Under (`a`, `w`) it gives a different key for each of the three. No item named `DEFAULT_PACKAGE_IDENTITY` exists under `src/`. | Test (TC-401) |
| FR-091-AC-19 | The assembler admits a parameter typed `Float64[nearest-even]`, `Float32[toward-zero]` or `Float64`. Resolving each gives a `ValueType::Float` of the written width and rounding mode `nearest-even`, `toward-zero` and `exact` (the bare spelling). The evaluator rounds a `Float64[mode]` `+` by that mode. It refuses a parameter typed `Reference<M::T>`, in a unit with no admitted domain package, with an unresolved-type-name error naming `M::T`, and returns no `PackageDeclarations`. | Test (TC-405) |
| FR-091-AC-20 | The assembler module is under the layer-3 `check` core. Its non-test code has no `use` edge or inline path to `qsl_cst`. Its `#[cfg(test)]` code may reach `qsl_cst` only to run S1 and S2. | Test (TC-402) |
| FR-091-AC-21 | `catalog_code()` on each S2 and assembler cause returns the code in the Catalog codes table, and matches every cause with no `_` arm. The diagnosed-source cause returns its diagnostic's own code. The undeclared-alias cause `missing_declaration`, the duplicate-alias cause `ambiguous_declaration`, the alias-cycle cause `invalid_package`, the duplicate-enum-member cause `ambiguous_declaration`/`ambiguous-name`, the nominal-admission-fault cause `runtime_invariant`/`established-invariant-broken`, the duplicate dimension or unit name cause `ambiguous_declaration`/`ambiguous-name`, the unresolved unit dimension name cause `missing_declaration`/`missing-name`, the zero-denominator cause `undefined_expression`/`unproved-nonzero`, and the decimal-scale limit `stage_limit_exceeded`/`work-budget-exceeded`. Each unit-graph topology cause returns the code and cause of its QSpec FR-142 step, rule or row. | Test (TC-406) |
| FR-091-AC-22 | For a unit with one profile selection, alias `v`, and `function f using v(): Boolean pure { true }`, the assembler records `f`'s `using` alias as resolved to that selection. With `function g using w(): Boolean pure { true }` added, it returns one refusal holding an undeclared-alias error, code `missing_declaration`/`missing-selection`, that names `w` and the span of `g`'s `using` field, and no `PackageDeclarations`. A unit that declares two profile selections with alias `v` refuses with a duplicate-alias error, code `ambiguous_declaration`/`ambiguous-name`, naming `v` and both selection spans. | Test (TC-412) |
| FR-091-AC-23 | S1 admits a unit whose function parameter is typed `Float32` or `Float64` with no `[mode]`, and S2 builds that parameter's type form with head `Float32` or `Float64` and no rounding mode. | Test (TC-405) |
| FR-091-AC-24 | Spine `compile` of a unit that declares `import "test/units" as u;`, with no library supplied as `test/units`, refuses at stage `intake`, before assembly, with `missing_import`/`missing-selection` naming `test/units` at the import's identity string (FR-099, ADR-015 D-1), and emits no package. | Test (TC-405) |
| FR-091-AC-25 | S2 builds, for `ordered enum Level { LOW, HIGH = "High", }`, an enum form with name `Level` and the span of `Level`, ordered, and the members `LOW` with no display string and then `HIGH` with display string `"High"` and the span of that literal, each member with the span of its case name. For `enum Color { RED, BLUE }` it builds an enum form that is not ordered, with members `RED` and then `BLUE`, in source order. | Test (TC-480) |
| FR-091-AC-26 | The `FunctionDeclaration` that S2 builds from `predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }` reads: kind `Predicate`; name `Positive`; `using` field alias `v`, with the span of that `v`; parameter `x`, a type form with head `Int` and bounds spelled `0` and `9`; result, a type form with head `Boolean` and the span of the `Boolean` token; no measure; body `Binary{Greater, Name("x"), Integer(0)}`. The `FunctionDeclaration` that S2 builds from a `function` declaration has kind `Function`. | Test (TC-480) |
| FR-091-AC-27 | The assembler returns, for `ordered enum Status { READY, DONE }` and then `enum Color { RED, BLUE = "Blue" }` under source owner authority `a`, identity `u`, `enums` holding `Status` and then `Color`. `Status`'s declaration key is vector N1 and its members are `READY` then `DONE`, with `READY`'s member key N2. `Color`'s declaration key is N3 and its members are `BLUE` then `RED`. Assembling `enum Color { RED, BLUE }` under (`a`, `u`) also gives N3. Assembling the `Status` source under (`a`, `w`) gives N4. `declared_type_spans` holds the span of each of `Status` and `Color`. | Test (TC-481) |
| FR-091-AC-28 | On the package that `PackageDeclarations::check` admits and links from the assembler output for a unit with one profile selection `v`, `ordered enum Status { READY, DONE }`, `function isReady using v(s: Status): Boolean pure { s = Status::READY }`, `function ok using v(): Boolean pure { isReady(Status::READY) }` and `function later using v(): Boolean pure { Status::READY < Status::DONE }`, `isReady`'s resolved parameter type is `ValueType::Enum` of `Status`'s shape, and calling `ok` and `later` through `CheckedPackage::call` each return a completed outcome with value `true`. With `enum Color { RED, BLUE }` and `function bad using v(): Boolean pure { Color::RED < Color::BLUE }`, check refuses with `ill_typed`. With `function gone using v(): Boolean pure { Status::GONE = Status::READY }`, check refuses with `missing_declaration`/`missing-name` naming `Status::GONE`. | Test (TC-481) |
| FR-091-AC-29 | The assembler returns one refusal, and no `PackageDeclarations`, for a unit with `enum E { A, B, A }`, `enum F { X }`, `record F { y: Boolean; }`, `function f using v(p: F): Boolean pure { true }` and `function g using v(p: Shade): Boolean pure { true }`. It holds a duplicate-enum-member error, code `ambiguous_declaration`/`ambiguous-name`, naming `E`, `A` and the spans of both `A` members; an ambiguous-type-name error naming `F`, the span of `p`'s type form in `f` and the spans of both `F` declarations; and an unresolved-type-name error naming `Shade`. | Test (TC-481) |
| FR-091-AC-30 | The assembler returns, for `predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }` and then `function three using v(): Boolean pure { Positive(3) }`, `functions` holding `Positive`, kind `Predicate`, and then `three`, kind `Function`. `Positive`'s resolved signature has parameter type `Int[0..9]` and result type `Boolean`, and its `using` alias resolves to the selection `v`. Calling `three` through `CheckedPackage::call` on the checked package returns a completed outcome with value `true`. `predicate Q using w(x: Boolean): Boolean { x }`, in a unit with no selection `w`, refuses with an undeclared-alias error naming `w`. `predicate R using v(x: Boolean): Boolean { R(x) }` refuses at check with the FR-146 `missing-measure` obligation, as a recursive function written without `decreases` does. | Test (TC-481) |
| FR-091-AC-31 | S2 builds, for `dimension Length;`, a dimension form named `Length` with no terms. For `dimension Accel = Length * Time^-2 / Mass;` it builds terms `Length` (no operator, no exponent), `Time` (`*`, exponent `-2`) and `Mass` (`/`, no exponent), each with the span of its name and of its exponent when written. For `unit C : Temperature = rational(1, 1) * K + decimal(27315, 2);` it builds a unit form named `C` with dimension name `Temperature`, scale `rational` with parts `1` and `1`, target `K` and offset `decimal` with parts `27315` and `2`, each with its span. For `unit m : Length = rational(1, 1);` the unit form has no target and no offset. | Test (TC-482) |
| FR-091-AC-32 | The assembler returns, for vectors Q1 to Q10's sources in one unit under source owner (`a`, `u`), `units` holding dimensions keyed Q1 to Q5 and units keyed Q6 to Q10. Q3's and Q4's terms are the base terms the vectors list, so `Accel = Speed / Time` normalizes to `Length^1 Time^-2`. `km`'s edge has scale `1000` (from `rational(2000, 2)`), `cm`'s scale `1/100` (from `decimal(1, 2)`), and `C`'s offset `5463/20`; `C` is affine and `km` is not. `nominal_spans` holds the span of each declared name by its key. A unit with no dimension or unit form has the empty `units`. With the decimal-scale bound set to `4`, `unit c : Length = decimal(1, 5) * m;` gives a limit refusal, `stage_limit_exceeded`/`work-budget-exceeded`, naming bound `4`, actual `5` and the span of `decimal(1, 5)`, and `decimal(1, 4)` assembles. | Test (TC-483) |
| FR-091-AC-33 | A parameter typed `m`, in a unit that declares `dimension Length;` and `unit m : Length = rational(1, 1);`, refuses with an unresolved-type-name error naming `m`. | Test (TC-483) |
| FR-091-AC-34 | The assembler returns one refusal, and no `PackageDeclarations`, for a unit with `dimension Length;`, `dimension Mass;`, `dimension Mass;`, `dimension Area = Width^2;`, `dimension P = Q; dimension Q = P;`, `unit a : Length = rational(1, 0);`, `unit b : Length = rational(2, 1) * c;`, `unit c : Length = rational(1, 2) * b;` and `unit d : Ghost = rational(1, 1);`. It holds exactly these errors: a duplicate-name error naming `Mass` and both spans (`ambiguous_declaration`/`ambiguous-name`); an unresolved-name error naming `Ghost` (`missing_declaration`/`missing-name`); and a zero-denominator error at `rational(1, 0)` (`undefined_expression`/`unproved-nonzero`). The topology check does not run, so `Width`, the `P`/`Q` cycle and the `b`/`c` cycle are not reported. With the second `dimension Mass;`, `unit a` and `unit d` removed, the unit refuses with exactly one topology error, the first defect: at `Length`'s turn, row 6, `invalid_package`/`definition-cycle` with edges `b -> c` and `c -> b`; `Width` and the `P`/`Q` cycle, at later turns, are not reported. | Test (TC-483) |
| FR-091-AC-35 | Each of these units refuses with exactly one topology error carrying its QSpec FR-142 row's code and cause and naming what the row retains, and returns no `PackageDeclarations`: `dimension L; unit r : L = rational(1, 1); unit z : L = rational(0, 1) * r;` (row 1, `invalid_package`/`invalid-value`, `z` and its scale 0); `dimension L; unit r : L = rational(1, 1); unit u : L = rational(1, 1) * ghost;` (row 2, `missing_declaration`/`missing-name`, `u` and `ghost`); `dimension L; dimension T; unit r : L = rational(1, 1); unit s : T = rational(1, 1) * r;` (row 3, `invalid_package`/`incompatible-definition`, `s`, `r`, `L` and `T`); `dimension L; unit r : L = rational(1, 1); unit q : L = rational(1, 1);` (row 4, `invalid_package`/`conflicting-definition`, `L`, then `r` and `q` in source order); and `dimension L; unit r : L = rational(2, 1);` (row 5, `invalid_package`/`invalid-value`, `r`, scale 2 and offset 0). `dimension L; dimension N = L / L;` assembles, `N` being the dimensionless dimension with empty terms, and so does `dimension L; unit m : L = rational(1, 1); dimension N = L / L; unit one : N = rational(1, 1);`. | Test (TC-483) |
| FR-091-AC-36 | Check admits `function u using v(x: Int[0, 18446744073709551615]): Boolean pure { x >= 0 }` and the same function with `x: Int[0, 9223372036854775808]` and with `x: Int[0, 18446744073709551616]` (2^64, inside i128); each parameter's resolved type is `ValueType::Int` with exactly those bounds. The 2^64 unit also keys, lowers and replays like the others (FR-092-AC-14, FR-098-AC-11). Check admits a parameter typed `Int[-170141183460469231731687303715884105728, 170141183460469231731687303715884105727]`, whose bounds are `i128::MIN` and `i128::MAX` exactly, and the body `x <= 170141183460469231731687303715884105727` over it. | Test (TC-909) |
| FR-091-AC-37 | Check refuses `Int[0, 170141183460469231731687303715884105728]` (`i128::MAX + 1`) with `IntegerOutsideI128` naming site `upper`, value `170141183460469231731687303715884105728`, limit `i128::MAX` and the type form's span, and `Int[-170141183460469231731687303715884105729, 0]` naming site `lower` and limit `i128::MIN`. Each refusal's catalog code is `ill_typed`/`type-mismatch`, and the compile returns no checked package, so nothing is keyed or lowered. | Test (TC-909) |
| FR-091-AC-38 | Over `x: Int[-170141183460469231731687303715884105728, 0]`: the body `x >= -170141183460469231731687303715884105728` checks, its right operand is one `Integer` literal of value `i128::MIN` with no `Negate` node, that literal node keys to FR-092 vector L7, and lowering writes it as one literal. The body `x >= -170141183460469231731687303715884105727` checks with its right operand a `Negate` of the literal 170141183460469231731687303715884105727, as before. The bodies `x < 170141183460469231731687303715884105728`, `x >= -(170141183460469231731687303715884105728)` and `x >= -170141183460469231731687303715884105729` refuse `IntegerOutsideI128` at the literal's span, with site `literal`, the literal's value as written, limit `i128::MAX` and code `ill_typed`/`type-mismatch`, and return no checked package. | Test (TC-909) |
| FR-091-AC-39 | S2 maps `a div b`, `a rem b` and `a mod b` to distinct `IntegerDiv`, `IntegerRem` and `IntegerMod` binary forms with operand order and original spans, including in an unreachable branch. Check admits each well-typed operator under a resolved `quire.value.complete/v1` selection, using the checked package's exactly-one law for `div`/`rem` and Euclidean meaning for `mod`; without that facet, each receives a located `unsupported_construct` rather than `UnrepresentedConstruct`. `/` never becomes integer division; only under complete value does a rational `/` with integer operands and an expected rational type follow QSpec FR-149. | Test |
| FR-091-AC-40 | The assembler admits `function f using v(a: String, b: UUID, c: Timestamp): Timestamp pure { c }` and resolves `a`, `b`, `c` and the result to the native String, UUID and Timestamp value types; a use of these names needs no `using` selection or domain package. A unit declaring `record UUID { x: Integer; }` and using `UUID` as a type refuses `ambiguous_declaration`/`ambiguous-name` naming the use, the span of the record declaration and the native `UUID` candidate with no span. | Test (TC-1028) |
| FR-091-AC-41 | S2 builds, for the bodies `"abc"`, `""`, `"\u00e9"`, `"é"`, `"e\u0301"`, `"a\nb"`, `"\ud83d\ude00"` and `"\u0000"`, one `Expression::String` each, holding the scalar sequences: `a`, `b`, `c`; empty; U+00E9 (twice); U+0065 U+0301; `a`, LF, `b`; the single scalar U+1F600; U+0000. For `UUID("00112233-4455-6677-8899-aabbccddeeff")`, `Timestamp(0)`, `Timestamp(-1)` and `Timestamp(-170141183460469231731687303715884105728)` it builds a `Call` of the name `UUID` or `Timestamp` and no new expression kind; check resolves each to the native `UUID` or `Timestamp` literal, the last holding `i128::MIN`, and `Timestamp(-0)` and `Timestamp(0)` are one value. | Test (TC-1052) |
| FR-091-AC-42 | `"\ud800"` and `"\q"` refuse `invalid_syntax`/`invalid-escape`, and an unterminated literal, a literal holding a raw line feed and `Timestamp(007)` refuse `invalid_syntax`/`invalid-token`, each at the offending token with no form. `UUID(x)` over a `String` parameter, `UUID(5)`, `Timestamp(x)`, `Timestamp("5")`, `Timestamp(1 + 2)`, `Timestamp()` and `Timestamp(1, 2)` each refuse `ill_typed`/`type-mismatch` at the argument (at the call for `Timestamp()` and `Timestamp(1, 2)`). `function UUID using v(): Boolean pure { true }` beside a use `UUID("00112233-4455-6677-8899-aabbccddeeff")` refuses `ambiguous_declaration`/`ambiguous-name` naming the use, the declaration's span and the native constructor with no span; `M::UUID("00112233-4455-6677-8899-aabbccddeeff")` over an alias `M` that declares no `UUID` refuses `missing_declaration`/`missing-name`. A parameter or field named `timestamp` or `uuid` stays valid. | Test (TC-1052) |
| FR-091-AC-43 | Check refuses `UUID("00112233-4455-6677-8899-AABBCCDDEEFF")`, `UUID("00112233445566778899aabbccddeeff")`, `UUID("{00112233-4455-6677-8899-aabbccddeeff}")`, `UUID("urn:uuid:00112233-4455-6677-8899-aabbccddeeff")`, `UUID("00112233-4455-6677-8899-aabbccddeef")`, `UUID("00112233-4455-6677-8899-aabbccddeefg")` and `UUID("")` with `MalformedUuidLiteral`, code `ill_typed`/`type-mismatch`, naming the argument as written and the call's span. It refuses `Timestamp(170141183460469231731687303715884105728)` and `Timestamp(-170141183460469231731687303715884105729)` with `IntegerOutsideI128`, site `literal`, limit `i128::MAX` in both cases (the second names the positive literal 170141183460469231731687303715884105729, as FR-091-AC-38 states), code `ill_typed`/`type-mismatch`; `Timestamp(170141183460469231731687303715884105727)` checks. A refused literal reaches no keying, lowering or IR. | Test (TC-1052) |
| FR-091-AC-44 | In a unit declaring `dimension Time;`, `unit s : Time = rational(1, 1);` and `unit ms : Time = rational(1, 1000) * s;`, S1 parses `d: Quantity<ms>` as a `Quantity` type form over the qualified name `ms`, and the assembler resolves it to the type of the unit node `ms`, introducing no declaration; `d: Quantity<Time>` (a dimension) and `d: Quantity<nothing>` each refuse with an unresolved-name error naming the name (`missing_declaration`/`missing-name`), and `f(3 s)` where `f` takes `d: Quantity<ms>` refuses `ill_typed`/`type-mismatch` at the argument. A unit declaring a unit `ms` and a record `ms` checks, the type position meaning the record and `Quantity<ms>` the unit; two units named `ms` declared in one unit of compilation refuse at the two declarations, each named, `ambiguous_declaration`/`ambiguous-name` (the duplicate dimension or unit name rule); and a declaration named `Quantity` refuses `invalid_syntax` (QSpec FR-142-AC-13). | Test (TC-483) |
| FR-091-AC-45 | Base-dimension rule (QSpec FR-142-AC-16). `dimension L; dimension A = L ^ 2; unit m2 : A = rational(1, 1);` refuses with one error at `L`'s turn, `missing_declaration`/`missing-name`, naming `L` and `m2`, the first unit in source order whose dimension has a nonzero exponent of `L`; with `unit m : L = rational(1, 1);` added it assembles. `dimension L; dimension A = L ^ 2;`, with no unit, assembles: a unitless base dimension carried by no unit-bearing dimension, and a derived dimension without a unit, are not refused. `dimension L; dimension N = L / L; unit one : N = rational(1, 1);` assembles, since `N`'s empty map names no base dimension. | Test (TC-483) |
| FR-091-AC-46 | First defect (QSpec FR-142-AC-15). `dimension L; unit r : L = rational(1, 1); unit z : L = rational(0, 1) * ghost;` refuses row 1 at `z`, not row 2; `dimension L; unit r : L = rational(1, 1); unit u : L = rational(1, 1) * ghost; unit z : L = rational(0, 1) * r;` refuses row 2 at `u`, the earlier unit; `dimension L; unit r : L = rational(1, 1); unit q : L = rational(1, 1); unit s : L = rational(1, 1) * s;` refuses row 4 (`r`, `q`), not row 6 (`s`'s self-cycle); `dimension L; dimension T; unit t : T = rational(0, 1) * tr; unit tr : T = rational(1, 1); unit l : L = rational(2, 1);` refuses row 5 at `L`'s root `l`, not row 1 at `t`, since `L` takes its turn before `T`; `dimension A = B; dimension B = A; dimension L; unit l : L = rational(2, 1);` refuses step (-1)'s `invalid_package`/`definition-cycle` at `A`, edges `A -> B` and `B -> A`, not row 5 at `l`. | Test (TC-483) |
| FR-091-AC-47 | Least-unit cycle (QSpec FR-142-AC-15). `dimension L; unit r : L = rational(1, 1); unit y : L = rational(1, 1) * x; unit x : L = rational(1, 1) * y;` refuses row 6, `invalid_package`/`definition-cycle`, with edges `y -> x` then `x -> y`, from `y`, the cycle's least unit by source position, not from `x`, the least by name. | Test (TC-483) |
| FR-091-AC-48 | S2 builds, for the bodies `float64(bits: 0x3ff0000000000000)`, `float64(bits: 0x8000000000000000)`, `float64(bits: 0x7ff0000000000001)` and `float32(bits: 0x7fc00001)`, one `FloatBits` each holding the width `64`, `64`, `64` and `32` and the bit patterns `0x3ff0000000000000`, `0x8000000000000000`, `0x7ff0000000000001` and `0x7fc00001`, each with the constructor's span, and refuses no unit with `UnrepresentedConstruct`. | Test (TC-1091) |
| FR-091-AC-49 | `float64(bits: 0x3ff0)`, `float64(bits: 0x3FF0000000000000)`, `float32(bits: 3f800000)`, `float32(bits: 0x3f80000g)`, `float32(bits: 0x3f800000000)` and `float64(0x3ff0000000000000)` refuse `invalid_syntax`/`invalid-token` at the constructor before S2. | Test (TC-1091) |
| FR-091-AC-50 | In a unit declaring `model R = "acme/rpc" …`, `dimension Time;`, `unit s : Time = rational(1, 1);`, `unit tenth : R::Time = rational(1, 10) * R::s;` and `dimension Speed = R::Time ^ -1;`, where `acme/rpc` declares a base dimension `Time` and a root unit `s`, the assembler admits every form with no unresolved-name and no duplicate error: the source `Time` and `R::Time` are two dimensions and the source `s` and `R::s` two units, and the forms keep the qualified names as written for S3. `unit u : Time2 = rational(1, 1);` with no `Time2` declared in the source refuses with an unresolved-name error naming `Time2`, and a second `dimension Time;` refuses `ambiguous_declaration`/`ambiguous-name` at the two declarations, each named. | Test (TC-483) |
| FR-091-AC-55 | Package dimensions and units (QSpec FR-142-AC-16, AC-18, AC-19). With admitted domain packages `P` and `Q` selected by the unit in that order, `P` first and `Q` second, `P` declaring the TC-187 `P` dimensions and units and `Q` declaring U11z2's `T` with its unit `s`, each of QSpec TC-187's U11r, U11s, U11t, U11u, U11y, U11z1 and U11z2, its `P` declarations supplied by `P` and its local declarations in the unit, gives TC-187's result through the assembler. U11r admits a local `cm` that targets `P`'s root `m`. U11s refuses row 6 at `P`'s `L`, retaining `a -> b` and `b -> a`, and `P`'s `m` is not re-checked. U11t refuses row 4 with `P`'s root `m` and the local `m2`. U11u refuses row 1 at `z`. U11y and U11z1 refuse the base-dimension rule at `P`'s `L`, `missing_declaration`/`missing-name`, retaining `L` and `m2`, located at `P`'s `model_root` node and naming `L`'s FCD identity. U11z2 refuses the base-dimension rule at `P`'s `L` before the later zero scale. FR-142-AC-19's `unit tick : M::Rate = 1/2 * M::us`, typed by the package dimension `Rate`, is admitted, and `unit tick : Rate = 1/2 * M::us` refuses `missing_declaration`/`missing-name` at `Rate`. Dimension-name sharing (QSpec FR-142-AC-19): a unit declaring `dimension Time;` beside `acme/rpc` also declaring a dimension `Time` assembles with no error, the two being distinct dimensions; two admitted packages that each declare a dimension `Rate`, with no source `Rate`, assemble with no error; a source unit `ms` and `acme/rpc`'s unit `ms` both check, as `Quantity<ms>` and `Quantity<M::ms>`. | Test (TC-483) |
| FR-091-AC-56 | `Bytes("")`, `Bytes("00ff")` and `Bytes("deadbeef")`, and `JsonObject("{}")`, `JsonObject("[]")`, `JsonObject("3")`, `JsonObject("null")` and `JsonObject("{\"a\":1,\"b\":[true,null]}")`, are admitted with the types `Bytes` and `JsonObject`; `Bytes("00FF")`, `Bytes("0")`, `Bytes("0x00")` and `Bytes("00 ff")` refuse `ill_typed`/`type-mismatch` with the cause `MalformedBytesLiteral` naming the text as written and the span of the call, and `JsonObject("{\"b\":1,\"a\":1}")`, `JsonObject("{\"a\": 1}")`, `JsonObject("1.0")`, `JsonObject("{\"a\":1,\"a\":2}")`, `JsonObject("")` and `JsonObject("{")` the same with `MalformedJsonObjectLiteral`. `Bytes(x)` over `x: String`, `Bytes(5)`, `JsonObject(x)` and `Bytes("00", "11")` refuse `ill_typed`/`type-mismatch`; `"00ff"` at a `Bytes` slot refuses the same; a unit declaring `function Bytes using v(x: Integer): Integer pure { x }` makes `Bytes("00")` an `ambiguous_declaration`/`ambiguous-name` naming the declaration and the native constructor; `3 s` and `rational(3, 2) s` are `Duration` values where the unit `s` is typed by a base dimension `Time`. | Test (TC-1231) |
| FR-091-AC-57 | S2 builds, for the bodies `decimal(125, 2)`, `decimal(-5, 0)`, `decimal(0, 3)` and `decimal(1, 4294967295)`, one `DecimalValue` each holding the coefficient `125`, `-5`, `0` and `1` and the scale `2`, `0`, `3` and `4294967295` exactly as written, each with the constructor's span, reduces none of them, and refuses no unit with `UnrepresentedConstruct`; the exact numbers of `unit cm : Length = decimal(1, 2) * m;` and of a quantity literal magnitude still build their unit and quantity forms, not `DecimalValue`. | Test (TC-1232) |
| FR-091-AC-58 | `decimal(1)`, `decimal(1, 2, 3)`, `decimal 5`, `decimal(1 2)`, `decimal(1, x)` and `decimal(1, 2` refuse `invalid_syntax`/`unexpected-token` (a missing or extra argument, comma or parenthesis, or a non-numeric argument); `decimal(+1, 2)`, `decimal(01, 2)`, `decimal(-0, 2)`, `decimal(1.5, 2)`, `decimal(1, 02)`, `decimal(1, -2)` and `decimal(1, 2.0)` refuse `invalid_syntax`/`invalid-token` (a numeric token whose spelling is not `signed-int` for the coefficient or `uint` for the scale), each at the constructor before S2. `function decimal using v(): Boolean pure { true }` refuses `invalid_syntax` at the declaration name, because `decimal` is reserved. | Test (TC-1232) |
| FR-091-AC-59 | S2 builds, for `dimension Distance ~ Length by 1 dist = 1 km;`, one dimension-equivalence form with the dimension names `Distance` and `Length` in that order, each with its span, two ratio terms (the integer `1` with the unit name `dist`, the integer `1` with the unit name `km`) each with its spans, and the declaration's span, and no dimension form; for `dimension R::Time ~ F::Time by 1 R::s = rational(1, 1000) F::ms;` a form whose names are the qualified names `R::Time` and `F::Time` and whose unit names are `R::s` and `F::ms` as written, each with the span of both segments, the second number a `rational` with parts `1` and `1000`; for `decimal(5, 1)` in a ratio term a `decimal` number with coefficient `5` and scale `1`; and for `dimension R::Time ~ F::Time by 1 R::s = 1 F::s;` followed by `dimension F::Time ~ R::Time by 1 F::s = 1 R::s;` two forms in source order. `dimension Length;` and `dimension Accel = Length * Time^-2 / Mass;` still build dimension forms. `dimension R::Time;`, `dimension A ~ B ~ C;`, `dimension A ~;`, `dimension A ~ B = C;`, `dimension A ~ B;` (no `by` clause), `dimension A ~ B by 1 u;`, `dimension A ~ B by u = 1 v;`, `dimension A ~ B by 1 u = 1;` and `dimension A ~ B by 1 u = 1 v w;` each refuse `invalid_syntax` at the offending token and build no form. | Test (TC-482) |
| FR-091-AC-60 | Dimension equivalence decisions (QSpec FR-142-AC-22). With `model R = "acme/rpc" …` and `model F = "acme/flow" …`, each package declaring base dimension `Time` with root unit `s`, and a unit declaring `dimension Time; unit s : Time = rational(1, 1);`, the assembler admits `dimension R::Time ~ F::Time by 1 R::s = 1 F::s;`, `dimension R::Time ~ Time by 1 R::s = 1 s;`, and, with `dimension Dur = R::Time; unit dsec : Dur = rational(1, 1);` after `dimension R::Time ~ F::Time by 1 R::s = 1 F::s;`, `dimension Dur ~ F::Time by 1 dsec = 1 F::s;`; `dimension A ~ B by 1 ua = 1 ub;`, `dimension B ~ C by 1 ub = 1000 uc;` and `dimension C ~ A by 1000 uc = 1 ua;` over three source base dimensions with root units `ua`, `ub`, `uc` are admitted together. It refuses each of these with exactly one error at the equivalence form: `dimension R::Time ~ Z::Time by 1 R::s = 1 Z::s;` with no model alias `Z`, `dimension R::Time ~ R::Speed by 1 R::s = 1 R::mps;` where `acme/rpc` declares no `Speed`, `dimension R::Time ~ Nothing by 1 R::s = 1 s;` and `dimension R::Time ~ F::Time by 1 R::s = 1 F::second;` where `acme/flow` declares no `second`, `missing_declaration`/`missing-name` at the unresolved name; `dimension R::Time ~ R::Time by 1 R::s = 1 R::s;` and `dimension Time ~ Time by 1 s = 1 s;`, `invalid_package`/`definition-cycle` retaining the edge `node -> node`; `dimension F::Time ~ R::Time by 1 F::s = 1 R::s;` and `dimension F::Time ~ R::Time by 1 F::s = 2 R::s;` after `dimension R::Time ~ F::Time by 1 R::s = 1 F::s;`, `invalid_package`/`conflicting-definition` at the later form retaining both; and, with `acme/rpc` declaring `Area = Length ^ 2` and its root `m2`, `dimension R::Time ~ R::Area by 1 R::s = 1 R::m2;`, and `dimension Dur ~ F::Time by 1 dsec = 1 F::s;` with no `dimension R::Time ~ F::Time by 1 R::s = 1 F::s;`, `invalid_package`/`incompatible-definition` retaining both nodes. `dimension R::Speed ~ F::Speed by 1 R::mps = 1 F::mps;` over speeds that are `Length / Time` of their package, each `mps` the root of its speed, is admitted after `dimension R::Length ~ F::Length by 1 R::m = 1 F::m;` and `dimension R::Time ~ F::Time by 1 R::s = 1 F::s;` and refused `incompatible-definition` without them. A source declaring `dimension L; unit r : L = rational(1, 1); unit z : L = rational(0, 1);` and `dimension L ~ L by 1 r = 1 r;` refuses `definition-cycle` for the equivalence and not row 1 at `z`, because base pairs are decided before the first turn; the same source with `dimension Dur = L; unit rd : Dur = rational(1, 1); dimension L2; unit r2 : L2 = rational(1, 1); dimension Dur ~ L2 by 1 rd = 1 r2;` in place of `dimension L ~ L …;` refuses row 1 at `z` and not the equivalence's `incompatible-definition`, because every other pair is decided after the last turn. The decisions are the same in any source order of the equivalence forms within a phase. | Test (TC-483) |
| FR-091-AC-61 | The assembler mints keys for Q11, Q12 and Q13's sources under source owner (`a`, `u`) as the vector table gives: Q12 (`root_scale` 1000/1) and Q13 (1/200, the reciprocal of the ratio written for the descending order); `dimension Length ~ Distance by 1 km = 1 dist;` and `dimension Distance ~ Length by 1 dist = 1 km;` give Q12's one key, the `dimension_node_ids` in ascending key order and the ratio stated for the node listed first, and `by 1 dist = 1000 m` gives it too (one ratio, two unit spellings); Q1's and Q11's keys are the same with and without the equivalence; a dimension `Speed = Length / Time` keeps its `terms` and key (Q3's) beside `dimension Length ~ Distance by 1 km = 1 dist;`. Over the package dimensions of QSpec's `model-dimension-acme-rpc-time`, `model-dimension-acme-flow-time` and `model-dimension-acme-clock-time` and owner `{kind: "source", authority: "agent-ix", identity: "example"}`, `dimension R::Time ~ F::Time by 1 R::s = 1000 F::ms;` (flow's `ms` is `1/1000` of its root `s`) mints the key whose `dimension_node_ids` are the two package dimension keys in ascending key order, `dimension R::Time ~ G::Time by 1 R::s = 1000 G::ms;` (clock's root is `ms`) mints the key of its own preimage in the same way, and `dimension F::Time ~ G::Time by 1 F::s = 1000 G::ms;` does likewise, each `root_scale` stated for the node listed first and each key equal to its preimage value. `nominal_spans` holds the declaration span of each equivalence under its key. | Test (TC-483) |
| FR-091-AC-62 | Effect on the admitted graph and the base-dimension rule (QSpec FR-142-AC-24). With `acme/rpc` declaring base dimension `Mass` with root unit `kg` and `acme/flow` declaring base dimension `Mass` with no unit, `dimension R::Mass ~ F::Mass by 1 R::kg = 1 F::kg;` refuses `missing_declaration`/`missing-name` at `F::kg`, and `dimension Force = F::Mass; unit n : Force = rational(1, 1);` refuses `missing_declaration`/`missing-name` by the base-dimension rule at `F::Mass`'s turn, naming `ix://acme/flow/dimension/Mass` and `n`, with or without any equivalence. The admitted `UnitGraph` over `dimension R::Time ~ F::Time by 1 R::s = 1 F::s;` holds the equivalence node beside the three `Time` nodes, derives one class of `R::Time` and `F::Time` whose class root is the canonical root of the member holding the least dimension node key, and derives one class of `Distance` and `Length` for Q12; a graph with no equivalence derives no class. | Test (TC-483) |
| FR-091-AC-63 | Ratio checks (QSpec FR-142-AC-22 steps 4, 5, 6 and 7). With the packages of FR-091-AC-60 and `acme/rpc` declaring `unit m : Length = rational(1, 1)`, the assembler refuses `invalid_package`/`incompatible-definition` at the form, retaining the unit and the dimension node it was named for, for `dimension R::Time ~ F::Time by 1 R::m = 1 F::s;` and `dimension R::Time ~ F::Time by 1 F::s = 1 R::s;` (each unit named for the other dimension); `invalid_package`/`invalid-value`, retaining the unit and its composed offset `5` or the number, for `dimension R::Time ~ F::Time by 1 R::s = 1 F::local;` where `acme/flow` declares `unit local : Time = rational(1, 1) * s + rational(5, 1);`, for `by 0 R::s = 1 F::s` and for `by rational(-1, 1) R::s = 1 F::s`; `invalid_package`/`incompatible-definition` retaining both nodes for the speeds of FR-091-AC-60 declared `by 1 R::mps = 2 F::mps` after the base pairs (the base roots fix the ratio 1), and the same speeds `by 1 R::mps = 1 F::mps` admitted; and, over three source base dimensions with roots `ua`, `ub`, `uc`, `invalid_package`/`conflicting-definition` at the third form, retaining all three in source order, for `A ~ B by 1 ua = 1 ub;`, `B ~ C by 1 ub = 1000 uc;`, `C ~ A by 1 uc = 1 ua;`. A ratio written through a larger unit is the same ratio: `by 1 R::s = 1 F::s` and `by 1 R::s = 1000 F::ms` (flow's `ms` is `1/1000` of its root) are both admitted and mint one key, and a source unit `unit tenth : A = rational(1, 10) * ua;` gives `dimension A ~ B by 1 tenth = 1 ub` the ratio 10 (one root of `A` is ten roots of `B`). | Test (TC-483) |
| FR-091-AC-65 | For a source name represented by a `NameForm`, S2 preserves its exact spelling, its whole-name span, and its ordered identifier segments as separately available values: `T` has `["T"]`, `M::T` has `["M", "T"]`, and `A::M::T` has `["A", "M", "T"]`. One segment denotes an unqualified name; two or more denote a qualified name. Expression-name forms and other source references represented by `NameForm` expose the same segment structure. Every S3 branch that depends on whether one of these source names is qualified reads that structure; it does not inspect the spelling for `::`. Existing name-resolution and refusal outcomes remain unchanged. | Inspection |

## Dependencies

- [TC-1052](../test-cases/TC-1052-native-string-uuid-and-timestamp-literals.md)
  verifies the native scalar literals.
- **Upstream:** [FR-067](FR-067-add-s2-forms-and-retire-seam-5.md) built the
  `forms` core: the dispatch table, `ParsedForm`'s span, edition and
  declared-extent accessors, and the `RecoveringCst` and `NoDispatchEntry`
  causes. This requirement adds the `Value` family's entries and production
  (M-3b), which FR-067-CON-2 leaves to the family, and refines FR-067's
  dispatch key to each declaration's leading token.
  [FR-068](FR-068-split-expression-checking-into-check-stage.md) placed
  `PackageDeclarations` and its `check` in the layer-3 `check` core.
- The K5 slice replaces the `ValueType` fields of the `forms` types
  with a syntactic type form, and gives `PackageDeclarations` a check-owned
  resolved signature. This requirement states the same direction.
- [FR-065](FR-065-migrate-function-application-to-checked-family.md) takes
  a family's parsed forms as its input. This requirement produces them from
  source for the `Value` family.
- ADR-013 T-4's `Staged`, `StageFailure` and `LimitExceeded` are #213 S-5b's
  ([FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)).
  T-5's `Locus` and O-12's `SourceRegion` are #213 S-4's. The
  criteria above observe a refusal's cause, limit kind, bound and byte span,
  whichever carrier holds them.
- The unit's `SourceOwner` is the `authority` and `identity` of the unit's
  `RawSourceRef`, which the caller names and S0 mints
  ([FR-001](FR-001-read-exact-source.md), ADR-013 §7 slice S-4b).
- Record, tuple and function keys over `SourceOwner` use QSL's
  `quire.structural-node/v1` preimage
  ([FR-092](FR-092-key-type-parameter-and-declared-nodes.md)). QSpec
  publishes an owner-bearing preimage for nominal enum, dimension and unit
  nodes only; the structural-node preimage is proposed to
  `ix://agent-ix/quire-specification` under ADR-013 QC-18 and QC-24.
- The enum entries rely on `value::enumeration` computing preimages
  and never minting a key (ADR-011 FB-13), and on lowering
  building an admitted `EnumBinding`'s declaration and member nodes from
  its preimage (FR-092 rule 1, QSL-238's enum half).
- The dimension and unit hand-off: the assembler ends at an admitted
  `UnitGraph` in `PackageDeclarations::units`, which keeps each unit's,
  dimension's and dimension equivalence's preimage bytes; FR-094's lowering
  builds their nodes.
- QSpec FR-142 "Dimension and conversion algebra" (the unit-graph topology
  rules, order and causes), "Dimension equivalence" (the declaration, its
  four decision steps and its effect) and "Quantity type reference" (the
  complete-V1 source form `Quantity<u>`), and the shared grammar's
  `dimension-equivalence` production.
- FR-091-AC-17's code needs the QSpec catalog to name an alias cycle under
  `definition-cycle`. The catalog scopes that cause to definition
  dependencies and to FR-151 dispatch cycles, and has no row for a type
  alias. The one-sentence catalog extension is proposed to
  `ix://agent-ix/quire-specification`.
- QSpec's `quire.native.diagnostics/v1` catalog defines both
  `resource_exhausted` storage causes: `allocation-failed` with `requested`
  and `reservation-size-overflow` with `upstream` (QSpec PR #236;
  the capacity cause was added by
  QSpec PR #234). QSL preserves the upstream `StorageRequest` amount and unit
  in each single payload field, rendered as `<unit>:<amount>`.
- **Downstream:** the ADR-011 §5 stage driver calls S1, S2, the assembler
  and `check` in that order.

## Rulings

Lead rulings of 2026-09-22 on the open questions below. Each row is
decided; the last column names the fact that reopens it.

| Question | Ruling | Reason | Reopen when |
| --- | --- | --- | --- |
| FR-091-OQ-1, nested constructs | S2 builds the `Deref`, `Pre` and `AllInstances` variants that the `forms` core defines. The check stage refuses each with the owning family's catalogued cause, never `unsupported_construct`. `pre(e)` in a function body is `wrong_snapshot`/`forbidden-pre-read`, a `ProtocolClause` cause. | The catalog forbids a producer from choosing a broader listed code to discard a distinction it knows. FR-090 makes `forbidden-pre-read` a `ProtocolClause` cause. | ADR-012 §3's rule that a family owns its grammar productions is ruled to cover nested sub-expressions. |
| FR-091-OQ-1, declarations | `Value` owns the `enum`, `dimension`, `unit` and `predicate` declaration productions. The `enum`, `dimension` and `unit` entries need their key minting inside `check` (FB-13). | QSpec FR-322 classes enum, dimension and unit as `scalar_type` nodes. `predicate` is a function form with a `Boolean` result. | QSpec makes an enum a `union-decl` case rather than a `scalar_type`. |
| FR-091-OQ-2 | S2 carries each unit's profile, import and model selections, and each form keeps its `using` alias. E3 resolves every alias to a declared profile selection, or refuses with `missing_declaration`/`missing-selection`. ADR-011 §2.2's E2 Version cell reads "Edition and the unit's profile, import and model selections carried". The forms `FunctionDeclaration` has a `using` field. | ADR-011 OBS-007 makes S2 the only source of check-stage input from source. QSpec requires `using` to name a declared alias, with no default. | QSpec makes the compile request's input inventory, not each unit, the source of selections. S2 still carries each unit's selections for the driver to compare, and only the lock assembly moves. |
| FR-091-OQ-3 | Record, tuple and function node keys are minted over `SourceOwner{authority, identity}`. ADR-013 O-04 and ADR-012 §2 state the owner reading of QC-18. `check` has no `DEFAULT_PACKAGE_IDENTITY`. | Neither the source grammar nor the v2 wire carries a package name, so replay could not rebuild a `name@version` key. QSpec's `proposals/checked-package-v2/README.md` publishes the owner reading for nominal enum, dimension and unit nodes; for `composite_type` and `function` nodes QSL keys by its proposed `quire.structural-node/v1` preimage, which carries the owner (FR-092). | QSpec adds a package name to source or to the v2 wire, or a declaration must keep its key when it moves between the units of one package. |
| FR-091-OQ-4 | `ValueType::Float` carries the rounding mode, in `quire-exact` (`FloatType`) and in QSL, and the evaluator applies it (done). A bare `Float32` or `Float64` is strict `exact`, and `qsl-cst` accepts it. | QSpec FR-322 makes the rounding mode part of the type, and FR-148 requires the evaluator to use it. | QSpec FR-322 moves float rounding off `type_pinned_modes`. |
| FR-091-OQ-5, integer operators | Resolved: S2 maps `div`, `rem` and `mod` to distinct forms regardless of reachability. Check uses the declaration's resolved `using` selection: absent `quire.value.complete/v1` refuses each at its operator locus; selected complete value admits well-typed forms. The checked package's exactly-one integer-division law governs `div`/`rem`; `mod` stays Euclidean. Their checker and evaluator arms are required by FR-091-AC-39 and QSpec FR-147. `/` has no integer-division meaning; QSpec FR-149 owns exact integer-to-rational promotion only under complete value. | QSpec FR-147 and its shared grammar admit these operators only under the complete-value facet, while QSpec FR-039 preserves the state-only refusal. S2 must not discard the distinction before profile admission. | QSpec changes the facet, operator law or slash rule. |
| FR-091-OQ-5, other constructs | `collect` maps to `Query{Map}`. `reaches` is a `StateModel` construct with no variant; it and the other listed unrepresented constructs are refused with `UnrepresentedConstruct`. A variant for one of them comes with its checker and evaluator arms. `e[i]` depends on QSpec semantics. | QSpec FR-145 makes `map` and `collect` one operation. QSL FR-008-AC-20's duplicate-output `collect` refusal belongs to the native lane, not to complete-V1; QSpec FR-008-AC-5 refuses `collect` only where `quire.state.queries/v1` is selected without `quire.value.complete/v1`. | M-6a must compile existing native programs that use the refused constructs. |
| FR-091-OQ-7 | An alias cycle is `invalid_package`/`definition-cycle` at the check stage. | The check stage uses the same code for an FR-151 dispatch call-graph cycle (`qsl-semantics/src/check/refusal.rs`, `CheckCause::DefinitionCycle`), and the cause's payload is dependency edges. | QSpec declines the catalog extension and keeps `definition-cycle` to `DefinitionRef` closures. |
| FR-091-OQ-11 | Source dimensions and units (team lead ruling, 2026-09-26). (a) Dimension terms are normalized: derived terms expand to base terms, repeated exponents add, zeros drop, and terms sort by key. (b) `rational`/`decimal` numbers are reduced, not refused; a zero denominator refuses as `undefined_expression`/`unproved-nonzero`. (d) Units are named as types through `Quantity<u>` (QSpec FR-142). Parts of (a) and (c) of this ruling are replaced by FR-091-OQ-13. | (a) matches FR-142's normalized base-dimension map, and a derived term is how acceleration is written from speed. (b) the reduced rational is the value written (ADR-013 R-07). (d) the complete-V1 `type-ref` names a quantity type as `Quantity<u>` over a unit (QSpec FR-142-AC-11), and reading any other type name as a unit would invent syntax meaning. A quantity-typed operation parameter is therefore the `Quantity<u>` annotation (FR-185). | QSpec changes or withdraws `Quantity<u>` (FR-142). |
| FR-091-OQ-13 | Unit-graph topology (lead default, 2026-10-22, not an owner answer). A derived dimension that cancels to no terms is the dimensionless dimension, admitted. The dimension and unit topology, including dimension term resolution, dimension and unit cycles, and package dimensions and units as imported nodes, takes QSpec FR-142's rules, order and causes. | QSpec FR-142 fixes the topology rules, their first-defect order and their causes, and admits `N = L / L` (FR-142-AC-15 to AC-19); following it gives every checker of the language one refusal for one unit graph. | QSpec changes FR-142's topology rules or causes. |
| FR-091-OQ-14 | Dimension equivalence in the assembler (lead default, 2026-10-23, not an owner answer; Peter may veto). The assembler decides every `dimension A ~ B by x U = y V;` by QSpec FR-142's seven steps inside the topology check, in the two phases FR-142 fixes, and treats a pair that step 1 refuses as decided in the first phase. An equivalence form is no node of the turn list, so a package dimension that only an equivalence names takes its turn with the unreached package dimensions. A unit whose target chain a row 1, 2, 3 or 6 defect breaks fixes no scale, so the equivalence naming it is decided up to step 4 and the unit's dimension reports the defect at its turn. `UnitGraph::admit` holds the equivalence nodes (with their `root_scale`) and derives the classes and the class-root edges; no dimension key or `terms` list changes. | FR-142 fixes the steps and the phases but states neither where a step-1 refusal of a non-base pair falls, whether an equivalence reaches a package dimension for the turn walk, nor how a ratio is read through a defective chain; these three readings give one refusal per unit graph and change no admitted result. | QSpec states any point differently. |

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
  producer does not represent. The Catalog codes table keeps
  `unsupported_construct` for these two causes.
