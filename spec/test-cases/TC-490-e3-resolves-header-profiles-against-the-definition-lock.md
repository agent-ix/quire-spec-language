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
header profile with its catalogued code and cause. Scope: FR-110-AC-1 to
FR-110-AC-6.

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

Tag the test `#[trace("FR-110-AC-1", …, "FR-110-AC-6", "TC-490")]` over the
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

## Status

Planned; no test backs this case (QSL-234).
