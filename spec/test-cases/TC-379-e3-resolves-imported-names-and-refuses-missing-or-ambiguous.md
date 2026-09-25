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
reference through its import qualifiers and the library lock, and refuses a
missing or ambiguous name with the E3 refusal codes. Scope: FR-087-AC-13.

## Test Procedure

1. Package `P` imports `L@1` (exporting `R`) with no `as` qualifier and
   declares no `R`. Check a `P` body that references `R`, and one that
   references `L::R`.
2. Package `P` imports `A@1` and `B@1`, both `as a`. Check a `P` body that
   uses `a::R` twice.
3. Package `P` imports `L@1` `as l`. Check a `P` body that references
   `l::R`, and one that references `l::Q` (`L@1` exports no `Q`).
4. Package `P` imports `L@1` `as l`. Check a `P` body that references
   `l::R` against a library lock that holds no selection for `L`.

## Expected Results

- Step 1: both references are refused `missing_declaration` /
  `missing-name`.
- Step 2: each of the two uses is refused `ambiguous_declaration` /
  `ambiguous-name`; each refusal carries two loci, one per import binding
  `a`.
- Step 3: `l::R` resolves to `PackageNodeKey{package: <L@1's package_id>,
  node: <R's WireNodeId>}`; `l::Q` is refused `missing_declaration` /
  `missing-name`.
- Step 4: `l::R` is refused `missing_declaration` / `missing-name`.

## Status

Steps 1 and 3 pass locally (QSL-255 part b): `qsl-replay`
`spine::dependency_tests::e3_resolves_an_imported_name_only_through_its_qualifier`,
with a library function `f` in place of `R`, since ADR-015 D-5 admits an
imported name only as a callee; `l::f`'s resolution to `{package_id, node}`
is asserted by `an_imported_call_is_typed_from_the_library_and_lowered_to_a_dependency_reference`.
Step 2 is not reachable as written: two imports `as a` are refused by the
assembler as a duplicate alias (`ambiguous_declaration`/`ambiguous-name`,
FR-091) before E3 sees a use. Step 4 is not reachable: ADR-015 D-1 refuses
an import with no supplied library at stage `intake`
(`missing_import`/`missing-selection`) before E3.
