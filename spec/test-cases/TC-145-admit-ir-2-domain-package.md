---
id: TC-145
title: "Admit a Semantic IR 2.0.0 domain package as model declarations"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: verifies }
---
# TC-145: Admit a Semantic IR 2.0.0 domain package as model declarations

## Description

Verify that the compiler admits a domain package only through quire-rs
extraction, the `agent-ix-extraction-frontend` lift and the
`agent-ix-semantic-ir` reader, and that each admitted type definition becomes one
model declaration keyed by its artifact identity. Scope: FR-056-AC-1,
FR-056-AC-2, FR-056-AC-5, FR-056-AC-6.

## Test Procedure

1. Run intake over a small spec artifact bundle with the real module manifests.
   The bundle declares one identified record with fields, one value record, one
   enumeration, one operation, one relation with a declared name and one
   population. Inspect the admission report.
2. Recompute the `sha256-jcs` digest of the lifted IR document independently and
   compare it with the reported domain package digest. Check that every
   declaration key is (domain package identity, IR node identity, `sha256-jcs`)
   and that each IR node identity equals its source artifact id.
3. Change only one artifact's `displayName`, re-run intake and compare
   declaration keys, export records, ordering and the linker's binding of a
   native reference. Then give two artifacts equal `displayName` values.
4. Offer a document the `agent-ix-semantic-ir` reader refuses, and separately a
   document with contract version `1.2.0`.
5. Remove the relation's `name`, then separately its `sourceSpan`. Point a
   reference member at an artifact id absent from the bundle.
6. Lower each intake limit to zero, to the exact usage of step 1 and to one
   below it.

## Expected Results

- Step 1 yields exactly one declaration per type definition, each with its bound
  Quire meaning, source locus and export records; the relation yields one
  `relationship` export with its declared name and span.
- Step 2 digests and keys match the independent computation.
- Step 3 leaves every key, export, ordering and binding byte-identical; equal
  `displayName` values stay two distinct declarations.
- Step 4 refuses the package: the reader case retains every reader diagnostic
  with its node and locus, and the version case returns
  `unsupported-ir-contract-version`. No declaration is admitted.
- Step 5 refuses the relation with `incomplete-relation-declaration` and the
  referring declaration with `dangling-model-reference`, each located.
- Step 6: zero and one-short limits return `resource_exhausted` with dimension,
  limit and usage and no admitted package; the exact limit admits.
