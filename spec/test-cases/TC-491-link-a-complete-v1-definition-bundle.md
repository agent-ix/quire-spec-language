---
id: TC-491
title: "library::bundle links a complete-V1 bundle and refuses each closure, facet and limit defect"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-111
    type: verifies
---
# TC-491: library::bundle links a complete-V1 bundle and refuses each closure, facet and limit defect

## Description

Verify `library::bundle::link_bundle` over a caller-supplied
`DefinitionCatalog`. Scope: FR-111-AC-1 to FR-111-AC-7. These are the
scenarios the former `tests/it/complete_package.rs` and
`complete::package_tests` covered against the retired resolver, moved to
the relocated module with their QSpec tags (`QSpec-FR-131-AC-1` to
`QSpec-FR-131-AC-3`, `QSpec-FR-339-AC-3`).

## Test Procedure

Fixtures build `Definition`s with `ReaderAuthority::fixture()`: one per
complete-V1 facet, together carrying the 176-capability inventory, with
dependency edges between them.

1. Link the full root set; read the bundle's definitions, facets,
   capabilities, identity and limits. Validate the capabilities against a
   broader and a narrower known set.
2. Link against catalogs that lack a root's identity, and hold a root whose
   edge names an absent definition.
3. Link two roots of one identity at two selections, a root whose closure
   reaches another selection of a root's identity, and a catalog with a
   dependency cycle.
4. Link a closure missing one facet, and one missing one capability.
5. Compute the identity of the golden preimage, then change one
   definition's role, dependencies, capabilities and bytes in turn.
6. For each `PackageLimits` field, build and link at its bound and one
   past it; raise `definitions` above the default and build a catalog the
   default refuses.
7. For every refusal above, check its (code, cause) pair against the
   catalog.

## Expected Results

- Step 1 links, with nine facets and every closed definition; the
  identity is the same under both known sets, and only
  `validate_known_capabilities` refuses the narrower one.
- Step 2 refuses `unknown_profile`/`unsupported-selection` and
  `missing_import`/`missing-selection`, each naming the root's index.
- Step 3 refuses `ambiguous_declaration`/`conflicting-authority` twice,
  naming both selections, and `invalid_package`/`definition-cycle` with
  the cycle in path order.
- Step 4 refuses `invalid_package`/`feature-set-mismatch` naming the facet,
  and `unknown_required_feature`/`unsupported-feature`.
- Step 5 matches the golden vector, and each change gives a different
  identity.
- Step 6 admits each bound and refuses one past it with
  `resource_exhausted`/`insufficient-next-charge`; the raised ceiling
  admits.
- Step 7: every pair is listed by `quire.native.diagnostics/v1`.

