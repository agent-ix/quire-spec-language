---
id: TC-227
title: "allInstances requires subtype closure, never a partial set"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-227: allInstances requires subtype closure, never a partial set

## Description

Verify that `allInstances<T>` on a successfully admitted population returns
a typed incomplete result naming uncovered subtype closure and returns the
complete deduplicated set only when every effective subtype of `T` is
covered, in canonical reference-key order independent of input order. Scope:
FR-084-AC-2 and FR-090-AC-21. The separate object-closure admission case
(open extent yields unknown-closure and no binding) is owned by TC-226 under
FR-084-AC-1; no query runs without an admitted binding. This test targets
the "no partial substitute" exit condition: it must fail against an
implementation that returns a collection when subtype closure is missing.

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
3. Query `allInstances<Item>` through `CheckedPackage::evaluate` against this
   admitted binding.
4. Declare a second population, extent `closed`, member types `{Item,
   SpecialItem}`, covering every conforming subtype, with the same members,
   and query `allInstances<Item>` against it, once with members supplied in
   one order and again with members supplied in a different order.

## Expected Results

Step 3 returns `FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause))`;
`cause.undefined_record()` has reason `incomplete-closure`, queried type
`Item`, and the uncovered subtype `SpecialItem`, with no collection. It does
not return a refusal or a partial set containing only the plain `Item`
members it managed to visit. Step 4 returns the complete set (every `Item`
and `SpecialItem` member) in the same canonical reference-key order
regardless of input member order. A mutant that returns the partially-gathered
`Item`-only set in step 3 produces a non-empty, plausible-looking `Set`
there, failing the result-type assertion.

The subtype-incomplete result is evaluated through the public
`CheckedPackage::evaluate` adapter. Its outcome is
`FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause))`, whose
`undefined_record()` has reason `incomplete-closure`, `closure` `subtype`,
queried `type` `Item`, and the complete missing-subtype list containing
`SpecialItem`; it exposes no member collection. The exact query and every
missing effective subtype are retained structurally at that boundary.
