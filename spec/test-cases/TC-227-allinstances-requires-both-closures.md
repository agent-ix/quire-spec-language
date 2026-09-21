---
id: TC-227
title: "allInstances requires both object and subtype closure, never a partial set"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: verifies
---
# TC-227: allInstances requires both object and subtype closure, never a partial set

## Description

Verify that `allInstances<T>` returns a typed incomplete result naming the
missing closure when subtype closure for `T` does not hold, even though
object closure holds, and returns the complete deduplicated set only when
both closures hold, in canonical reference-key order independent of input
order. Scope: FR-084-AC-2. This test targets the "no partial substitute" exit
condition applied to closed-population queries: it must fail against an
implementation that returns whatever members it has already gathered when
subtype closure is missing.

Catches an implementation that, on discovering mid-query that the
population's declared member types do not cover every effective subtype of
`T`, returns the members it has already visited as if they were the
complete answer, rather than discarding the partial collection and
reporting incompleteness — invisible to a test that only checks "the query
did not silently return `false`/`None`," since a partial `Set` is neither.

## Test Procedure

1. Declare object type `Item` with a subtype `SpecialItem` (`supertypes:
   [Item]`).
2. Declare a population with extent `closed`, declared member types
   `{Item}` only (not `SpecialItem`), and admit a binding whose members
   include several plain `Item`s and at least one `SpecialItem`, where the
   `SpecialItem` member itself is individually well-typed and admissible
   (object closure holds).
3. Query `allInstances<Item>` against this binding.
4. Declare a second population, extent `closed`, member types `{Item,
   SpecialItem}`, covering every conforming subtype, with the same members,
   and query `allInstances<Item>` against it, once with members supplied in
   one order and again with members supplied in a different order.

## Expected Results

Step 3 returns the typed incomplete result naming the missing subtype
closure (`SpecialItem` not covered), and no collection — not a set
containing only the plain `Item` members it managed to visit. Step 4 returns
the complete set (every `Item` and `SpecialItem` member) in the same
canonical reference-key order regardless of input member order. A mutant
that returns the partially-gathered `Item`-only set in step 3 produces a
non-empty, plausible-looking `Set` there, failing the result-type assertion.
