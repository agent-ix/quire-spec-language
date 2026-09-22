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

## Expected Results

- Step 1: both references are refused `missing_declaration` /
  `missing-name`.
- Step 2: each of the two uses is refused `ambiguous_declaration` /
  `ambiguous-name`; each refusal carries two loci, one per import binding
  `a`.
- Step 3: `l::R` resolves to `PackageNodeKey{package: <L@1's package_id>,
  node: <R's WireNodeId>}`; `l::Q` is refused `missing_declaration` /
  `missing-name`.

## Status

Planned; no test backs this case. The importing package's checker does not
yet resolve a name through import qualifiers. Steps 1 and 2 are the vectors
`l02_an_import_without_a_qualifier_binds_no_name` and
`l03_a_shared_qualifier_is_ambiguous_at_each_use` exercised against
`library`'s former `resolve_name`, which PR #299 removed from `library`
(FR-087 Description, item 3, owner ruling (b)).
