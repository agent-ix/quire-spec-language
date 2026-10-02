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

When spine `compile` (`qsl_replay::spine::compile`) compiles a complete-V1
unit, E3 SHALL resolve each of the unit's `profile <alias> = "<identity>";`
header declarations in the catalog of the family that owns the selected
definition, and refuse, with a catalogued code and cause, every header
profile that does not resolve (ADR-011 §2.4, amended 2026-09-26).
This requirement specifies the `Value` family's catalog,
`DefinitionLock` (`qsl-semantics/src/value/definition.rs`), which reads
QSpec's `complete-value-lock.json` by reference, including its
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

## Inputs

- The unit's profile selections as S2 carries them
  (`qsl_foundation::selection::ProfileSelection`: alias, `DefinitionRef`
  identity, declaration span, identity-literal span).
- `DefinitionLock::pinned()`, the closed catalog of QSpec
  `complete-value-lock.json` rows, including its `header_selectable_layers`
  rows (each layer's identity, the layers it requires and its admitted-form
  definition file), read from the `quire-specification` crate's
  compiled-in bytes, which the resolution reads itself.

## Outputs

- `check::resolve_profiles(profiles: &[ProfileSelection])
  -> Result<(), Vec<ProfileRefusal>>`, in layer-3 `check`, reading
  `DefinitionLock::pinned()`. Success returns
  nothing: the `Value` family's lock rows are fixed by the catalog, so no
  later stage reads a resolved profile.
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

### Model selections

E3 SHALL resolve each `model` declaration only against the domain package
I1 admitted for it (FR-056; ADR-011 §2.4). A declaration naming no supplied
package, a stale version or digest, or a `sha256:` compiled-model digest
refuses at I1 with FR-056's cause (FR-056-AC-2, FR-056-AC-7, FR-056-CON-4).
The emitted `model_selections` are `CheckedGraph::model_selections`, one
`{identity, version, sha256-jcs digest}` entry per admitted package.

### Lock rows

The emitter SHALL write `profile_selections` empty for a package whose
declarations are all `Value`-family: FR-322 fills it with clause profile
rows (`temporal_profile`, `protocol_profile`).

The emitter SHALL write `definition_selections` as the union, over the
unit's header profiles, of the selected layer and every layer its
`requires` closure names, and no layer that requires a selected layer. When
that union holds `quire.value.complete/v1`, the entries also hold the
`qualification_catalog` rows the lock's package-selection rules select for
the unit; otherwise they hold the `edition` row and no other
`qualification_catalog` row. The two sides of a QSpec AD-003 `requires`
edge therefore compile to distinct packages.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-110-CON-1 | The resolution's one input is the unit's profile selections; it reads the catalog, including the `header_selectable_layers` rows that name each layer's admitted-form set and `requires` edges, from `DefinitionLock::pinned()` (ADR-011 §2.4: the lock evidence derives from the source and the QSL build). QSL defines no admitted-form set of its own. | Design | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-110-AC-1 | A unit whose one header profile names `quire.value.complete/v1` compiles through spine `compile`. Its emitted lock has empty `profile_selections`, and its `definition_selections` hold `quire.value.complete/v1`, `quire.state.core/v1` and the `qualification_catalog` rows the package-selection rules select for the unit, and no other layer. | Test (TC-490) |
| FR-110-AC-2 | The same unit with the profile identity `test:unknown-profile` refuses `unknown_profile`/`unsupported-selection` at the span of the identity literal, naming alias `v`, the selection and required role `root`, and produces no package. | Test (TC-490) |
| FR-110-AC-3 | The same unit with the header profile set to the `ieee_profile` row's identity refuses `unknown_profile`/`wrong-selection-role`, retaining the selection and required role `root`. The `edition` row's identity (`ix:native`) refuses the same way. | Test (TC-490) |
| FR-110-AC-5 | The same unit with the header `profile v = "quire.value.complete/v1" version "1" digest "sha256:<64 a>";` refuses as a syntax error at the `version` token, before E3, and produces no package. | Test (TC-490) |
| FR-110-AC-6 | A unit with three header profiles, the `root` selection under alias `v`, the `ieee_profile` row's identity under `w` and an `unsupported-selection` selection under `x`, refuses with exactly two refusals, `w`'s then `x`'s, and no package. A unit with the `root` selection under two aliases compiles. | Test (TC-490) |
| FR-110-AC-7 | For each of the five layers, a unit whose one header profile names the layer and whose declarations use only that layer's forms compiles, and its emitted lock's `definition_selections` hold the layer and every layer its `requires` closure names and no layer that requires it. | Test (TC-490) |
| FR-110-AC-8 | A unit whose header profile names `quire.state.core/v1` and whose declaration calls a named predicate refuses `unsupported_construct`/`expression-form` at the call's span naming `quire.state.core/v1`; a unit naming `quire.state.core/v1` that declares a named predicate refuses `unsupported_construct`/`declaration-form` at the declaration's span; both units naming `quire.state.queries/v1` compile. | Test (TC-490) |

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

## Status

Implemented, apart from the remaining work below, and backed by the
TC-490 test (`a_header_profile_resolves_only_against_the_root_row`,
`qsl-replay/src/spine.rs`).

Remaining work: the implementation still parses a header's `version` and
`digest` and compares them with the `root` row. AC-5 states the behavior: a
header profile is `profile <alias> = "<identity>";` and resolves by identity
alone. Layer
selection (AC-7, AC-8) and AC-1's layer-closure lock rows are not yet
implemented.
