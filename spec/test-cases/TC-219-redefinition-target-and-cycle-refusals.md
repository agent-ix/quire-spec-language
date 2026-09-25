---
id: TC-219
title: "A missing redefinition target and a supertype cycle each refuse with a named cause"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: verifies
---
# TC-219: A missing redefinition target and a supertype cycle each refuse with a named cause

## Description

Verify two independent negative paths: a redefinition naming a target that
is not declared on any conforming supertype refuses with a
redefinition-target cause, and a declared `supertypes` cycle refuses
specialization for every declaration on the cycle, deriving no conformance
edge across it. Scope: FR-082-AC-2, and FR-082-AC-6 for the same two
refusals at the expression checker.

Catches an implementation that silently treats a missing redefinition target
as "no redefinition" (admitting the member as if it were newly declared,
rather than refusing) — which passes any test that only checks a *present*
redefinition target — and separately catches an implementation that detects
a cycle but still derives a conformance edge for one direction of it before
refusing (visible only by then asserting that neither direction of the cycle
conforms to the other).

## Test Procedure

1. Declare an object type `X` with a field `redefines: missing_field`, where
   no supertype of `X` declares `missing_field`. Run redefinition checking.
2. Declare object types `M` (`supertypes: [N]`) and `N` (`supertypes: [M]`),
   forming a two-node cycle. Run specialization checking over both.
3. Inspect the outcome of step 1 and the outcome of step 2.
4. For step 2, additionally query whether `M` conforms to `N` and whether
   `N` conforms to `M`.

## Expected Results

Step 1 refuses with a redefinition-target cause naming `missing_field`; no
member is admitted for `X`'s redefinition. Step 2 refuses specialization for
both `M` and `N` with a specialization-cycle cause. Step 4 reports that
neither conforms to the other (no edge derived across the cycle in either
direction). A mutant that admits the missing target as an unrelated new
member passes step 1's "some outcome exists" check but fails a check that no
member is admitted; a mutant that derives one cycle edge before refusing
fails step 4.
