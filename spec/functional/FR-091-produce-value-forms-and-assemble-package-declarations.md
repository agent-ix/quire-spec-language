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

Each entry makes exactly one call into the `Value` family's production
function (FR-067-CON-4, verified by inspection). When this table lands, the
`record` entry replaces the M-3a no-entry fixture token FR-067-AC-3's test
uses, so that test moves to a spelling no family claims.

When a declaration's leading token has no entry, S2 refuses the whole unit.
The cause is `NoDispatchEntry`, and the refusal names the token spelling and
the declaration's span. S2 returns no form for any declaration of that unit.

The `Value` family also owns the `enum` (including `ordered enum`),
`dimension`, `unit` and `predicate` declaration productions. No other family
has a dispatch entry, production function or parsed-form type for them.
Enum, dimension and unit declarations are value types (QSpec FR-322 classes
them as `scalar_type` nodes), and a `predicate` is a function form with a
`Boolean` result. The table above has no entry for these four, so S2
refuses them with `NoDispatchEntry`. An `enum`, `dimension` or `unit`
entry admits a declaration whose `NodeKey` only `check` mints (FB-13).

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
- `functions`: one entry per `FunctionDeclaration`, in source order, each
  name-callable with clause kind `Body`. Each entry carries a check-owned
  resolved signature: the parameter types and the result type that check
  resolved from the type forms, and the profile selection its `using`
  alias resolves to. The resolved signature is a `check` type, not a
  `forms` type.
- `types`: one composite declaration per record form and tuple form. Each
  has a declaration key that `check` mints (FB-13, ADR-013 O-04).

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
owner ([FR-092](FR-092-key-type-parameter-and-declared-nodes.md)). QSpec
publishes owner scope for nominal enum, dimension and unit nodes; the
structural-node preimage is a QSL proposal to QSpec (Dependencies).

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
  resolves to `ValueType::Composite` of that declaration's key.

The assembler refuses in these cases:

- **Unresolved type name.** A qualified name names no alias, record or tuple
  form of the unit and nothing in an admitted domain package. The refusal
  names the name and the span of the type form.
- **Ambiguous type name.** A qualified name names more than one alias,
  record or tuple form of the unit (ADR-013 O-11). The refusal names the
  name, the span of the referencing type form and the span of each
  candidate declaration.
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
`ambiguous-name`.

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
| FR-091-AC-6 | S2 refuses with cause `NoDispatchEntry`, naming the leading token and the declaration's span, an admissible unit whose only declaration is an `enum`, an `ordered enum`, a `predicate`, a `dimension` or a `unit` declaration. No module under `forms` other than the `Value` family form builder names the `EnumDeclaration`, `Predicate`, `DimensionDeclaration` or `UnitDeclaration` CST production. | Test (TC-395) |
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
| FR-091-AC-21 | `catalog_code()` on each S2 and assembler cause returns the code in the Catalog codes table, and matches every cause with no `_` arm. The diagnosed-source cause returns its diagnostic's own code. The floating-type cause returns `unknown_required_feature`, the undeclared-alias cause `missing_declaration`, the duplicate-alias cause `ambiguous_declaration`, and the alias-cycle cause `invalid_package`. S2's nesting-depth limit refusal reports `stage_limit_exceeded`/`nesting-depth-exceeded`. | Test (TC-406) |
| FR-091-AC-22 | For a unit with one profile selection, alias `v`, and `function f using v(): Boolean pure { true }`, the assembler records `f`'s `using` alias as resolved to that selection. With `function g using w(): Boolean pure { true }` added, it returns one refusal holding an undeclared-alias error, code `missing_declaration`/`missing-selection`, that names `w` and the span of `g`'s `using` field, and no `PackageDeclarations`. A unit that declares two profile selections with alias `v` refuses with a duplicate-alias error, code `ambiguous_declaration`/`ambiguous-name`, naming `v` and both selection spans. | Test (TC-412) |
| FR-091-AC-23 | S1 admits a unit whose function parameter is typed `Float32` or `Float64` with no `[mode]`, and S2 builds that parameter's type form with head `Float32` or `Float64` and no rounding mode. | Test (TC-405) |
| FR-091-AC-24 | Spine `compile` of a unit that declares `import "test/units" version "2" digest "<64 lowercase hex>" as u;`, with no library supplied as `test/units`, refuses at stage `intake`, before assembly, with `missing_import`/`missing-selection` naming `test/units` at the import's identity string (FR-099, ADR-015 D-1), and emits no package. | Test (TC-405) |

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
- An `enum`, `dimension` or `unit` entry needs `value::enumeration` and
  `value::unit` to leave the FB-13 debt list, which QSL-131 owns.
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

- FB-13 (QSL-141): `enum`, `ordered enum`, `predicate`, `dimension` and
  `unit` declarations still refuse `NoDispatchEntry` (FR-091-AC-6). Their
  parsed-form types, the assembler wiring of `EnumDeclaration::admit` and
  `UnitGraph::admit`, and the retained unit and dimension preimages
  (QSL-238, QSL-64) land with the post-G2 M-6 lanes.
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
