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
FR-056-AC-7, FR-056-AC-42.

## Test Procedure

1. Admit a lifted package with N declarations at `normalize.record` = N, then at
   N − 1.
2. Offer a package with a stale digest at `normalize.record` = 0.
3. Run the bundle entry point over a bundle, then run the intake seam over the
   bytes that bundle lifts to with the same selection, definition and limits.
   Repeat each run.
4. Admit a package whose operation parameter, field and result carry `typeRef`
   `ix://quire/unit/ms` (FR-056-AC-42), assemble it with a unit declaring `ms`,
   then with a unit declaring no `ms`.

## Expected Results

- Step 1: the exact bound completes with N declarations; N − 1 is incomplete at
  `normalize.record` with no declaration.
- Step 2 refuses `stale_dependency`/`content-mismatch`, not an incomplete
  result: admission checks precede the first charge.
- Step 3: all four results are byte-identical.
- Step 4: intake admits the package with the `typeRef`s unresolved; the first assembly resolves each to `ms`; the second refuses `missing_declaration`/`missing-name` naming the member path.
