---
id: TC-995
title: "A variant case carrying a payload refuses as an unsupported declaration form"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-995: A variant case carrying a payload refuses as an unsupported declaration form

## Description

Verify that a case with a `payloadType` refuses as an unsupported declaration form, not a malformed one. Scope: FR-056-AC-25.

## Test Procedure

1. Hand-author Semantic IR bytes, since the lift never emits a `payloadType` on an enumeration value: variant type `Tier` with cases `Standard` and `Gold`, where `Gold` has `payloadType` naming a declared type, so the reader admits it. Admit the package.
2. Remove the payload from `Gold` and admit.

## Expected Results

- Step 1 refuses `unsupported_construct`/`declaration-form` at node `ix://<package identity>/variant/Tier-Gold`, path `Variant.payloadType`, never `invalid_model_binding`, and admits no declaration of the package. `Standard` raises no refusal.
- Step 2 admits as TC-994 step 1.
- Assertions compare typed code, cause, node identity and path.
