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
a typed incomplete result naming subtype closure and queried type `T` when
`T` conforms to no declared member type, and otherwise returns the complete
deduplicated set in canonical reference-key order independent of input order.
Conformance is reflexive: a declared member type covers itself and every type
that conforms to it, but a declared child does not cover its parent or sibling. Scope:
FR-084-AC-2 and FR-090-AC-21. The separate object-closure admission case
(open extent yields unknown-closure and no binding) is owned by TC-226 under
FR-084-AC-1; no query runs without an admitted binding. This test targets
the "no partial substitute" exit condition: it must fail against an
implementation that returns a collection when subtype closure is missing.
The implementation binders are delivered by companion PR #763:
`qsl-eval/tests/it/model_reference_queries.rs::tc_227_incomplete_subtype_closure_is_undefined_through_checked_evaluate`
for the S6a incomplete result, and
`qsl-semantics/tests/it/model_population.rs::all_instances_requires_declared_subtype_closure`
for the complete-coverage/canonical-order control, and
`qsl-semantics/tests/it/model_population.rs::all_instances_does_not_walk_unrelated_type_catalog`
for the no-catalog-walk guard. Both names were verified on the companion PR
branch; the source implementation remains in that code PR.

Catches an implementation that returns a complete collection when `T` does
not conform to any declared member type, or returns a partial collection
instead of the typed incomplete result.

## Test Procedure

1. Query `T` with a declared parent and then a declared grandparent; both
   cover the query. Query `T` when `T` itself is declared; reflexivity covers
   that query.
2. Query a parent `T` with only its child declared, and query one child with
   only its sibling declared. Both are incomplete because `T` does not
   conform to a child or sibling.
3. Through `CheckedPackage::evaluate`, query an incomplete case against an
   admitted binding. This is the separate S6a incomplete-closure case, not an
   object-closure admission failure.
4. For a covered case, admit equivalent members in different input orders
   and query `allInstances<T>` against both bindings.

## Expected Results

Step 2/3 returns `FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause))`;
`cause.undefined_record()` has reason `incomplete-closure`, queried type
`T`, and subtype marker `T`, with no collection. It does not return a
refusal or a partial set. Step 4 returns the complete set in the same
canonical reference-key order regardless of input member order. A mutant
that returns a set in an incomplete case fails the result-type assertion.

The subtype-incomplete result is evaluated through the public
`CheckedPackage::evaluate` adapter. Its outcome is
`FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause))`, whose
`undefined_record()` has reason `incomplete-closure`, `closure` `subtype`,
queried `type` `T`, and subtype marker `T`; it exposes no member collection.
The query and marker are retained structurally at that boundary.
