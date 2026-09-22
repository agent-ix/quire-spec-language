---
id: TC-385
title: "FamilyRefusal::catalog_code is exhaustive and returns only catalogued codes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-385: FamilyRefusal::catalog_code is exhaustive and returns only catalogued codes

## Description

Verify FR-090-AC-4. For every `FamilyRefusal` variant, `catalog_code()`
returns a code and cause pair that the `quire.native.diagnostics/v1`
revision FR-322 selects defines. The method matches every variant with no
`_` arm (ADR-013 O-17, C-15). Scope: FR-090-AC-4.

This catches two faults: a `catalog_code()` that invents a code the catalog
does not define (O-17 forbids this), and a `_` arm that lets a new variant
compile without its own code.

FR-090-OQ-2 blocks this test for `FamilyNotNativelyEvaluable`: the selected
revision (`1-draft.6`) has no code for that variant yet.

## Test Procedure

1. Build one value of every `FamilyRefusal` variant. The list is written in
   the test body, one entry per variant.
2. Call `catalog_code()` on each value.
3. Compare each returned `(code, cause)` pair with the pair the test
   expects for that variant. Each expected pair is a code and cause that the
   catalog revision FR-322 selects defines. The test names that revision and
   the catalog row it takes the pair from (ADR-013 O-17, Validation and
   diagnostics).
4. Build the crate under the repository's seam-probe configuration
   (`make seam-probe`, FR-063). Add `FamilyRefusal`'s `catalog_code()` match
   to the checked-in seam-probe list, so that a probe-only variant fails
   that match with `E0004`.

Tag the test `#[trace("FR-090-AC-4", "TC-385")]`.

## Expected Results

- Every step-3 pair equals its expected catalogued pair.
- Step 1's list has as many entries as `FamilyRefusal` has variants. Step 4
  shows that a variant with no arm fails to compile, so the list cannot fall
  behind the enum without the build failing.

## Status

Planned; no test backs this case. Blocked for `FamilyNotNativelyEvaluable` by FR-090-OQ-2.
