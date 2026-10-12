---
id: TC-227
title: "allInstances requires both object and subtype closure, never a partial set"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-227: allInstances requires both object and subtype closure, never a partial set

## Description

Verify that `allInstances<T>` returns a typed incomplete result naming the
missing closure when either object closure or subtype closure for `T` does
not hold, and returns the complete deduplicated set only when both closures
hold, in canonical reference-key order independent of input order. Scope:
FR-084-AC-2 and FR-090-AC-21. This test targets the "no partial substitute"
exit condition: it must fail against an implementation that returns a
collection when either closure is missing.

Catches an implementation that, on discovering mid-query that the
population's declared member types do not cover every effective subtype of
`T`, returns the members it has already visited as if they were the
complete answer, rather than discarding the partial collection and
reporting incompleteness — invisible to a test that only checks "the query
did not silently return `false`/`None`," since a partial `Set` is neither.

## Test Procedure

1. Declare object type `Item` with a subtype `SpecialItem` (`supertypes:
   [Item]`).
2. Exercise `allInstances<Item>` through the public
   `CheckedPackage::evaluate` boundary with object closure not established.
   Use the test fixture's closure-controlled input to reach the query without
   substituting an admission refusal or unknown-closure result.
3. Declare a population with extent `closed`, declared member types
   `{Item}` only (not `SpecialItem`), and admit a binding whose members
   include several plain `Item`s and at least one `SpecialItem`, where the
   `SpecialItem` member itself is individually well-typed and admissible
   (object closure holds).
4. Query `allInstances<Item>` through `CheckedPackage::evaluate` against this
   binding.
5. Declare a second population, extent `closed`, member types `{Item,
   SpecialItem}`, covering every conforming subtype, with the same members,
   and query `allInstances<Item>` against it, once with members supplied in
   one order and again with members supplied in a different order.

## Expected Results

Step 2 returns `FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause))`;
`cause.undefined_record()` has reason `incomplete-closure`, closure `object`,
queried type `Item`, and no collection. Step 4 returns the typed incomplete
result naming the missing subtype closure (`SpecialItem` not
covered), and no collection — not a set containing only the plain `Item`
members it managed to visit. Step 5 returns the complete set (every `Item`
and `SpecialItem` member) in the same canonical reference-key order
regardless of input member order. A mutant that returns the partially-gathered
`Item`-only set in step 4 produces a non-empty, plausible-looking `Set`
there, failing the result-type assertion.

The subtype-incomplete result is evaluated through the public
`CheckedPackage::evaluate` adapter. Its outcome is
`FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause))`, whose
`undefined_record()` has reason `incomplete-closure`, `closure` `subtype`,
queried `type` `Item`, and the complete missing-subtype list containing
`SpecialItem`; it exposes no member collection. The exact query and every
missing effective subtype are retained structurally at that boundary.
