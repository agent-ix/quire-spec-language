---
id: TC-242
title: "A selected object's reference key names the same most-specific type through every conforming query"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: verifies
---
# TC-242: A selected object's reference key names the same most-specific type through every conforming query

## Description

Verify that the same underlying object, selected via `allInstances<A>` and
separately via `allInstances<C>` (where the object's own most-specific
effective type `C` is a proper subtype of `A`), yields the identical
`ReferenceKey` in both results — never a key recomputed from the queried
static type. Scope: FR-084-AC-6, quire-specification FR-153-AC-6.

Backed by the existing test
`tests/model_population.rs::l01_all_instances_selects_subtype_population_once`,
which already carries the `FR-153-AC-6` trace tag and asserts this exact
guarantee: querying a fixture population's `b1` member (most-specific type
`model.B`) through `allInstances<model.A>` and through
`allInstances<model.B>` returns the same `ReferenceKey` in both cases
(`b1_via_a == b1_via_b`), and each `ReferenceKey.type_identity` is
`model.B`'s effective identity, never `model.A`'s.

This moved here from an earlier, FR-081-scoped draft that tried to state
the guarantee at the runtime `Reference<T>`/static-upcast level, citing
quire-specification FR-151's "reference upcasts only" as its dispatch-time
type-checking rule. That citation supports type-checking dispatch
arguments, not a reference-identity guarantee, and the draft's test
procedure re-read one map lookup twice with no mechanism for a static type
to enter — nothing could make it fail. FR-153-AC-6 is the requirement that
actually states this identity guarantee, at the population-query level
`crate::model` owns and can construct directly.

Catches an implementation that derives a `ReferenceKey.type_identity` from
the queried static type `T` instead of from the object's own most-specific
effective type — for example, stamping the query's own `T` onto the
returned key rather than reading it from the admitted member's own
original type.

## Test Procedure

1. Admit a domain package with object type `A` (base), object type `B`
   (`supertypes: [A]`), where a population member `b1`'s most-specific
   effective type is `B`.
2. Admit a closed population binding covering both `A` and `B`.
3. Call `allInstances<A>(p)` and locate `b1`'s `ReferenceKey` in the result.
4. Call `allInstances<B>(p)` and read `b1`'s `ReferenceKey` in the result.

## Expected Results

The `ReferenceKey` found in step 3 and the one found in step 4 are
byte-identical, and both carry `type_identity` equal to `B`'s effective
identity, never `A`'s. A mutant that stamps the queried type `T` onto the
returned key produces a different `type_identity` between step 3 and step
4 (`A` vs. `B`), failing the byte-identical assertion.
