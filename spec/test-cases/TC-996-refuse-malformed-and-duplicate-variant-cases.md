---
id: TC-996
title: "Empty, spanless, mis-shaped and duplicate variant cases refuse with their causes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-996: Empty, spanless, mis-shaped and duplicate variant cases refuse with their causes

## Description

Verify the intake and reader refusals of a variant type's cases. Scope: FR-056-AC-26.

## Test Procedure

Every step offers hand-authored Semantic IR bytes unless stated.

1. `Tier` with an empty `variants` list.
2. `Tier` with one case whose `origin` is a generated origin with no source span.
3. A construct with meaning `quire.meaning.model.variant-type/v1` and shape `record`.
4. `Tier` with two cases named `Gold` and one identity `variant/Tier-Gold`. Then lift a bundle whose enumeration rows slug alike (`Snap_Shot`, `Snap__Shot`).
5. `Tier` with two cases named `Gold` carrying distinct identities, which the reader admits. Then one case whose identity is not `variant/Tier-<name>`.
6. `Tier` with a case that has no `name`.

## Expected Results

- Steps 1 to 3 refuse `invalid_model_binding`/`malformed-declaration` at node `ix://<package identity>/Tier` with paths `TypeDefinition.variants`, `TypeDefinition.variants[0].origin` and `TypeDefinition.kind` respectively.
- Step 4 is refused before intake: the reader's duplicate-identity diagnostic is retained with no declaration, and the lift raises `UNSLUGGABLE_NAME` for the slug-alike rows (FR-095 collision case (b)).
- Step 5 refuses `invalid_model_binding`/`conflicting-binding` at node `ix://<package identity>/Tier`, path `TypeDefinition.variants[1]`. The mis-minted identity refuses `invalid_model_binding`/`malformed-declaration` at the case node whose identity is not `variant/Tier-<name>`, path `Variant.identity`. A case that repeats a name stops after `conflicting-binding`: its identity, origin and payload checks are skipped, because a repeated name with a distinct identity is always mis-minted too.
- Step 6 is refused by the reader's schema, diagnostic retained, no declaration.
- Assertions compare typed code, cause, node identity and path.
