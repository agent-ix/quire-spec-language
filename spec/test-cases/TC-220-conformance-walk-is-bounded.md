---
id: TC-220
title: "A conformance ancestor walk past the bound refuses instead of truncating"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: verifies
---
# TC-220: A conformance ancestor walk past the bound refuses instead of truncating

## Description

Verify that an ancestor chain exceeding the checker's bound refuses with a
named resource-exhaustion cause and reports neither conformance nor
non-conformance, while a chain at exactly the bound is admitted. Scope:
FR-082-AC-3.

Catches an implementation that silently truncates the ancestor walk at the
bound and returns `false` (non-conformant) for the unreached remainder — a
defect indistinguishable from a correct "does not conform" result unless the
test specifically checks that the outcome is the distinct incomplete/refused
variant, not a `false` conformance verdict. A truncate-and-report-false
mutant would pass any test that only asserts "conformance is false past the
bound."

## Test Procedure

1. Declare a linear chain of object types one longer than the conformance
   walk's bound (each `supertypes: [previous]`), so that the deepest type's
   conformance to the shallowest requires a walk one step past the bound.
2. Query whether the deepest type conforms to the shallowest.
3. Declare a second chain at exactly the bound's length, and query the same
   conformance relation for it.
4. Inspect the result type of step 2 (not merely its truthiness) and of step
   3.

## Expected Results

Step 2's result is the distinct resource-exhaustion outcome, naming the
bound, not a boolean `false`. Step 3's result is a completed conformance
verdict of `true`. A mutant that truncates at the bound and returns `false`
produces a boolean result in step 2 that is indistinguishable in shape from
a genuine non-conformance verdict, failing the result-type assertion.
