---
id: TC-227
title: "Reusable semantic library resolution"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/FR-307
    type: verifies
---

# TC-227: Reusable semantic library resolution

## Description

Resolve a compatible diamond and reject conflict, cycle, ambiguity and invalid migration.

## Test Procedure

Use the exact version-locked subject and inputs named by FR-307. Execute its positive case, each declared boundary, and each explicit refusal/non-conclusive mutation through the real local Rust boundary; compare the complete typed result and artifact accounting with the criterion.

Select the `quire.value.complete/v1` definitions at revision `1-draft.1` by
their exact DefinitionRefs in
[`complete-value-lock.json`](../../proposals/quire-v1/definitions/complete-value-lock.json).
Unless a row states limits, run it under limits large enough that no charge is
denied. Integer literals, parameters, `let` names and field projections make no
charge. Every `ill-typed` expectation is a type-checking
`refused { code: ill_typed }` made before any charge.

Build libraries and importing packages as CheckedPackage V2 artifacts, and
compare each import digest with the library `package_id`.

| Vector | Input | Expected result/disposition |
| --- | --- | --- |
| L01 | `import "L" version "1" digest "<package_id of L@1>" as l;` using `l::R`; then the same import with a digest of L's raw source bytes; then the same import with `version "2"` and L@1's `package_id` when only L@1 is available | admitted with export identity (L package key, `R` node key); `refused { code: stale_dependency, cause: byte-digest-mismatch }`; `refused { code: stale_dependency, cause: revision-mismatch }` |
| L02 | `import "L" version "1" digest "<package_id of L@1>";` without `as`, then a use `R` of L's export; then a use `L::R` | the import is admitted and verified but binds no qualifier; `refused { code: missing_declaration, cause: missing-name }` at `R`; the same refusal at `L::R`, because the library name is not a qualifier |
| L03 | two imports of different libraries both `as a`, then `a::R` | each use is `refused { code: ambiguous_declaration, cause: ambiguous-name }` listing both import paths |
| L04 | P imports A and B; A and B each import L version 1 with the same `package_id`; then B imports L version 1 with a different `package_id`; then B imports L version 2 | one export identity for `R` and one lock selection for L; `refused { code: invalid_package, cause: conflicting-definition }` listing `P -> A -> L` and `P -> B -> L`; the same refusal |
| L05 | A imports B and B imports A | `refused { code: invalid_package, cause: definition-cycle }` whose dependency-edge payload lists `A -> B -> A` |
| L06 | P imports libraries `Z` and `A` in that source order | the lock lists `A` then `Z` |
| L07 | L is migrated to L' and P to P', with evidence recorded against P's export identity | L' and P' have new `package_id`s, and the old evidence keeps its old key and is not relabeled |
| L08 | L is migrated to L' with a changed export, and L' is presented with L's `package_id`; then P imports L' by that digest | `refused { code: invalid_package, cause: invalid-value }` at member path `/package_id`, because the recomputed digest of L''s identity preimage differs; P's import resolves no L' and binds no lock selection or evidence |

## Expected Results

Every positive and boundary result matches FR-307; each invalid, unsupported or exhausted mutation returns its exact typed disposition with source/provenance, emits no approximation, and preserves independent sibling results.
