---
id: TC-995
title: "A resolving variant payload becomes a typed union member"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-995: A resolving variant payload becomes a typed union member

## Description

Verify that a resolving admitted payload type is retained as one typed union
position, a payload-free case remains nullary, and a Clock payload refused by
the FCD reader surfaces at QSL intake according to QSpec FR-154. Scope:
FR-056-AC-25.

## Test Procedure

1. Hand-author Semantic IR bytes, since the lift never emits a `payloadType` on
   an enumeration value: variant type `Tier` with cases `Standard` and `Gold`,
   where `Gold` has a resolving `payloadType` naming native `Boolean`. Admit and
   assemble the package.
2. Remove the payload from `Gold` and admit the package.
3. Set `Gold.payloadType` to `Clock`, which is not an admitted value type.

## Expected Results

- Step 1 admits both cases; `Standard` has no payload and `Gold` has one
  Boolean payload position. The model type assembles as a union composite with
  one member per case.
- Step 2 admits as TC-994 step 1, with both cases nullary.
- Step 3 refuses `invalid_model_binding`/`malformed-declaration` at node
  `ix://<package identity>/variant/Tier-Gold`, path `Variant.payloadType`, per
  QSpec FR-154; it admits no declaration of the package.
- Assertions compare typed code, cause, node identity and path.
