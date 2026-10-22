---
id: FR-110
title: "E3 resolves a unit's header profile selections against the DefinitionLock catalog"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-014
    type: implements
  - target: ix://agent-ix/quire-spec-language/US-005
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-453
    type: depends_on
---
# FR-110: E3 resolves a unit's header profile selections against the DefinitionLock catalog

## Description

When spine `compile` (the FR-278 composition) compiles a complete-V1
unit, E3 SHALL resolve each of the unit's `profile <alias> = "<identity>";`
header declarations in the catalog of the family that owns the selected
definition, and refuse, with a catalogued code and cause, every header
profile that does not resolve (ADR-011 §2.4, amended 2026-09-26).
This requirement specifies the `Value` family's catalog,
`DefinitionLock` (`qsl-semantics/src/value/definition.rs`), which names
each definition the QSL build implements by `{authority, identity}`, the
identities QSpec's `complete-value-lock.json` names, including its
`header_selectable_layers` rows. A clause profile (FR-322 roles
`temporal_profile` and `protocol_profile`) resolves in its clause family's
catalog (ADR-012 §2, the `TemporalTrace` and `ProtocolClause` families,
#218), which also writes its `profile_selections` row.

This is the header-selection half of `complete::resolve_source_package`'s
successor (FR-087, "`ResolvedSourcePackage` retires when both successors
exist"; ruling on QSL-229). Its model half is I1 (FR-056), which spine
`compile` already runs, and which is the one model resolution path. The
lock's edition and definition selections come from the same catalog
through the emitter (ADR-011 §2.4, amended 2026-09-24).

FR-361 states what a unit that selects `quire.model.complete/v1` gains, and
the complete-model lock rows the catalog then holds.

## Inputs

- The unit's profile selections as S2 carries them
  (`qsl_foundation::selection::ProfileSelection`: alias, `DefinitionRef`
  identity, declaration span, identity-literal span).
- `DefinitionLock::pinned()`, the closed catalog of the definitions QSpec
  `complete-value-lock.json` names, each by `{authority, identity}`,
  including its `header_selectable_layers` rows (each layer's identity, the
  layers it requires and its admitted-form definition file). It holds no
  revision, no digest and no QSpec document; `make conformance` compares it
  with QSpec's lock, read at run time.

## Outputs

- `check::resolve_profiles(profiles: &[ProfileSelection])
  -> Result<(), Vec<ProfileRefusal>>`, in layer-3 `check`, reading
  `DefinitionLock::pinned()`. Success returns
  nothing: the `Value` family's lock rows are fixed by the catalog. The
  layers the headers resolve to reach the checked graph as `header_layers`,
  which the emitter reads for `definition_selections` ("Lock rows").
- `check::ProfileRefusal`: the profile's alias, the span of its identity
  literal, the `DefinitionRef` it selected, the required role (`root`), the
  `root` row it was compared with (`CatalogEntry`) and a `ProfileCause`:
  `UnsupportedSelection` or `WrongSelectionRole`. Its `code()` and `cause()`
  give the catalog pair in the table below.
- `qsl_replay::spine::CompileRefusal::Profile`: the refusals and the region
  of the first, reported at spine stage `assembly`.

## Behavior

### Profile resolution

For a header profile the `Value` family resolves, E3 SHALL compare its
identity, in source order, with the `DefinitionLock` rows. A header selects
a header-selectable layer (see "Layer selection"); the catalog's
`qualification_catalog` rows are selected by its own package-selection
rules (always, conditional and exactly-one roles), not by a header. E3
compares the header's identity, the only member a header profile carries
(QSpec shared grammar `profile = 'profile', ident, '=', string, ';'`). The
emitted lock records
the catalog rows, so the package's identity binds the definitions the
build compiled against.

| Header selection | Code | Cause |
| --- | --- | --- |
| identity equals a `header_selectable_layers` row's | resolves to that layer | |
| identity equals no catalog row's, and no clause family's catalog holds it | `unknown_profile` | `unsupported-selection` |
| identity equals a `qualification_catalog` row's and no layer's | `unknown_profile` | `wrong-selection-role` |

- If any header profile refuses, E3 SHALL return every refusing profile, in
  source order, and spine `compile` SHALL produce no checked package and no
  bytes (ADR-011 §2.3, E3 row).
- An `unknown_profile` refusal SHALL retain the supplied selection and the
  required role, `root` (native-diagnostics: "retain the supplied selection
  and required role").
- The spine registers no clause family yet, so every header profile a spine
  unit declares reaches this table.
- E3 does not admit or refuse `=` and `!=`: whether an equality is admitted
  depends on its operand types, so S3 decides it after typing (Equality by
  operand family).
- A unit whose header declares no profile selects no layer. Its lock holds
  the package-selection `always` rows, as for a unit that selects
  `quire.value.complete/v1`, and no layer row. This is the explicit default.
- Two header profiles that both select the same layer both resolve;
  a repeated alias is the assembler's duplicate-alias refusal (FR-091-AC-22).
- Every code and cause is one `quire.native.diagnostics/v1` lists. The
  `unsupported-selection` row gives the pair the editor's profile check
  gives for the same standing (`complete::editor::profile_refusal`).

### Placement

Spine `compile` SHALL run the resolution in every unit it compiles, the
unit itself and each library the S4 source resolution compiles (ADR-015
D-1), after I1 and the S4 source resolution and before the FR-091
assembler. Replay recompiles through the same spine (ADR-011 §2.1 E9), so
it applies the same resolution.

### Layer selection

Every layer of QSpec AD-003's state and value/model hierarchy is
header-selectable (QSpec FR-453; the `header_selectable_layers` rows of
QSpec `complete-value-lock.json`, which the QSpec definitions README
section "Header-selectable layers" cites): `quire.state.core/v1`,
`quire.state.queries/v1`, `quire.state.graph/v1`,
`quire.value.complete/v1` and `quire.model.complete/v1`.

- When a header profile's identity names a `header_selectable_layers` row,
  E3 SHALL resolve it to that layer.
- When a declaration's `using` alias names a header profile resolved to a
  layer, S3 SHALL admit the declaration under exactly the admitted-form set
  of that layer's `admitted_forms` definition, together with the sets of
  the layers its `requires` closure names.
- When such a declaration uses a declaration form outside that set, S3
  SHALL refuse it with `unsupported_construct`/`declaration-form` at the
  declaration's span, naming the selected layer. When it uses an expression
  form outside that set, S3 SHALL refuse it with
  `unsupported_construct`/`expression-form` at the expression's span,
  naming the selected layer. S3 never admits a form under a wider layer.
  FR-361 item 12 governs a form of the model families that a unit not
  selecting the model profile writes: it receives FR-361's refusal alone.

### Editor profile check

The editor's profile check (`complete::editor::profile_refusal`, which
`complete::parse_with_catalog`, `complete::format_document`,
`complete::analyze_document` and `complete::apply_edit_with_catalog` run)
compares each header profile identity with the `ProfileCatalog` its caller
supplies. The root crate's `complete::header_profile_catalog()` SHALL return
the production catalog: a `ProfileCatalog` built from the identities of
`DefinitionLock::pinned().layers()`, the one layer table E3 reads, with no
second list of layer identities. It contains each of those identities and no
other.

- With that catalog, the editor's profile check SHALL admit a header
  profile exactly when E3 resolves it to a layer.
- With that catalog, the editor's profile check SHALL refuse every other
  header profile with `unknown_profile`/`unsupported-selection` at the span
  of its identity literal. The editor has no catalog roles, so a
  `qualification_catalog` row's identity, which E3 refuses as
  `wrong-selection-role`, also receives `unsupported-selection` there.

### Equality by operand family

QSpec's layer definitions (`state-core.md`, `state-queries.md`,
`state-graph.md` under QSpec `proposals/quire-v1/definitions/`) admit
equality by what it compares, not by the operator. For a declaration whose
`using` alias names a header profile resolved to a layer (Layer selection),
S3 SHALL classify each `=` and `!=` by its typed operand families (FR-149)
and admit it under the declaration's selected layers only as this table
states. A declaration that selects no layer is not restricted by this
section.

| Operand family | Layers that admit `=` and `!=`, first the one a refusal names |
| --- | --- |
| Boolean, integer, rational, decimal, text, enum, quantity, UUID, Timestamp (scalar) | every layer |
| `Reference<T>` and object identity | `quire.state.graph/v1`, `quire.model.complete/v1` |
| record, tuple, union, `Option<T>`, `Sequence<T>`, `Set<T>`, `Bag<T>`, `OrderedSet<T>` (whole value) | `quire.value.complete/v1`, `quire.model.complete/v1` |

- An operand type's families SHALL be the family of the type itself and of
  every type it contains at any depth: `Sequence<Reference<T>>` and a record
  with a `Reference<T>` field are both whole-value and reference. A
  reference's target type `T` is not a contained type, so `a = b` over
  `Reference<Order>` has the reference family alone, whatever `Order`'s
  fields are.
- A declaration's selected layers admit a family when the family's row names
  one of them or a layer in one of their `requires` closures, and admit an
  equality only when they admit every family of its operand type.
- When a declaration's selected layers do not admit the operand family of
  an `=` or `!=`, S3 SHALL refuse it with
  `unsupported_construct`/`expression-form` at the operator's span, naming
  the selected layer and the first layer named by the first row, in table
  order, that the selected layers do not admit.
- S3 SHALL run this check after the equality is typed. An equality whose
  typing refuses (`ill_typed`/`type-mismatch` or
  `ill_typed`/`operator-ineligible`, FR-149) SHALL report that refusal and no
  layer refusal.
- An equality nested inside another form SHALL be classified by its own
  operands: `present(o)` admits under `quire.state.core/v1` while `o = p`
  over `Option<T>` operands does not.

### Model selections

E3 SHALL resolve each `model` declaration only against the domain package
I1 admitted for it (FR-056; ADR-011 §2.4). A declaration naming no supplied
package, a stale digest, or a `sha256:` compiled-model digest
refuses at I1 with FR-056's cause (FR-056-AC-2, FR-056-AC-7, FR-056-CON-4).
The emitted `model_selections` are `CheckedGraph::model_selections`, one
`{identity, sha256-jcs digest}` entry per admitted package.

### Lock rows

The emitter SHALL write `profile_selections` empty for a package whose
declarations are all `Value`-family: FR-322 fills it with clause profile
rows (`temporal_profile`, `protocol_profile`).

The emitter SHALL write `definition_selections` as the union, over the
unit's header profiles, of the selected layer and every layer its
`requires` closure names, and no layer that requires a selected layer,
together with the `qualification_catalog` rows QSpec FR-001 selects for
those layers: the lock's `package_selection` rules when the union holds its
`package_selection.layer`, and its `package_selection.without_layer` rows
otherwise. The two sides of a QSpec AD-003 `requires`
edge therefore compile to distinct packages.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-110-CON-1 | The resolution's one input is the unit's profile selections; it reads the catalog, including the `header_selectable_layers` rows that name each layer's admitted-form set and `requires` edges, from `DefinitionLock::pinned()` (ADR-011 §2.4: the lock evidence derives from the source and the QSL build). QSL's Rust layer table is the machine form of QSpec's prose admitted-form sets; QSL adds no form of its own. | Design | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-110-AC-1 | A unit whose one header profile names `quire.value.complete/v1` compiles through spine `compile`. Its emitted lock has empty `profile_selections`, and its `definition_selections` hold `quire.value.complete/v1`, `quire.state.core/v1` and the `qualification_catalog` rows the package-selection rules select for the unit, and no other layer. | Test (TC-490) |
| FR-110-AC-2 | The same unit with the profile identity `test:unknown-profile` refuses `unknown_profile`/`unsupported-selection` at the span of the identity literal, naming alias `v`, the selection and required role `root`, and produces no package. | Test (TC-490) |
| FR-110-AC-3 | The same unit with the header profile set to the `ieee_profile` row's identity refuses `unknown_profile`/`wrong-selection-role`, retaining the selection and required role `root`. The `edition` row's identity (`ix:native`) refuses the same way. | Test (TC-490) |
| FR-110-AC-5 | A unit whose header `profile` declaration holds any token after its identity string, before the `;` (QSpec shared grammar `profile = 'profile', ident, '=', string, ';'`), refuses as a syntax error at that token, before E3, and produces no package. | Test (TC-490) |
| FR-110-AC-6 | A unit with three header profiles, the `root` selection under alias `v`, the `ieee_profile` row's identity under `w` and an `unsupported-selection` selection under `x`, refuses with exactly two refusals, `w`'s then `x`'s, and no package. A unit with the `root` selection under two aliases compiles. | Test (TC-490) |
| FR-110-AC-7 | For each of the five layers, a unit whose one header profile names the layer and whose declarations use only that layer's forms compiles, and its emitted lock's `definition_selections` hold the layer and every layer its `requires` closure names and no layer that requires it. | Test (TC-490) |
| FR-110-AC-8 | A unit whose header profile names `quire.state.core/v1` and whose declaration calls a named predicate refuses `unsupported_construct`/`expression-form` at the call's span naming `quire.state.core/v1`; a unit naming `quire.state.core/v1` that declares a named predicate refuses `unsupported_construct`/`declaration-form` at the declaration's span; both units naming `quire.state.queries/v1` compile. | Test (TC-490) |
| FR-110-AC-9 | Under `make conformance`, `DefinitionLock::pinned()`'s catalog rows (role, authority and identity, in order), its package-selection rules, its trigger vocabulary and its selection refusal codes equal QSpec's `complete-value-lock.json`, and QSpec's selection vectors give the outcomes the lock's rules compute. | Test (TC-490) |
| FR-110-AC-10 | Under `quire.state.core/v1`, `x = y` over `Int[0, 9]`, over `Boolean` and over an enum compiles. Under `quire.state.core/v1` and under `quire.state.queries/v1`, each of a whole-record equality `r = q` (a record whose fields are `Int[0, 9]` and `Boolean`), a sequence equality `s = t` (over `Sequence<Int[0, 9]>`), an option equality `o = p` (over `Option<Int[0, 9]>`) and a reference equality `a = b` (over `Reference<Order>`) refuses `unsupported_construct`/`expression-form` at the operator's span, naming the selected layer and, in order, `quire.value.complete/v1`, `quire.value.complete/v1`, `quire.value.complete/v1` and `quire.state.graph/v1`. Under `quire.state.graph/v1`, `a = b` compiles and `r != q` refuses naming `quire.value.complete/v1`. Under `quire.value.complete/v1`, `r = q`, `s = t` and `o = p` compile and `a = b` refuses naming `quire.state.graph/v1`; under `quire.model.complete/v1`, all four compile. Under `quire.state.core/v1`, `r = 1` with `r` a record refuses `ill_typed`/`type-mismatch` and no layer refusal. | Test (TC-490) |
| FR-110-AC-11 | In a unit with no header profile, `r = q` over records and `x = y` over `Int[0, 9]` compile. Under `quire.value.complete/v1`, `s = t` over `Sequence<Reference<Order>>` and `r = q` over a record with a `Reference<Order>` field each refuse `unsupported_construct`/`expression-form` at the operator naming `quire.state.graph/v1`; under `quire.state.graph/v1` the same two refuse naming `quire.value.complete/v1`; under `quire.model.complete/v1` both compile. | Test (TC-490) |
| FR-110-AC-12 | `complete::header_profile_catalog()` contains each identity of `DefinitionLock::pinned().layers()`, and contains neither `test:unknown-profile` nor the identity of any `qualification_catalog` row. For each of the five layers, `complete::parse_with_catalog` over a unit whose one header profile names the layer, with that catalog, adds no `unknown_profile` diagnostic. The same call over the unit with the identity `test:unknown-profile` reports `unknown_profile`/`unsupported-selection` at the identity literal's span, the pair FR-110-AC-2 gives; over the unit whose header profile names the `ieee_profile` row's identity, which E3 refuses `wrong-selection-role` (FR-110-AC-3), it reports `unknown_profile`/`unsupported-selection` at the identity literal's span. | Test (TC-490) |

## Dependencies

- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §2.1 (E3 row), §2.3 (E3 refusals), §2.4 (lock evidence, amended
  2026-09-24 and 2026-09-26), §5 (spine `compile`).
- [FR-087](FR-087-typestate-and-cross-package-node-key.md), whose AC-7 and
  CON-4 retire `ResolvedSourcePackage` once this requirement and the
  dependency half exist.
- [FR-056](FR-056-admit-domain-package-model-declarations.md), I1: the
  model half.
- [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md),
  whose assembler resolves each `using` alias to a header profile
  (FR-091-AC-22) after this resolution.
- QSpec `proposals/quire-v1/definitions/complete-value-lock.json` (the
  catalog rows and `header_selectable_layers` rows), `native-diagnostics.md`
  (the codes and causes) and FR-322 (the lock members).
- QSpec FR-453 and QSpec FR-001 (header selection of each layer and its
  restricted admission).
