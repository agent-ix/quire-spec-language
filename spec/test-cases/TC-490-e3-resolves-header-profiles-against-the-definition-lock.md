---
id: TC-490
title: "E3 resolves header profile selections against the DefinitionLock catalog"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: verifies
---
# TC-490: E3 resolves header profile selections against the DefinitionLock catalog

## Description

Verify that spine `compile` admits a header profile only when it selects
the `DefinitionLock` catalog's `root` row exactly, and refuses every other
header profile with its catalogued code and cause, and that a header
naming a QSpec AD-003 layer restricts its declarations to that layer.
Scope: FR-110-AC-1 to FR-110-AC-8.

This catches a resolver that reads the header as informational, one that
compares only the identity, one that accepts any catalog row as a profile,
and one that stops at the first refusing profile.

## Test Procedure

Every unit starts with `language "ix:native" edition "1-draft";`, then its
header profiles, then `function f using v(): Boolean pure { true }`. `R`
is `DefinitionLock::pinned().entry(CatalogRole::Root)`; the test reads its
identity, revision value and digest from the catalog, never from literals.

1. Compile the unit whose one profile is `v` with `R`'s identity, revision
   value and `sha256:`-spelled digest. Read the emitted lock.
2. Compile it with the identity `test:unknown-profile`.
3. Compile it with the `ieee_profile` row's identity, revision value and
   digest, then with the `edition` row's.
4. Compile it with `R`'s identity and version `"1"`.
5. Compile it with `R`'s identity and revision value and digest
   `sha256:` followed by 64 `a`s.
6. Compile a unit with profiles `v` (exact `R`), `w` (`R`'s identity,
   version `"1"`) and `x` (`test:unknown-profile`). Then compile a unit
   with profiles `v` and `u`, both exact `R`.
7. For each QSpec AD-003 layer (`quire.state.core/v1`,
   `quire.state.queries/v1`, `quire.state.graph/v1`,
   `quire.value.complete/v1`, `quire.model.complete/v1`), compile a unit
   whose one profile `v` names the layer and whose one declaration uses
   only that layer's forms. Read each emitted lock.
8. Compile a unit whose profile `v` names `quire.state.core/v1` and whose
   declaration calls a named predicate, then the same unit with `v` naming
   `quire.state.queries/v1`.

Tag the test `#[trace("FR-110-AC-1", …, "FR-110-AC-8", "TC-490")]` over the
criteria each step backs.

## Expected Results

- Step 1 compiles. `profile_selections` is empty, and
  `definition_selections` holds one entry for `quire.value.complete/v1`,
  equal to `R.reference()`.
- Step 2 refuses `unknown_profile`/`unsupported-selection` at the identity
  literal's span, naming `v`, the selection and required role `root`. No
  package.
- Step 3 refuses `unknown_profile`/`wrong-selection-role` twice, once per
  compile, each retaining the selection and required role `root`.
- Step 4 refuses `stale_dependency`/`revision-mismatch`, retaining the
  selection and `R`'s identity, revision and digest.
- Step 5 refuses `stale_dependency`/`byte-digest-mismatch`, retaining the
  selection and `R`'s digest.
- Step 6's first unit refuses with exactly two refusals, `w`'s then `x`'s,
  and no package. Its second unit compiles.
- Step 7: each unit compiles; each lock's `definition_selections` holds the
  layer and every layer it requires and no layer that requires it, written
  out per layer as a literal in the test.
- Step 8: the state-core unit refuses `unsupported_construct`/
  `declaration-form` at the call's span naming `quire.state.core/v1`, with
  no package; the state-queries unit compiles.

## Status

Implemented: `a_header_profile_resolves_only_against_the_root_row`
(`qsl-replay/src/spine.rs`) backs steps 1-6. Steps 7 and 8 are planned.
