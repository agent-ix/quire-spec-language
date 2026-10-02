---
id: TC-379
title: "E3 resolves an imported name to its PackageNodeKey and refuses a missing or ambiguous one"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: verifies
---
# TC-379: E3 resolves an imported name to its PackageNodeKey and refuses a missing or ambiguous one

## Description

Verify that the importing package's own checker resolves a qualified
reference through its import qualifiers and refuses a missing name with the
E3 refusal codes, and that a duplicate import alias and an import with no
supplied library are refused before E3 by the stages that own them. Scope:
FR-087-AC-13.

## Test Procedure

1. Package `P` imports `L@1` (exporting `R`) with no `as` qualifier and
   declares no `R`. Check a `P` body that references `R`, and one that
   references `L::R`.
2. Package `P` imports `A@1` and `B@1`, both `as a`, with both libraries
   supplied. Compile a `P` body that uses `a::R` twice.
3. Package `P` imports `L@1` `as l`. Check a `P` body that references
   `l::R`, and one that references `l::Q` (`L@1` exports no `Q`).
4. Package `P` imports `L@1` `as l`. Compile a `P` body that references
   `l::R` with no library supplied as `L`.

## Expected Results

- Step 1: both references are refused `missing_declaration` /
  `missing-name`.
- Step 2: the assembler refuses the second alias `a` with
  `ambiguous_declaration` / `ambiguous-name`, naming the alias and both
  import spans. E3 does not run, so neither use of `a::R` is refused.
- Step 3: `l::R` resolves to `PackageNodeKey{package: <L@1's package_id>,
  node: <R's WireNodeId>}`; `l::Q` is refused `missing_declaration` /
  `missing-name`.
- Step 4: the import is refused `missing_import` / `missing-selection` at
  stage `intake`, at the import's identity string. E3 does not run, so
  `l::R` is not refused.

## Status

Steps 1 and 3 pass locally: `qsl-replay`
`spine::dependency_tests::e3_resolves_an_imported_name_only_through_its_qualifier`,
with a library function `f` in place of `R`, since ADR-015 D-5 admits an
imported name only as a callee; `l::f`'s resolution to `{package_id, node}`
is asserted by `an_imported_call_is_typed_from_the_library_and_lowered_to_a_dependency_reference`.
Steps 2 and 4 are planned in their rewritten form.

## References

- Linear QSL-263 (steps 2 and 4 rewritten to the stages that refuse them).
