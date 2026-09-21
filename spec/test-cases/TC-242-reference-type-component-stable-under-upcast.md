---
id: TC-242
title: "A reference's type component names the same most-specific effective type before and after an upcast"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
---
# TC-242: A reference's type component names the same most-specific effective type before and after an upcast

## Description

Verify that an object's effective declaration identity, read through a
conforming supertype's viewpoint (the model-binder-level analogue of a
reference upcast: querying the same object via a wider static type), is
the same `EffectiveId` as read through the object's own most-specific type
directly — never recomputed, widened or narrowed by which conforming type
the query names. Scope: FR-081-AC-8.

This exercises the identity guarantee at the declaration-identity level
`crate::model` actually owns and can construct directly, rather than at the
runtime `Reference<T>` value level, which is built by the evaluator
(FR-047's territory) from the `EffectiveId` this binder produces — the
guarantee this test checks is the one that value-level construction relies
on.

Catches an implementation that derives a different identity, or a
truncated/lossy one, depending on which conforming ancestor type a caller
names when resolving an object's effective type — for example, computing
`EffectiveId` from the queried static type's own declaration key instead of
from the object's actual most-specific effective declaration.

## Test Procedure

1. Admit a domain package with a three-level chain: object type `A`
   (abstract base), object type `B` (`supertypes: [A]`), object type `C`
   (`supertypes: [B]`, concrete, most-specific).
2. Run normalization and read `C`'s own effective declaration identity
   directly from the correspondence.
3. Confirm `C` conforms to `B` and to `A` under FR-082's conformance
   relation (the upcast-eligible paths).
4. Resolve the same underlying object's effective identity as it would be
   observed through the `B`-typed and `A`-typed viewpoints (for example,
   any lookup keyed by the object's most-specific type, regardless of the
   static type of the request).

## Expected Results

Step 4's identity, observed through either viewpoint, is byte-identical to
step 2's identity read directly from `C`. A mutant that computes an
identity from the queried static type (`B` or `A`) rather than from the
object's own most-specific effective type produces a different `EffectiveId`
in step 4, failing the byte-identical assertion.
