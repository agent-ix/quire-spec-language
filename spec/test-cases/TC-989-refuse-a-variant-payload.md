---
id: TC-989
title: "A variant case carrying a payload refuses as an unsupported declaration form"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-989: A variant case carrying a payload refuses as an unsupported declaration form

## Description

Verify that a case with a `payloadType` refuses as an unsupported declaration form, not a malformed one. Scope: FR-056-AC-25.

## Test Procedure

1. Lift `Tier` with cases `Standard` and `Gold`, where `Gold` carries `payloadType` naming a declared type. Admit it.
2. Remove the payload from `Gold` and admit.

## Expected Results

- Step 1 refuses `unsupported_construct`/`declaration-form` at `Gold`, naming the case and `payloadType`, never `invalid_model_binding`, and admits no declaration of the package. `Standard` raises no refusal.
- Step 2 admits as TC-988 step 1.
- Assertions compare typed code, cause and locus.
