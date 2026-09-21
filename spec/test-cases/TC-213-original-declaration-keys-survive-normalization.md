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
key added, dropped or altered. Scope: FR-081-AC-1.

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
   member and one scalar type, each with a distinct artifact id.
2. Record the exact set of original `DeclarationKey` values FR-056 admission
   returns for these five declarations.
3. Run the model binder's normalization over the admitted declarations.
4. Collect the set of original declaration keys the resulting correspondence
   names (one per `ViewEntry`/derivation-fact origin, deduplicated).
5. Compare the two sets for exact equality, and separately assert that no
   correspondence entry's original key differs by even one byte (package,
   node or digest-domain component) from its FR-056 source.

## Expected Results

The set from step 4 is exactly the set from step 2: no key is missing, no
key is added, and no key's bytes differ. A mutant that regenerates a key
from position, or drops a leaf declaration, produces a set that differs from
step 2's and fails the comparison.
