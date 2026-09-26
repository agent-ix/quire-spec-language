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
---
# FR-110: E3 resolves a unit's header profile selections against the DefinitionLock catalog

## Description

When spine `compile` (`qsl_replay::spine::compile`) compiles a complete-V1
unit, E3 SHALL resolve each of the unit's `profile … version … digest …`
header declarations in the catalog of the family that owns the selected
definition, and refuse, with a catalogued code and cause, every header
profile that does not resolve (ADR-011 §2.4, amended 2026-09-26).
This requirement specifies the `Value` family's catalog, QSL's
`DefinitionLock` (`qsl-semantics/src/value/definition.rs`), whose one
header-selectable row is `root`. A clause profile (FR-322 roles
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
  identity/version/digest, declaration span, identity-literal span).
- `DefinitionLock::pinned()`, the closed catalog of QSpec
  `complete-value-lock.json` rows, which the resolution reads itself.

## Outputs

- `check::resolve_profiles(profiles: &[ProfileSelection])
  -> Result<(), Vec<ProfileRefusal>>`, in layer-3 `check`, reading
  `DefinitionLock::pinned()`. Success returns
  nothing: the `Value` family's lock rows are fixed by the catalog, so no
  later stage reads a resolved profile.
- `check::ProfileRefusal`: the profile's alias, the span of its identity
  literal, the `DefinitionRef` it selected, the required role (`root`), the
  `root` row it was compared with (`CatalogEntry`) and a `ProfileCause`:
  `UnsupportedSelection`, `WrongSelectionRole`, `RevisionMismatch` or
  `ByteDigestMismatch`. Its `code()` and `cause()`
  give the catalog pair in the table below.
- `qsl_replay::spine::CompileRefusal::Profile`: the refusals and the region
  of the first, reported at spine stage `assembly`.

## Behavior

### Profile resolution

For a header profile the `Value` family resolves, E3 SHALL compare its
selection, in source order, with the `DefinitionLock` rows. The row a header profile selects is
`root` (`quire.value.complete/v1`): the value system every spine
declaration's `using` alias names (FR-091). The catalog's other rows are
selected by its own package-selection rules (always, conditional and
exactly-one roles), not by a header. The header's `version` is compared
with the row's revision value, and its `sha256:` digest with the row's
`quire.definition.bytes/v1` digest.

| Header selection | Code | Cause |
| --- | --- | --- |
| identity, version and digest equal the `root` row's | resolves | |
| identity equals no catalog row's, and no clause family's catalog holds it | `unknown_profile` | `unsupported-selection` |
| identity equals a catalog row's other than `root` | `unknown_profile` | `wrong-selection-role` |
| identity equals `root`'s, version differs | `stale_dependency` | `revision-mismatch` |
| identity and version equal `root`'s, digest differs | `stale_dependency` | `byte-digest-mismatch` |

- If any header profile refuses, E3 SHALL return every refusing profile, in
  source order, and spine `compile` SHALL produce no checked package and no
  bytes (ADR-011 §2.3, E3 row).
- An `unknown_profile` refusal SHALL retain the supplied selection and the
  required role, `root` (native-diagnostics: "retain the supplied selection
  and required role"). A `stale_dependency` refusal SHALL retain the
  selected triple and the `root` row's identity, revision and digest.
- The spine registers no clause family yet, so every header profile a spine
  unit declares reaches this table.
- Two header profiles that both select the `root` row exactly both resolve;
  a repeated alias is the assembler's duplicate-alias refusal (FR-091-AC-22).
- Every code and cause is one `quire.native.diagnostics/v1` revision
  `1-draft.7` lists. The `unsupported-selection`, `revision-mismatch` and
  `byte-digest-mismatch` rows give the pairs the editor's profile check
  gives for the same standing (`complete::editor::profile_refusal`).

### Placement

Spine `compile` SHALL run the resolution in every unit it compiles, the
unit itself and each library the S4 source resolution compiles (ADR-015
D-1), after I1 and the S4 source resolution and before the FR-091
assembler. Replay recompiles through the same spine (ADR-011 §2.1 E9), so
it applies the same resolution.

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
rows (`temporal_profile`, `protocol_profile`), and the `root` row is
already an always-selected `definition_selections` entry. The header
resolution adds nothing to the identity preimage, so it leaves `package_id`
unchanged.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-110-CON-1 | The resolution's one input is the unit's profile selections; it reads the catalog from `DefinitionLock::pinned()` (ADR-011 §2.4: the lock evidence derives from the source and the QSL build). | Design | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-110-AC-1 | A unit whose one header profile is `profile v = "quire.value.complete/v1" version "<root revision value>" digest "sha256:<root digest>"`, with the `root` row's catalog values, compiles through spine `compile`. Its emitted lock has empty `profile_selections` and exactly one `definition_selections` entry for `quire.value.complete/v1`, equal to the `root` row. | Test (TC-490) |
| FR-110-AC-2 | The same unit with the profile identity `test:unknown-profile` refuses `unknown_profile`/`unsupported-selection` at the span of the identity literal, naming alias `v`, the selection and required role `root`, and produces no package. | Test (TC-490) |
| FR-110-AC-3 | The same unit with the header profile set to the `ieee_profile` row's identity, revision value and digest refuses `unknown_profile`/`wrong-selection-role`, retaining the selection and required role `root`. The `edition` row's identity (`ix:native`) refuses the same way. | Test (TC-490) |
| FR-110-AC-4 | The same unit with the `root` identity and version `"1"` refuses `stale_dependency`/`revision-mismatch`, retaining the selected triple and the `root` row's identity, revision and digest. | Test (TC-490) |
| FR-110-AC-5 | The same unit with the `root` identity and revision value and a digest of 64 `a`s refuses `stale_dependency`/`byte-digest-mismatch`, retaining the selected triple and the `root` row's digest. | Test (TC-490) |
| FR-110-AC-6 | A unit with three header profiles, the exact `root` selection under alias `v`, a `revision-mismatch` selection under `w` and an `unsupported-selection` selection under `x`, refuses with exactly two refusals, `w`'s then `x`'s, and no package. A unit with the exact `root` selection under two aliases compiles. | Test (TC-490) |

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
  catalog rows), `native-diagnostics.md` (the codes and causes) and
  FR-322 (the lock members).
- Linear QSL-234 (this requirement), QSL-269 (the retirement it unblocks).

## Status

Specified under QSL-234; not yet implemented -- TC-490 planned. Owner of
the implementation: the QSL-234 implementation ticket.

Spine sources declare the placeholder header
`profile v = "quire.value.complete/v1" version "1" digest "sha256:aaaa…"`,
which this requirement refuses as `revision-mismatch`. Updates:

- **The implementing change** updates every source a spine compile reads
  to the `root` row's revision value and digest, and keeps `make ci` green:
  `tests/fixtures/spine-compile.native`, `spine-model.native` and
  `spine-run.native`; `tests/it/compile_command.rs` and
  `tests/it/spine_run.rs`; and `qsl-replay`'s `spine.rs` tests,
  `spine/call/tests.rs`, `spine/dependency_tests.rs` and
  `execute/tests.rs`. Sources that stop before E3 (the S1, S2, formatter,
  assembler and emitter unit tests) keep the placeholder. It also rewrites
  the `CatalogEntry::digest` doc (`value/definition.rs:311-313`), which
  says no reader verifies the digest; FR-110 reads the `root` digest.
- **Dependency on the A05 lane (QSL-273, QSL-271).** FR-108's corpus unit
  (`FR-108:51`) and TC-452 step 1's source (`TC-452:40`, and its byte
  count) spell `version "1"`. Those rows need the `root` revision value
  before FR-110 lands; this PR does not edit them.
