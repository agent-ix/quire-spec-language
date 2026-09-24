---
id: TC-412
title: "The assembler resolves each using alias to a declared profile selection and refuses an undeclared one"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-412: The assembler resolves each using alias to a declared profile selection and refuses an undeclared one

## Description

Verify that E3 resolves a form's `using` alias against the unit's own
profile selections, which S2 carries, and that an alias naming no declared
profile selection refuses. There is no default profile (QSpec
`proposals/quire-v1/shared-grammar.md`).

This catches an assembler that ignores `using`, and one that falls back to
a default profile for an undeclared alias.

Scope: FR-091-AC-22.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. Assemble a unit whose only declaration is
   `function f using v(): Boolean pure { true }`. Read the profile selection
   recorded for `f`.
2. Assemble a unit holding that declaration and then
   `function g using w(): Boolean pure { true }`.
3. Assemble a unit that declares a second profile selection with alias
   `v`, and `f`.

Tag the test `#[trace("FR-091-AC-22", "TC-412")]`.

## Expected Results

- Step 1 returns a `PackageDeclarations` value. `f`'s recorded selection is
  the unit's profile selection with alias `v`.
- Step 2 returns one refusal holding an undeclared-alias error, code
  `missing_declaration`/`missing-selection`, that names `w` and the span of
  `g`'s `using` field. It returns no `PackageDeclarations` value.
- Step 3 returns a refusal holding a duplicate-alias error, code
  `ambiguous_declaration`/`ambiguous-name`, naming `v` and both selection
  spans, and no `PackageDeclarations` value.

## Status

Steps 2 and 3 are backed by `qsl-semantics` `check::assemble` tests: `using_aliases_resolve_to_the_units_profile_selections`. Step 1 is partial: the assembler admits `f` because its alias names a profile selection, but records no resolved selection, since nothing reads one yet.
