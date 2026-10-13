---
id: TC-990
title: "Duplicate, missing and absent variant cases refuse as malformed declarations"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-990: Duplicate, missing and absent variant cases refuse as malformed declarations

## Description

Verify the malformed-declaration refusals of a variant type's cases. Scope: FR-056-AC-26.

## Test Procedure

1. Lift `Tier` with two cases named `Gold`. Admit it.
2. Lift `Tier` with an empty `variants` list.
3. Lift `Tier` with one case missing its name, then one missing its span.
4. Lift `Tier` with cases `Gold` and `gold`.

## Expected Results

- Step 1 refuses the second `Gold` with `invalid_model_binding`/`malformed-declaration`, naming the name, owner and both spans.
- Step 2 and each run of step 3 refuse `invalid_model_binding`/`malformed-declaration`.
- Step 4 admits two distinct `variant` exports.
- Assertions compare typed code, cause and locus.
