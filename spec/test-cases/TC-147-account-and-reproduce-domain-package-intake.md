---
id: TC-147
title: "Account for and reproduce domain-package intake"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: verifies }
---
# TC-147: Account for and reproduce domain-package intake

## Description

Verify `normalize.record` accounting under `ModelNormalizationLimitsV1` and
byte-identical intake results from both entry points. Scope: FR-056-AC-6,
FR-056-AC-7, FR-056-AC-42, FR-056-AC-44, FR-056-AC-45, FR-056-AC-50.

## Test Procedure

1. Admit a lifted package with N declarations at `normalize.record` = N, then at
   N − 1.
2. Offer a package whose identity differs from the selected one at `normalize.record` = 0.
3. Run the bundle entry point over a bundle, then run the intake seam over the
   bytes that bundle lifts to with the same selection, definition and limits.
   Repeat each run.
4. Admit a package whose operation parameter, field and result carry `typeRef`
   `ix://quire/unit/ms` (FR-056-AC-42), assemble it with a unit declaring `ms`,
   then with a unit declaring no `ms`.
5. Admit a package with clock members in an operation parameter, an operation
   result, an alias target, an aliased field, an optional field, a record, a sequence `items`, a map `values` and a multi-valued clock field
   (FR-056-AC-44), then a package whose alias target, sequence items and map
   values carry `ix://quire/unit/ms`, assembled with a unit declaring `ms` and
   with one declaring none (FR-056-AC-45).
6. Admit a package with a `reference` to `Clock`, clock and unit constraints, a `reference` to `ms` and an `ms` payload; separately offer a variant case whose `payloadType` is `Clock` (FR-056-AC-25, AC-50).

## Expected Results

- Step 1: the exact bound completes with N declarations; N − 1 is incomplete at
  `normalize.record` with no declaration.
- Step 2 refuses with FR-154's named cause for a package whose identity differs, not an incomplete
  result: admission checks precede the first charge.
- Step 3: all four results are byte-identical.
- Step 4: intake admits the package with the `typeRef`s unresolved; the first assembly resolves each to `ms`; the second refuses `missing_declaration`/`missing-name` naming the member path.
- Step 5: intake admits the clock package with no declaration for any clock; the unit package assembles to `Quantity<ms>`, `Sequence<Quantity<ms>>` and `Map<Text, Quantity<ms>>`, and refuses `missing_declaration`/`missing-name` at `TypeDefinition.target`, `TypeDefinition.items` and `TypeDefinition.values` when `ms` is absent.
- Step 6: the clock reference and constraints admit with no clock declaration; the unit constraint admits; the unit reference target refuses `malformed-declaration` at `TypeDefinition.target`; the unit payload refuses `unsupported_construct`/`declaration-form` at `Variant.payloadType`. The separate Clock payload refuses the same code and cause at node `ix://<package identity>/variant/Tier-Gold`, path `Variant.payloadType`, before any package declaration is returned (QSpec FR-154-AC-25; FR-056-AC-25).
