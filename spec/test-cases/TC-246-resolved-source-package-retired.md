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
successors exist and FR-087's held rows have a ruled home, with no
compatibility alias, feature-flagged fallback or wrapper keeping an old
name reachable, and that each scenario their tests cover is backed against
its row's successor. Scope: FR-087-AC-7, FR-087-CON-4.

The two successors (FR-087 Behavior, "`ResolvedSourcePackage` retires when
both successors exist"; ruling on QSL-229):

- **Dependency half.** Spine `compile`'s S4 source resolution against I2
  import views (FR-099, ADR-015 D-1). Implemented.
- **Header-selection half.** E3's resolution of header profiles against
  the `DefinitionLock` catalog, and of `model` declarations through I1
  (FR-110, FR-056). QSL-234.

## Test Procedure

1. Search every crate of the workspace (`src/`, `tests/`, `examples/`,
   `xtask/` and each layer crate) for `ResolvedSourcePackage`,
   `resolve_source_package` and `resolve_parsed_source`, and for each item
   the held rows' ruling removes.
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
   parse). For a held row, name the test against its ruled home, carrying
   the scenario's QSpec FR-131/FR-339 tags.

## Expected Results

- Steps 1-2: no occurrence of a removed item. `xtask` arch-lint tests that
  plant one of these names as synthetic source text are not occurrences.
- Step 3: the workspace builds with no reachable definition of a removed
  item under any feature combination.
- Step 4: every scenario maps to a backing test. A scenario with none
  fails this step.

## Status

Planned; no test backs this case. It runs once FR-110 is implemented and
the held rows are ruled (QSL-234); QSL-269 owns the retirement. Until then `ResolvedSourcePackage`
stays in place. See FR-087 Status, AC-7 and CON-4.
