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

Verify that spine `compile` admits a header profile only when its identity
is a `header_selectable_layers` row's, refuses every other header profile
with its catalogued code and cause, and restricts each declaration to its
selected layer's forms. Scope: FR-110-AC-1 to FR-110-AC-3 and FR-110-AC-5
to FR-110-AC-9.

This catches a resolver that accepts any catalog row as a profile, a parser
that still accepts a header `version` or `digest`, and a resolver that stops
at the first refusing profile.

## Test Procedure

Every unit starts with `language "ix:native" edition "1-draft";`, then its
header profiles, each `profile <alias> = "<identity>";`, then
`function f using v(): Boolean pure { true }`. `R` is
`DefinitionLock::pinned().entry(CatalogRole::Root)`; the test reads its
identity from the catalog, never from a literal.

1. Compile the unit whose one profile is `v` with `R`'s identity. Read the
   emitted lock.
2. Compile it with the identity `test:unknown-profile`.
3. Compile it with the `ieee_profile` row's identity, then with the
   `edition` row's.
4. Compile it with one further token, the word `version`, after the
   profile's identity string and before its `;`.
5. Compile a unit with profiles `v` (`R`'s identity), `w` (the
   `ieee_profile` row's identity) and `x` (`test:unknown-profile`). Then
   compile a unit with profiles `v` and `u`, both `R`'s identity.
6. For each QSpec AD-003 layer (`quire.state.core/v1`,
   `quire.state.queries/v1`, `quire.state.graph/v1`,
   `quire.value.complete/v1`, `quire.model.complete/v1`), compile a unit
   whose one profile `v` names the layer and whose one declaration uses
   only that layer's forms. Read each emitted lock.
7. Compile a unit whose profile `v` names `quire.state.core/v1` and whose
   declaration calls a named predicate, then one that declares a named
   predicate; then both units with `v` naming `quire.state.queries/v1`.

8. Under `make conformance`, read QSpec's `complete-value-lock.json` and
   `complete-value-selection-vectors.json` from `QSPEC_DIR`. Compare the
   lock with `DefinitionLock::pinned()`, and admit each selection vector
   through it.

Tag the test `#[trace("FR-110-AC-1", …, "FR-110-AC-9", "TC-490")]` over the
criteria each step backs.

## Expected Results

- Step 1 compiles. `profile_selections` is empty, and
  `definition_selections` holds `quire.value.complete/v1`,
  `quire.state.core/v1` and the `qualification_catalog` rows the
  package-selection rules select for the unit, and no other layer, written
  out as a literal in the test.
- Step 2 refuses `unknown_profile`/`unsupported-selection` at the identity
  literal's span, naming `v`, the selection and required role `root`. No
  package.
- Step 3 refuses `unknown_profile`/`wrong-selection-role` twice, once per
  compile, each retaining the selection and required role `root`.
- Step 4 refuses as a syntax error at the token after the identity string,
  before E3, with no package.
- Step 5's first unit refuses with exactly two refusals, `w`'s then `x`'s,
  and no package. Its second unit compiles.
- Step 6: each unit compiles; each lock's `definition_selections` hold the
  layer and every layer its `requires` closure names and no layer that
  requires it, written out per layer as a literal in the test.
- Step 7: the call refuses `unsupported_construct`/`expression-form` at the
  call's span and the predicate declaration refuses
  `unsupported_construct`/`declaration-form` at the declaration's span,
  each naming `quire.state.core/v1`, with no package; both
  `quire.state.queries/v1` units compile.
- Step 8: the catalog's (role, authority, identity) rows equal QSpec's
  `qualification_catalog` rows in order; its selection rules, trigger
  vocabulary and refusal codes equal QSpec's; QSpec's selection vectors give
  their recorded outcomes (FR-110-AC-9).

## Status

Partial: `a_header_profile_resolves_only_against_the_root_row`
(`qsl-replay/src/spine.rs`) backs steps 1-3 and 5. Step 4 fails until the
parser takes only `profile <alias> = "<identity>";` (FR-110 Status).
Step 8 is implemented (`conformance_catalog_matches_qspec_complete_value_lock`,
`qsl-semantics/tests/it/complete_value_lock.rs`, run by `make conformance`).
Step 1's layer-closure lock rows and steps 6 and 7 are planned.
