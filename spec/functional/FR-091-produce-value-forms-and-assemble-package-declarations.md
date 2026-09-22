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
through S1, S2 and E3 and through no other path.

The requirement carries testable criteria for decisions already taken:

- ADR-011 §2.1 E2 and E3 rows: the admitted input and owner of each edge.
- ADR-011 §2.2 E2 row. Provenance is carried: each form holds the span of its
  CST node. Identity: none. The edition is carried. Declared bounds and
  extents are carried as syntax. E3 row: bounds and extents are typed.
- ADR-011 §2.3. E2 refuses with diagnostics and builds no partial form. E3
  refuses with every error diagnostic and yields no partial output. Every
  stage entry takes explicit limits, and recursive stages (S2 among them)
  bound their depth by an explicit limit.
- ADR-011 §1: name binding is a phase inside S3.
- ADR-011 §3 FB-01: no stage after S1 reads source text, CST or token text
  to recover meaning. FB-13: only `check` calls the kernel `NodeKey`
  constructor.
- ADR-011 §6.1. `forms` core < family form builders is layer 2 and depends
  on layer 1 and F. A family module depends on its layer's core only. The
  `check` core is layer 3.
- ADR-012 §1 and §3: the `Value` family covers scalar and composite values,
  types, expressions and function application. Its parsing forms are
  literals, operators, `let`, `if`, calls, records, collections and function
  declarations.
- ADR-012 §3 and §4.3. The parser composes families through one closed entry
  table keyed by the leading-token kind. Each entry makes one call into one
  family production. The one `Expression` enum is defined in the `forms`
  core, and each variant's owning family adds and removes that variant.
- ADR-013 T-4 and T-5: a stage result is its output, a refusal with typed
  causes, a limit refusal (`LimitExceeded`: limit kind, configured bound,
  locus) or an internal fault. S0 to S2 name a location by a source region.

The complete-V1 grammar that `qsl-cst` transcribes (`qsl-cst/src/grammar.rs`)
is the authority for the syntax this requirement maps.

## Inputs

- The S1 output of one complete-V1 unit, `qsl_cst::ParsedSource`, whose CST
  is a `LosslessCst`.
- An explicit S2 limit set, which holds at least a nesting-depth bound.
- For the assembler: the parsed unit that S2 returned for that source.

## Outputs

- From S2: one parsed unit. It holds the edition that the header recorded
  and, in source order, one `Value`-family parsed form per declaration. Each
  form holds the span of its `Declaration` CST node.
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
holds, that is, no diagnostic and no recovery. Otherwise it refuses with
cause `RecoveringCst` and returns no parsed unit (ADR-011 §2.3 E2).

For an admissible source, S2 visits the unit's `Declaration` nodes in source
order. The first significant token of each declaration selects its entry in
the `forms` core's dispatch table (ADR-012 §3). The `Value` family has these
entries:

| Leading token | Declaration production | `Value` parsed form |
|---|---|---|
| `function` | `FunctionDeclaration` | function form |
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

- **Function form.** It carries the declared name; the `using` alias as
  written; each parameter's name and type reference, in order; the result
  type reference; the `decreases` measure expression when one is written;
  and the body expression.
- **Alias form.** It carries the declared name and the aliased type
  reference.
- **Record form.** It carries the declared name and each field's name, type
  reference and optional marker `?`, in order.
- **Tuple form.** It carries the declared name and each element type
  reference, in order.

A **type reference** is carried as syntax. The form holds the constructor
keyword or qualified name as written, together with every declared bound,
scale, rounding mode, text profile and element type reference, each as
written. It holds no `ValueType` and no `NodeKey` (ADR-011 §2.2 E2 row:
"Declared bounds and extents carried as syntax"; identity "none"). This rule
applies to every type reference in a `Value` form, including the target of
`convert<T>(e)` and the named types of `fold<A>`, `reduce<A>`, `count<N>`
and `sum<N>`.

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
| `convert<T>(e)` | `Convert`, with `T` carried as a type reference |
| `(e)` | the mapping of `e` |

Grouping and precedence come from the CST's own nesting. The production
never re-derives them from token order.

When a declaration contains an expression construct that is absent from the
table, S2 refuses the whole unit. The cause is `UnrepresentedConstruct`, and
the refusal names the construct's CST production and span. Examples are a
text literal, `decimal(..)`, `float32(bits: ..)`, `null` outside a record
field value, `none`, `self`, `result`, `div`, `rem`, `mod`, indexing
`e[i]`, `size<T>(e)`, `reaches(..)`, `collect(..)`, `deref(e)`, `pre(e)` and
`allInstances<T>(e)`. S2 returns no form for any declaration of that unit.

### S2 depth limit

S2 bounds its recursion by the nesting-depth bound in its limit set, not by
the native stack (ADR-011 §2.3 Limits). A declaration nested deeper than
that bound gives a limit refusal. The refusal names limit kind nesting
depth, the configured bound, and the span of the construct where the bound
was exceeded. It is never a truncated form and never a `NoDispatchEntry` or
`UnrepresentedConstruct` refusal. A declaration nested exactly to the bound
builds.

### S2 layering

The `Value` family form builder is a module under `forms`. It depends on the
`forms` core, `qsl-cst`, `qsl-foundation` and `quire-exact`, and on no other
family's module (ADR-011 §6.1). No `Value` parsed form, and no field of an
`Expression` variant that the `Value` family owns, has type `ValueType` or
`NodeKey`.

### The assembler builds `PackageDeclarations` from parsed forms

The assembler is a module of the layer-3 `check` core. It reads the parsed
unit only. It reads no `qsl_cst` type, no source text and no token text
(FB-01). It fills `PackageDeclarations` as follows:

- `aliases`: one entry per alias form, in source order, holding the declared
  name and the resolved `ValueType`.
- `functions`: one `FunctionDeclaration` per function form, in source order,
  built with `FunctionDeclaration::new`. So each one is clause kind `Body`
  and callable by name. Parameter and result types are resolved `ValueType`s.
- `types`: one composite declaration per record form and tuple form. Each
  has a declaration key that `check` mints (FB-13, ADR-013 O-04).

Type resolution is the E3 name-binding phase (ADR-011 §1). A type reference
resolves in one of three ways:

- a built-in constructor (`Boolean`, `Integer`, `Int[..]`, `Rational[..]`,
  `Decimal[..]`, `Text[..]`, `Option<..>`, `Sequence`/`Set`/`Bag`/
  `OrderedSet<..>[..]`) resolves to its `ValueType` over the declared bounds
  it carries;
- a qualified name that names an alias form of the same unit resolves to
  that alias's resolved type;
- a qualified name that names a record or tuple form of the same unit
  resolves to `ValueType::Composite` of that declaration's key.

The assembler refuses in these cases:

- **Unresolved type name.** A qualified name names no alias, record or tuple
  form of the unit. The refusal names the name and the span of the form
  that holds it.
- **Ill-formed scalar bounds.** The value type rejects a built-in
  constructor's declared bounds, for example an `Int` interval whose lower
  bound is above its upper bound, a `Rational` denominator interval that
  reaches below one, or a `Text` minimum above its maximum. The refusal
  carries that value type's own cause and the span of the form that holds
  the reference.
- **Alias cycle.** An alias resolves through a chain of aliases back to
  itself. The refusal names the aliases on the cycle.

A refusal carries every error the assembler found in the unit, not only the
first (ADR-011 §2.3 E3). The assembler returns no `PackageDeclarations`
value alongside a refusal.

The assembler fills no other `PackageDeclarations` member from parsed forms.
For a unit with no `import` or `model` selection, `enums`,
`model_operations`, `dispatch_operations`, `dispatch_tables` and
`model_correspondence` are empty, and `ieee_profile` is `None`.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-091-CON-1 | The `Value` family form builder and the assembler are the only producers of a `PackageDeclarations` value from complete-V1 source. The assembler's only input is S2 output. | Design | Inspection |
| FR-091-CON-2 | The dispatch table, the `Value` production's expression match and the assembler's type-reference match have no `_` or catch-all arm (ADR-012 §5.1). | Design | Test (TC-398) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-091-AC-1 | S2 returns a parsed unit with exactly four forms, in the order alias, function, record and tuple, for an admissible S1 output whose header records edition `1-draft` and which holds one profile selection and then, in order, one `type`, one `function`, one `record` and one `tuple` declaration. Each form's span equals the span of its `Declaration` CST node, and the unit's edition reads `1-draft`. | Test (TC-392) |
| FR-091-AC-2 | Given `function inc using v(x: Int[0, 9], y: Digit): Int[0, 10] pure decreases(x) { x + 1 }`, the function form reads: name `inc`; `using` alias `v`; parameter `x`, whose type reference is constructor `Int` with bounds spelled `0` and `9`; parameter `y`, whose type reference is the qualified name `Digit`; a result type reference `Int` with bounds `0` and `10`; measure `Name("x")`; and body `Binary{Add, Name("x"), Integer(1)}`. No accessor on the form returns a `ValueType` or a `NodeKey`. | Test (TC-393) |
| FR-091-AC-3 | For each row of the expression-mapping table, a function body that holds that construct maps to the listed `Expression` variant, with operands in source order. `a or b and c` maps to `Or(a, And(b, c))`. `a - b - c` maps to `Subtract(Subtract(a, b), c)`. `a implies b implies c` maps to `Implies(a, Implies(b, c))`. `(a + b) * c` maps to `Multiply(Add(a, b), c)`. | Test (TC-394) |
| FR-091-AC-4 | Given a `ParsedSource` that carries a recovery, and separately one that carries a diagnostic but no recovery, S2 refuses each with cause `RecoveringCst` and returns no parsed unit. | Test (TC-395) |
| FR-091-AC-5 | Given an admissible unit that holds a valid `function` declaration followed by an `invariant` declaration, S2 refuses the whole unit with cause `NoDispatchEntry`. The refusal names the spelling `invariant` and the `invariant` declaration's span, and S2 returns no form for the `function` declaration. | Test (TC-395) |
| FR-091-AC-6 | Given an admissible unit whose only declaration is an `enum`, a `predicate`, a `dimension` or a `unit` declaration, S2 refuses with cause `NoDispatchEntry` and names that declaration's leading token and span. Open question: FR-091-OQ-1. | Test (TC-395) |
| FR-091-AC-7 | Given a function whose body holds exactly one construct absent from the mapping table (one case each for a text literal, `decimal(1, 2)`, `a mod b`, `xs[0]` and `none`), S2 refuses the whole unit with cause `UnrepresentedConstruct`. The refusal names the construct's CST production and span, and S2 returns no form. The same holds for `deref(e)`, `pre(e)` and `allInstances<T>(e)`. Open questions: FR-091-OQ-5, and FR-091-OQ-1 for the last three. | Test (TC-396) |
| FR-091-AC-8 | With an S2 nesting-depth bound `L`, a function body of `L` nested `not` operators around a name builds a form. The same body with `L + 1` nested `not` operators gives a limit refusal that names limit kind nesting depth, the bound `L` and the span of the `not` construct at depth `L + 1`, with no parsed unit. A body nested to the deepest level S1 admits, far beyond `L`, gives the same limit refusal and does not overflow the stack. | Test (TC-397) |
| FR-091-AC-9 | The `Value` family form builder module imports only the `forms` core, `qsl_cst`, `qsl_foundation` and `quire_exact`. It imports no other family module and nothing under `crate::check`, `crate::value` or `crate::model`. No field of a `Value` parsed form, and no field of an `Expression` variant in the mapping table, has type `ValueType` or `NodeKey`. The dispatch `match`, the expression `match` and the assembler's type-reference `match` have no `_` arm. | Test (TC-398) |
| FR-091-AC-10 | The assembler returns a `PackageDeclarations` value whose `aliases` is `[("Digit", Int[0..9])]` and whose `functions` are `inc` and then `two`, with `inc`'s parameter type `Int[0..9]`, for the S2 output of source that declares `type Digit = Int[0, 9];`, then `function inc using v(x: Digit): Int[0, 10] pure { x + 1 }`, then `function two using v(): Int[0, 10] pure { inc(1) }`. | Test (TC-399) |
| FR-091-AC-11 | Given a unit with `function f using v(x: Missing): Boolean pure { true }` and `function g using v(y: Absent): Boolean pure { true }`, the assembler returns one refusal that holds two unresolved-type-name errors. One names `Missing` with `f`'s form span, and the other names `Absent` with `g`'s form span. It returns no `PackageDeclarations`. | Test (TC-400) |
| FR-091-AC-12 | Given parameters typed `Int[9, 0]`, `Rational[0, 1; 0, 5]` and `Text[5, 1; nfc]`, one per function, the assembler returns one refusal with three errors. Each carries the owning value type's own rejection cause and the span of its function form. It returns no `PackageDeclarations`. | Test (TC-400) |
| FR-091-AC-13 | Given `type A = B;` and `type B = A;`, the assembler refuses with an alias-cycle error that names both `A` and `B`, and returns no `PackageDeclarations`. | Test (TC-400) |
| FR-091-AC-14 | Given `record Point { x: Int[0, 9]; y: Int[0, 9]; }`, `tuple Pair(Int[0, 9], Int[0, 9]);` and `function px using v(p: Point): Int[0, 9] pure { p.x }`, the assembler's `types` holds one record declaration `Point` and one tuple declaration `Pair`, each keyed by a key that `check` minted. `px`'s parameter type is `ValueType::Composite` of `Point`'s key, and `PackageDeclarations::check` admits the package. Open question: FR-091-OQ-3. | Test (TC-401) |
| FR-091-AC-15 | The assembler module is under the layer-3 `check` core and has no `use` edge or inline path to `qsl_cst`. Its tests build its input only through S2 or from parsed-form values, never from a CST. | Test (TC-402) |
| FR-091-AC-16 | For the AC-10 source, which has no `import` or `model` selection, the assembled `PackageDeclarations` has empty `enums`, `model_operations`, `dispatch_operations`, `dispatch_tables` and `model_correspondence`, and `ieee_profile` is `None`. Open questions: FR-091-OQ-1 for `enums`, FR-091-OQ-2 for `ieee_profile`. | Test (TC-403) |
| FR-091-AC-17 | Calling `two` with no arguments through `CheckedPackage::call`, on the package that `PackageDeclarations::check` admits and links from the AC-10 assembler output, returns a completed outcome with integer value `2`. | Test (TC-399) |

## Dependencies

- **Upstream:** [FR-067](FR-067-add-s2-forms-and-retire-seam-5.md) built the
  `forms` core: the dispatch table, `ParsedForm`'s span, edition and
  declared-extent accessors, and the `RecoveringCst` and `NoDispatchEntry`
  causes. This requirement adds the `Value` family's entries and production
  (M-3b), which FR-067-CON-2 leaves to the family.
  [FR-068](FR-068-split-expression-checking-into-check-stage.md) placed
  `PackageDeclarations` and its `check` in the layer-3 `check` core.
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
`Expression::Convert` holds a `ValueType` target, which AC-9 does not allow.

## Open Questions

- **FR-091-OQ-1: Which family owns the `enum`, `predicate`, `dimension` and
  `unit` declaration productions, and the nested `deref`, `pre` and
  `allInstances` constructs?** ADR-012 §1 gives `Value` "scalar and composite
  values, types", and §3 lists its parsing forms as "literals, operators,
  `let`, `if`, calls, records, collections, function declarations". §3 gives
  `SumCase` "variant type declarations". Neither list names `enum` (which
  `PackageDeclarations::enums` holds), `predicate` (grammatically a
  Boolean-valued function declaration), `dimension` or `unit`. ADR-012 §4.3
  moves the `Deref` check arm to `StateModel` and `Pre` to `ProtocolClause`,
  but it rules on check arms, not on which production parses the construct
  when it is nested in a `Value` declaration. FR-091-AC-6, AC-7 (last three
  cases) and AC-16 (`enums`) depend on this.
- **FR-091-OQ-2: What do the unit's profile, import and model selections and
  a function's `using` alias become at E2 and E3?** The E2 row carries only
  the edition in its Version column. E3 admits "library lock" beside the
  parsed forms, and no FR says where lock evidence comes from (recorded on
  QSL-6). `using <alias>` is not trivia, so E2 cannot drop it. AC-2 has
  the form carry it as written. No text says what E3 does with it: whether
  it must name a profile selection of the unit, and what it refuses if it
  does not. `ieee_profile` is built by `DefinitionLock::admit_ieee_profile`,
  which needs the lock. FR-091-AC-16 (`ieee_profile`) depends on this. If E3
  must admit the profile against the lock, the AC-10 fixture gains that lock
  entry.
- **FR-091-OQ-3: Over which package identity does `check` mint record and
  tuple declaration keys?** ADR-013 O-04's node-identity preimage includes
  the declaring package's `name@version` (QC-18). No FR says where the
  assembler gets a package's name and version for a unit compiled from
  source; this is the same lock-evidence gap as FR-091-OQ-2. FR-091-AC-14
  depends on this.
- **FR-091-OQ-4: Where does a floating type's rounding mode go?** The grammar
  writes `Float32[mode]` and `Float64[mode]`. `ValueType::Float` holds only
  an `IeeeWidth`, and ADR-013 R-07 forbids a conversion that drops a
  declared value. No criterion covers resolving a floating type reference
  until this is ruled.
- **FR-091-OQ-5: Does the `Value` production represent the rest of the
  complete-V1 `Value` syntax?** Text, decimal and float literals, `none`,
  `null`, `self`, `result`, `div`, `rem`, `mod`, indexing, `size<T>`,
  `reaches` and `collect` have grammar productions and no `Expression`
  variant. Adding a variant needs checker and evaluator support as well.
  FR-091-AC-7 states what happens today: the production refuses. Whether
  M-3b for `Value` must add variants for these constructs is not ruled.
- **FR-091-OQ-6: Does every nested expression carry its span at S2?** The E2
  row says "each form holds the span of its CST node". `Expression` variants
  carry no span, and E3 keys the source map by occurrence key (ADR-013 O-07,
  QSL-159). The criteria above require spans on declaration forms only.
- **FR-091-OQ-7: Which catalog codes do the S2 and assembler causes map to?**
  ADR-013 O-17 gives each cause type one exhaustive `catalog_code()` into
  the catalog revision that FR-322 selects, and forbids inventing a code.
  This requirement does not verify whether that revision defines codes for
  `RecoveringCst`, `NoDispatchEntry`, `UnrepresentedConstruct`, an
  unresolved type name or an alias cycle (compare FR-090-OQ-2). No criterion
  above asserts a catalog code.
