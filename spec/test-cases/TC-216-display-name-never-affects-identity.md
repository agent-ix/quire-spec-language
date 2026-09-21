---
id: TC-216
title: "Changing only a display name leaves every key, identity and ordering unchanged"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
---
# TC-216: Changing only a display name leaves every key, identity and ordering unchanged

## Description

Verify that the model binder never reads a `title`/`displayName` field when
computing an identity, a key, a digest or an ordering. Scope: FR-081-AC-4.
This is the test the "no display-name identity" exit condition names
directly: it must fail against an implementation that keys on display names.

Catches an implementation that, anywhere in the binder — ordering a sort key,
disambiguating two same-shaped declarations, or forming part of a digest
preimage — reads the declaration's title instead of, or in addition to, its
artifact-id-based declaration key. Any such read is invisible to a test that
only checks correctness against one fixed set of titles; it only surfaces
when the same structural package is run twice with different titles and the
outputs are compared byte-for-byte.

## Test Procedure

1. Admit domain package P1: an object type declared with id `Order` and
   title `Order`, one field, one operation.
2. Admit domain package P2: byte-identical to P1 except the object type's
   title is changed to `Sales Order (legacy)`; the artifact id, every field,
   type, multiplicity and edge is unchanged.
3. Run the model binder's normalization over P1 and over P2 independently.
4. Compare, byte for byte: every original declaration key, every effective
   declaration identity, and the correspondence's declaration ordering,
   between the P1 run and the P2 run.

## Expected Results

Every compared value in step 4 is byte-identical between the two runs. A
mutant that folds `displayName` into any digest preimage, sort key or
disambiguation rule produces at least one differing byte between the P1 and
P2 results, failing the comparison.
