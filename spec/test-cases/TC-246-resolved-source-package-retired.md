---
id: TC-246
title: "ResolvedSourcePackage is retired, with no dangling caller"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: verifies
---
# TC-246: ResolvedSourcePackage is retired, with no dangling caller

## Description

Verify that `ResolvedSourcePackage`, `complete::resolve_source_package` and
`command::resolve_parsed_source` are removed in one change, made once both
successors exist, with no compatibility alias, feature-flagged fallback
or wrapper keeping an old name reachable; that the closure and bundle now
live in `library::bundle` (FR-111); and that each scenario their tests
cover is backed against its row's successor. Scope: FR-087-AC-7, FR-087-CON-4.

The two successors (FR-087 Behavior, "`ResolvedSourcePackage` retires when
both successors exist"; ruling on QSL-229):

- **Dependency half.** Spine `compile`'s S4 source resolution against I2
  import views (FR-099, ADR-015 D-1). Implemented.
- **Header-selection half.** E3's resolution of header profiles against
  the `DefinitionLock` catalog, and of `model` declarations through I1
  (FR-110, FR-056). Implemented.

## Test Procedure

1. Search every crate of the workspace (`src/`, `tests/`, `examples/`,
   `xtask/` and each layer crate) for `ResolvedSourcePackage`,
   `resolve_source_package`, `resolve_parsed_source`,
   `struct SourceAuthority`, `complete::SourceAuthority`, `ModelCatalog`,
   `ModelArtifact` and `qsl_semantics::complete`. The composed linker's
   unrelated `ConflictKind::SourceAuthority` variant is not an occurrence.
2. Search for any re-export, type alias or `pub use` naming a removed item
   under any spelling.
3. Build the workspace. A build that succeeds with a removed item reachable
   under any conditional compilation flag is a duplication finding, not a
   pass (the TC-170/TC-246 shared standard).
4. For each test of the former `tests/it/complete_package.rs` and
   `complete::package_tests`, find its row in FR-087 Behavior's disposition
   table. For a row with a successor, name the test that backs the scenario
   against it (TC-446 for imports, TC-490 for profiles, TC-412 for aliases,
   TC-145/TC-147 for models, the spine's S1 refusal for an inadmissible
   parse, TC-491 for the closure, bundle, limits and identity, carrying the
   scenario's `QSpec-FR-131`/`QSpec-FR-339` tags).

| Former scenario | Backing test |
| --- | --- |
| unknown, stale-revision, stale-digest and conflicting header profiles | `a_header_profile_resolves_only_against_the_root_row` (TC-490, `qsl-replay/src/spine.rs`) |
| stale profile at parse time | `tests/it/complete_editor.rs` |
| duplicate selection alias | `using_aliases_resolve_to_the_units_profile_selections` and its duplicate-alias case (FR-091-AC-22, `qsl-semantics/src/check/assemble/tests.rs`) |
| inadmissible parse | the spine's S1 refusal (`tests/it/spine_run.rs`, `tests/it/compile_command.rs`) |
| model selection and a `sha256:` model digest | TC-442 step 4 (`tests/it/compile_command.rs`) |
| closure, bundle, limits, identity, causes | TC-491 (`library::bundle_tests`, carrying FR-131/FR-339 tags) |
| source authority and span on a refusal | deleted with `SourceAuthority` (FR-087 disposition table); a bundle refusal names its root's index |

## Expected Results

- Steps 1-2: no occurrence of a removed item. `xtask` arch-lint tests that
  plant one of these names as synthetic source text are not occurrences.
- Step 3: the workspace builds with no reachable definition of a removed
  item under any feature combination.
- Step 4: every scenario maps to a backing test. A scenario with none
  fails this step.
