---
id: TC-214
title: "Structurally identical declarations from distinct originals never collapse to one effective identity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
---
# TC-214: Structurally identical declarations from distinct originals never collapse to one effective identity

## Description

Verify that two admitted declarations with identical declared shape but
distinct original declaration keys always yield distinct effective
declaration identities. Scope: FR-081-AC-2.

Catches an implementation that computes the effective-identity digest over
only the declared shape (field names, types, multiplicities) and omits the
original declaration key from the preimage — an easy mistake, since the
shape-only digest still "looks" content-addressed and would pass any test
that only checks determinism within one declaration. Two distinct
declarations with the same shape would then silently collapse into one
effective declaration, which is exactly the identity-equivalence collapse
FR-081-AC-2 forbids.

## Test Procedure

1. Admit a domain package declaring two independent object types, `OrderA`
   and `OrderB`, each with exactly one field `total` of the same value type
   and the same typed multiplicity, and no supertype relationship between
   them.
2. Run the model binder's normalization.
3. Read the effective declaration identity the correspondence assigns to
   `OrderA`'s own effective type and to `OrderB`'s.
4. Assert the two effective identities are unequal.
5. Assert both original declaration keys (`OrderA`, `OrderB`) remain present
   in the correspondence as two separate entries, each naming its own
   effective identity.

## Expected Results

Step 4's identities differ. Step 5 finds two distinct correspondence entries.
A mutant that hashes only declared shape produces equal identities in step 4
and a correspondence with one entry silently representing both original
declarations, failing step 5.
