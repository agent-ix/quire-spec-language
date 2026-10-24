---
id: TC-213
title: "Original declaration keys survive normalization unchanged"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
---
# TC-213: Original declaration keys survive normalization unchanged

## Description

Verify that every original declaration key the model binder's correspondence
names is byte-identical to the key FR-056 admission produced for it, with no
key added, dropped or altered, and that its per-key origin is preserved
verbatim. Scope: FR-081-AC-1, FR-081-AC-7 and FR-081-AC-8.

Catches an implementation that re-keys a declaration while building the
correspondence — for example, one that mints a fresh key from the
declaration's position in an internal vector, or that drops a declaration
whose supertypes/subsets/redefines edges are all empty on the assumption it
needs no correspondence entry. Both would still let every other assertion in
a shallow "the model binds" test pass, because the effective view would
still resolve names correctly for the *surviving* declarations; only a
direct key-set comparison against FR-056's own admitted output catches the
drop or the rekey.

## Test Procedure

1. Admit a domain package containing at least five declarations: two object
   types (one a supertype of the other), one field member, one operation
   member and one scalar type, each with a distinct declaration identity.
   Use object types A and B, field A/x, operation A/set and scalar Count.
   Give A the origin
   `{"source":{"sourceIdentity":"ix://test/origin/spec","path":"spec/A.md","startLine":2,"startColumn":1,"endLine":8,"endColumn":9}}`,
   B the distinct origin
   `{"source":{"sourceIdentity":"ix://test/origin/spec","path":"spec/B.md","startLine":3,"startColumn":2}}`,
   and Count the origin
   `{"generated":{"generatorIdentity":"ix://test/origin/generator","generatorVersion":"1.2.3","inputIdentities":["ix://test/origin/A","ix://test/origin/B"]}}`.
   A/x and A/set retain their actual source origins. These are the FCD
   common.schema.json origin variants: no source span is added to Generated.
2. Record the exact set of original `DeclarationKey` values FR-056 admission
   returns for these five declarations, and a map from each admitted key to
   its complete verbatim origin value. Preserve optional source members as
   present or absent and generated inputIdentities in their supplied order.
3. Run the model binder's normalization over the admitted declarations.
4. Collect the set of original declaration keys from the correspondence
   owner's complete retained intake inventory, including the scalar. Collect
   effective-entry and derivation-fact origin keys separately, deduplicated.
5. Compare the two sets for exact equality, and separately assert that no
   correspondence entry's original key differs by even one byte (package,
   node or digest-domain component) from its FR-056 source. Compare the
   retained key-to-origin map exactly with the admitted map, not just the
   set of origin values: each complete origin must belong to the same key.
6. Assert effective-entry/fact origin keys are a subset of the retained
   inventory, that the two object types and owned field/operation receive
   their selected qualify facts, and that the scalar receives no fabricated
   qualify fact, effective entry or extra effective-declaration/hash charge.
7. Repeat with reversed IR-node order, then with equal nodes selected under a
   different content digest of the same package identity. Original keys and
   qualifying effective-declaration identities remain equal; effective-view
   identities differ between selections. Each origin remains the actual FCD
   source or generated origin; compare the complete key-to-origin map after
   each run and invent no byte span for generated origins.
8. Apply separate retention mutants that swap A's and B's valid Source
   origins, alter only A's endColumn, remove only A's optional endLine, change
   Count's generatorVersion, reverse Count's inputIdentities, or replace
   Count's Generated origin with B's valid Source origin. Keep declaration
   keys unchanged. Every mutant fails exact per-key origin-map equality.

## Expected Results

The set from step 4 is exactly the set from step 2: no key is missing, no
key is added, and no key's bytes differ. A mutant that regenerates a key
from position, or drops a leaf declaration, produces a set that differs from
step 2's and fails the comparison. The retained per-key origin map equals the
admitted map for both node orders and both selections. Every origin-only
mutation in step 8 fails even though the key set remains equal; Generated
retains its generator payload and has no fabricated source/path/span member.
