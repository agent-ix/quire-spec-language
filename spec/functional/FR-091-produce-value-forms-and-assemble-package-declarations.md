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
  none; edition carried; declared bounds and extents carried as syntax. E3
  row: QSL keys the source map by occurrence key; bounds and extents typed.
  E9 row: the packet's occurrence key resolves to a nested span.
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

The complete-V1 grammar that `qsl-cst` transcribes (`qsl-cst/src/grammar.rs`)
is the authority for the syntax this requirement maps.

## Inputs

- The S1 output of one complete-V1 unit, `qsl_cst::ParsedSource`, whose CST
  is a `LosslessCst`.
- An explicit S2 limit set, which holds at least a nesting-depth bound.
- For the assembler: the parsed unit that S2 returned for that source.

## Outputs

- From S2: one parsed unit. It holds the header's edition and, in source
  order, one `Value`-family parsed form per declaration.
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
function. FR-067-AC-3's shape test covers every entry in the table.

When a declaration's leading token has no entry, S2 refuses the whole unit.
The cause is `NoDispatchEntry`, and the refusal names the token spelling and
the declaration's span. S2 returns no form for any declaration of that unit.

### What a `Value` parsed form carries

Every `Value` parsed form carries the span of its `Declaration` CST node and
the unit's edition. It carries no semantic identity (FR-067-AC-2).

- **Function form.** This is the `forms` core's one `FunctionDeclaration`
  type. It carries the declared name; the `using` alias as written; each
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
`forms` has no second function type that holds `ValueType`.

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
| `map`, `filter`, `flatMap`, `forall`, `exists` `(x in c: e)` | `Query` with `BinderQuery` `Map`, `Filter`, `FlatMap`, `Forall`, `Exists` |
| `flatten(e)` | `Flatten` |
| `fold<A>(acc, x in c: s, identity: i)`, `reduce<A>(acc, x in c: s)` | `Accumulate`, with the identity when written |
| `count<N>(x in c: p)`, `sum<N>(x in c: e)` | `Count`, `Sum` |
| `size(e)` | `Size` |
| `contains(c, v)` | `Contains` |
| `convert<T>(e)` | `Convert`, with `T` carried as a type form |
| `(e)` | the mapping of `e` |

Grouping and precedence come from the CST's own nesting. The production
never re-derives them from token order.

`deref(e)`, `pre(e)` and `allInstances<T>(e)` have `Expression` variants
that other families own: `Deref` and `AllInstances` belong to `StateModel`,
and `Pre` to `ProtocolClause` (ADR-012 §1, §3, §4.3). When one of these is
nested in a `Value` declaration, the `Value` production refuses the whole
unit with cause `ForeignFamilyConstruct`. The refusal names the construct,
its owning family and its span (FR-091-OQ-1).

A construct that no family's variant represents refuses the whole unit with
cause `UnrepresentedConstruct`, naming the construct's CST production and
span. These constructs are text literals, `decimal(..)`, `float32(bits: ..)`
and `float64(bits: ..)`, `null` outside a record field value, `none`,
`self`, `result`, `div`, `rem`, `mod`, indexing `e[i]`, `size<T>(e)`,
`reaches(..)` and `collect(..)` (FR-091-OQ-5). S2 returns no form for any
declaration of that unit.

### S2 depth limit

S2 bounds its recursion by the nesting-depth bound `L` in its limit set, not
by the native stack (ADR-011 §2.3 Limits). Depth counts `Expression` nodes.
The root expression of a function body or measure is at depth 1, and each
child is one deeper than its parent. A body whose deepest node is at depth
`L` builds. A body with a node at depth `L + 1` gives a limit refusal that
names limit kind nesting depth, the bound `L`, and the span of the first node
at depth `L + 1` in source order. It is never a truncated form and never a
`NoDispatchEntry`, `ForeignFamilyConstruct` or `UnrepresentedConstruct`
refusal. The S2 bound applies to sources that S1 admits: S1's own nesting
clamp is independent of it.

### S2 layering

The `Value` family form builder is a module under `forms`. ADR-011 §6.1
allows layer 2 to depend on layer 1 (`qsl-cst`), F (`qsl-foundation`) and K
(`quire-exact`), and on its own layer's `forms` core. The builder depends on
no other family module and on nothing in layers 3 to 6. No `Value` parsed
form, type form or `Value`-owned `Expression` variant has a field of type
`ValueType` or `NodeKey`.

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
  resolved from the type forms. The resolved signature is a `check` type,
  not a `forms` type.
- `types`: one composite declaration per record form and tuple form. Each
  has a declaration key that `check` mints (FB-13, ADR-013 O-04;
  FR-091-OQ-3).

Type resolution is the E3 name-binding phase (ADR-011 §1; ADR-013 O-11).
The resolution `match` has one explicit arm per type-form head:

- a built-in constructor (`Boolean`, `Integer`, `Int[..]`, `Rational[..]`,
  `Decimal[..]`, `Text[..]`, `Option<..>`, `Sequence`/`Set`/`Bag`/
  `OrderedSet<..>[..]`) resolves to its `ValueType` over the declared bounds
  the type form carries;
- `Float32[mode]` and `Float64[mode]` refuse. `ValueType::Float` holds only
  an `IeeeWidth` and cannot represent the rounding mode, and R-07 forbids
  dropping it. The refusal names the rounding mode and the type form's
  span (FR-091-OQ-4);
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
- **Alias cycle.** An alias resolves through a chain of aliases back to
  itself. The refusal names the aliases on the cycle.

A refusal carries every error the assembler found in the unit, not only the
first (ADR-011 §2.3 E3). The assembler returns no `PackageDeclarations`
value alongside a refusal. The assembler runs before check mints any
occurrence key, so each assembler error names its location as a
`Locus::Region` over the type form's or declaration's span (ADR-013 T-5).

### Catalog codes

Each S2 and assembler cause has one exhaustive `catalog_code()` (ADR-013
O-17). The codes are these existing `qsl-foundation` catalog codes:

| Cause | Code |
|---|---|
| `RecoveringCst` | `invalid_syntax` |
| diagnosed source (diagnostic, no recovery) | the first diagnostic's own code |
| `NoDispatchEntry`, `ForeignFamilyConstruct`, `UnrepresentedConstruct` | `unsupported_construct` |
| unresolved type name | `missing_declaration` |
| ambiguous type name | `ambiguous_declaration` |
| ill-formed scalar bounds | `ill_typed` |
| floating type | `unsupported_construct` |
| S2 nesting-depth limit | the nesting-depth limit code that ADR-013 QC-11 assigns (#213 S-5b) |
| alias cycle | FR-091-OQ-7 |

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-091-CON-1 | The `Value` family form builder and the assembler are the only producers of a `PackageDeclarations` value from complete-V1 source. The assembler's parsed-form input is S2 output. Its other E3 inputs (admitted domain packages, library lock) are those ADR-011 §2.1's E3 row admits, subject to FR-091-OQ-2 and FR-091-OQ-3. | Design | Inspection |
| FR-091-CON-2 | The dispatch table, the `Value` production's expression match and the assembler's type-form resolution match have no `_` or catch-all arm (ADR-012 §5.1). | Design | Test (TC-398) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-091-AC-1 | S2 returns a parsed unit with exactly four forms, in the order alias, function, record and tuple, for an admissible S1 output whose header records edition `1-draft` and which holds one profile selection with alias `v` and then, in order, one `type`, one `function`, one `record` and one `tuple` declaration. Each form's span equals the span of its `Declaration` CST node, and the unit's edition reads `1-draft`. | Test (TC-392) |
| FR-091-AC-2 | The `forms` `FunctionDeclaration` that S2 builds from `function inc using v(x: Int[0, 9], y: Digit): Int[0, 10] pure decreases(x) { x + 1 }` reads: name `inc`; `using` alias `v`; parameter `x`, a type form with head `Int` and bounds spelled `0` and `9`; parameter `y`, a type form with qualified-name head `Digit`; result, a type form with head `Int` and bounds `0` and `10`; measure `Name("x")`; body `Binary{Add, Name("x"), Integer(1)}`. No field of `FunctionDeclaration` or of the type form, and no accessor return type on either, is `ValueType` or `NodeKey`. | Test (TC-393) |
| FR-091-AC-3 | For each row of the expression-mapping table, a function body that holds that construct maps to the listed `Expression` variant, with operands in source order. `a or b and c` maps to `Or(a, And(b, c))`. `a - b - c` maps to `Subtract(Subtract(a, b), c)`. `a implies b implies c` maps to `Implies(a, Implies(b, c))`. `(a + b) * c` maps to `Multiply(Add(a, b), c)`. | Test (TC-394) |
| FR-091-AC-4 | S2 refuses a `ParsedSource` whose CST carries a recovery with cause `RecoveringCst`. It refuses an admissible parse, to which `prepend_diagnostic` has added one diagnostic, with the diagnosed-source cause holding that diagnostic's code. Neither refusal returns a parsed unit. | Test (TC-395) |
| FR-091-AC-5 | S2 refuses the whole unit with cause `NoDispatchEntry` for an admissible unit holding a valid `function` declaration followed by `invariant Positive using v on M::T at current { true }`. The refusal names the spelling `invariant` and that declaration's span, and S2 returns no form for the `function` declaration. | Test (TC-395) |
| FR-091-AC-6 | S2 refuses with cause `NoDispatchEntry`, naming the leading token and the declaration's span, an admissible unit whose only declaration is an `enum`, a `predicate`, a `dimension` or a `unit` declaration. Open question: FR-091-OQ-1. | Test (TC-395) |
| FR-091-AC-7 | S2 refuses the whole unit with cause `UnrepresentedConstruct`, naming the construct's CST production and span and returning no form, for a function body that holds exactly one of: a text literal, `decimal(1, 2)`, `a mod b`, `xs[0]`, `none`, `collect(x in c: x)`. Open question: FR-091-OQ-5. | Test (TC-396) |
| FR-091-AC-8 | S2 refuses the whole unit with cause `ForeignFamilyConstruct`, naming the construct, its owning family (`StateModel` for `deref(r)` and `allInstances<M::T>(p)`, `ProtocolClause` for `pre(a)`) and its span, for a function body that holds one of those constructs. Open question: FR-091-OQ-1. | Test (TC-396) |
| FR-091-AC-9 | With S2 nesting-depth bound `L = 8`, S2 builds a function body `not`×7 `a`, whose deepest node is at depth 8. It refuses `not`×8 `a` with a limit refusal that names limit kind nesting depth, bound `8`, and the span of the node at depth 9, and returns no parsed unit. It refuses `not`×20 `a` in the same way. S1 admits all three sources, so each refusal comes from S2. | Test (TC-397) |
| FR-091-AC-10 | Every `Expression` node in a `Value` form carries the span of the CST node it maps from. For the body `if a then b else c + d`, the `If` node's span covers the whole body, the `Add` node's span covers `c + d`, and the `Name("d")` node's span covers `d`. For `(a + b) * c`, the `Add` node's span covers `a + b`, without the parentheses. | Test (TC-403) |
| FR-091-AC-11 | The `Value` family form builder module has `use` edges and inline paths only to the `forms` core, `qsl_cst`, `qsl_foundation` and `quire_exact`. It has none to another family module or to anything under `crate::check`, `crate::value` or `crate::model`. No field of a `Value` parsed form, of the type form, or of an `Expression` variant in the mapping table has type `ValueType` or `NodeKey`. The dispatch `match`, the expression `match` and the assembler's resolution `match` have no `_` arm. | Test (TC-398) |
| FR-091-AC-12 | The assembler returns a `PackageDeclarations` value for the S2 output of source that declares `type Digit = Int[0, 9];`, then `function inc using v(x: Digit): Int[0, 10] pure { x + 1 }`, then `function two using v(): Int[0, 10] pure { inc(1) }`. Its `aliases` is `[("Digit", Int[0..9])]`. Its `functions` are `inc` and then `two`. `inc`'s check-owned resolved signature has parameter type `Int[0..9]` and result type `Int[0..10]`. | Test (TC-399) |
| FR-091-AC-13 | Calling `two` with no arguments through `CheckedPackage::call`, on the package that `PackageDeclarations::check` admits and links from the AC-12 assembler output, returns a completed outcome with integer value `2`. | Test (TC-399) |
| FR-091-AC-14 | The assembler returns one refusal holding two unresolved-type-name errors for a unit with `function f using v(x: Missing): Boolean pure { true }` and `function g using v(y: Absent): Boolean pure { true }`. One error names `Missing` with its type form's span, and the other names `Absent` with its type form's span. It returns no `PackageDeclarations`. | Test (TC-400) |
| FR-091-AC-15 | The assembler refuses with an ambiguous-type-name error for a unit with `type A = Int[0, 1];`, `type A = Int[0, 2];` and `function f using v(x: A): Boolean pure { true }`. The error names `A`, the span of `x`'s type form and the spans of both `type A` declarations. | Test (TC-400) |
| FR-091-AC-16 | The assembler returns one refusal with three errors for parameters typed `Int[9, 0]`, `Rational[0, 1; 0, 5]` and `Text[5, 1; nfc]`, one per function. Each error carries the owning value type's own rejection cause and its type form's span. It returns no `PackageDeclarations`. | Test (TC-400) |
| FR-091-AC-17 | The assembler refuses with an alias-cycle error naming both `A` and `B` for `type A = B;` and `type B = A;`, and returns no `PackageDeclarations`. | Test (TC-400) |
| FR-091-AC-18 | For `record Point { x: Int[0, 9]; y: Int[0, 9]; }`, `tuple Pair(Int[0, 9], Int[0, 9]);` and `function px using v(p: Point): Int[0, 9] pure { p.x }`, the assembler's `types` holds one record declaration `Point` and one tuple declaration `Pair`, each with a key that `check` minted. `px`'s resolved parameter type is `ValueType::Composite` of `Point`'s key, and `PackageDeclarations::check` admits the package. Open question: FR-091-OQ-3. | Test (TC-401) |
| FR-091-AC-19 | The assembler refuses a parameter typed `Float64[nearest-even]` with a floating-type error that names the rounding mode `nearest-even` and the type form's span. It refuses a parameter typed `Reference<M::T>`, in a unit with no admitted domain package, with an unresolved-type-name error naming `M::T`. Neither returns a `PackageDeclarations`. | Test (TC-405) |
| FR-091-AC-20 | The assembler module is under the layer-3 `check` core. Its non-test code has no `use` edge or inline path to `qsl_cst`. Its `#[cfg(test)]` code may reach `qsl_cst` only to run S1 and S2. | Test (TC-402) |
| FR-091-AC-21 | `catalog_code()` on each S2 and assembler cause returns the code in the Catalog codes table, and matches every cause with no `_` arm. The diagnosed-source cause returns its diagnostic's own code. The alias-cycle cause is outside this criterion (FR-091-OQ-7). | Test (TC-406) |

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
  (QSL-160). T-5's `Locus` and O-12's `SourceRegion` are #213 S-4's. The
  criteria above observe a refusal's cause, limit kind, bound and byte span,
  whichever carrier holds them.
- **Downstream:** the ADR-011 §5 stage driver calls S1, S2, the assembler
  and `check` in that order.

## Status

Specified for ADR-011 §7.3 M-3b (the `Value` family, tracked under QSL-141)
on the M-6a path (QSL-8). Not yet implemented. `forms::build_form` has a
test-only dispatch entry only, so every production CST refuses with
`NoDispatchEntry`. No module builds `PackageDeclarations` from parsed forms.
`Expression::Convert` and `FunctionDeclaration` hold `ValueType` fields,
which AC-2 and AC-11 do not allow. `Expression` nodes carry no span.

## Open Questions

- **FR-091-OQ-1: What does the `Value` production do with another family's
  construct, and who owns `enum`, `predicate`, `dimension` and `unit`?**
  ADR-012 §4.3 assigns the `Deref` and `AllInstances` variants to
  `StateModel` and `Pre` to `ProtocolClause`. It does not say whether the
  `Value` production refuses such a construct nested in a `Value`
  declaration, or hands it to the owning family's production. ADR-012 §1
  and §3 do not name the family that owns the `enum`, `predicate`,
  `dimension` and `unit` declaration productions. `enum` is held by
  `PackageDeclarations::enums`, and `predicate` is grammatically a
  Boolean-valued function declaration. FR-091-AC-6 and AC-8 depend on this.
- **FR-091-OQ-2: What do the unit's profile, import and model selections and
  a function's `using` alias become at E2 and E3?** The E2 row carries only
  the edition in its Version column. E3 admits "library lock" beside the
  parsed forms, and no FR says where a source unit's lock comes from
  (recorded on QSL-6). `using <alias>` is not trivia, so E2 cannot drop it;
  AC-2 has the form carry it as written. No text says what E3 does with it.
  `ieee_profile` is built by `DefinitionLock::admit_ieee_profile`, which
  needs the lock. No criterion covers `ieee_profile`.
- **FR-091-OQ-3: Over which package identity does `check` mint record and
  tuple declaration keys?** ADR-013 O-04's node-identity preimage includes
  the declaring package's `name@version` (QC-18). No FR says where the
  assembler gets a package's name and version for a unit compiled from
  source; this is the same lock-evidence gap as FR-091-OQ-2. FR-091-AC-18
  depends on this.
- **FR-091-OQ-4: Where is a floating type's rounding mode represented?**
  The grammar writes `Float32[mode]` and `Float64[mode]`, and
  `ValueType::Float` holds only an `IeeeWidth`. Under R-07 the assembler
  refuses floating types (AC-19). Whether the mode belongs in the value
  type, in the IEEE profile or elsewhere is not ruled (see QSL-131 for the
  kernel `ValueType` shape).
- **FR-091-OQ-5: Does the `Value` production represent the rest of the
  complete-V1 `Value` syntax?** Text, decimal and float literals, `none`,
  `null` outside a record field, `self`, `result`, `div`, `rem`, `mod`,
  indexing, `size<T>`, `reaches` and `collect` have grammar productions and
  no `Expression` variant. Adding a variant also needs checker and
  evaluator support. For `collect`, `src/forms/syntax.rs` documents
  `BinderQuery::Map` as "also spelled `collect`", while the native lane
  gives `collect` its own duplicate-output refusal (FR-008-AC-20). The two
  texts disagree on whether `collect` is `Map`. FR-091-AC-7 states what
  happens today: the production refuses.
- **FR-091-OQ-7: Which catalog code does an alias cycle map to?** ADR-013
  O-17 requires a code from the catalog revision FR-322 selects and forbids
  inventing one. No existing `qsl-foundation` code clearly names a cyclic
  type alias. A catalog entry is needed from
  `ix://agent-ix/quire-specification` (ADR-013 O-17).
- **FR-091-OQ-9: Do type-form bounds carry kernel values or spelled text?**
  A type form carries every declared bound "as spelled", and the ADR-011
  §2.2 E2 row carries declared bounds and extents "as syntax". Expression
  literals carry `quire_exact::Integer` values so that E3 never reads token
  text (ADR-011 §3 FB-01; ADR-013 OQ-A). Resolving a spelled bound at E3
  reads token text in the same way. FR-091-AC-2 and AC-16 observe bounds as
  spelled.
