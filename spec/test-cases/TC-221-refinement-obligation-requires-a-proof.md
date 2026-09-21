---
id: TC-221
title: "A narrowing field redefinition requires an established postcondition, not just a narrower declared shape"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: verifies
---
# TC-221: A narrowing field redefinition requires an established postcondition, not just a narrower declared shape

## Description

Verify that a narrowing field redefinition is refused unproved-refinement
when no postcondition fact establishes the narrowing, and is admitted only
once that obligation is established. Scope: FR-082-AC-4.

Catches an implementation that admits a narrowing redefinition on the
strength of its *declared* type/multiplicity alone (treating the type-system
narrowing as self-evidently safe) without checking for an actual proof
obligation — which would satisfy the acceptance test if it were run only on
the positive case; the negative case (a narrower declared type but no
established postcondition) is required to catch this.

## Test Procedure

1. Declare a supertype field with a wide value type or multiplicity.
2. Declare a subtype redefinition of that field with a strictly narrower
   type or multiplicity, and no postcondition clause establishing the
   narrowing anywhere in the subtype's operations.
3. Run field-redefinition checking over the subtype's declaration.
4. Add a postcondition clause to the subtype that establishes the narrowing
   fact for that field, and re-run field-redefinition checking.

## Expected Results

Step 3 refuses with cause unproved-refinement, naming the missing obligation
kind. Step 4 admits the redefinition. A mutant that checks only the declared
type/multiplicity narrowing and ignores the postcondition obligation admits
step 3 as well, failing the refusal assertion.
