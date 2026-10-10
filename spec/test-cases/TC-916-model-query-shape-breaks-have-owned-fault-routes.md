---
id: TC-916
org: agent-ix
title: "Model-query shape breaks have QSL-owned fault routes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-916: Model-query shape breaks have QSL-owned fault routes

## Description

Exercise model-query shape checks at their detecting producers, not by
constructing a fault and asserting its fields. Scope: FR-090-AC-14,
FR-090-AC-16. Private seams may supply malformed post-check state; public
admission remains unchanged.

## Test Procedure

1. Invoke the real allInstances model-query producer with a collection whose
   element type is not Reference. Record its result and query charge events.
2. Invoke the real lookup model-query producer with a non-Reference value.
   Record its result and query charge events.
3. Reach lookup's Option wrapping with a non-Option result type, once after
   a present successful query and once after an absent query in empty mode.
   Use valid nonempty identities so identity conversion does not fail first.
4. Pair each malformed fixture with its corrected shape. Include an absent
   empty result of the proper Option type and a present result of that type.
5. Exercise unresolved target type, foreign universe, above-maximum,
   absent-undefined and absent-refused controls. Repeat step 3 with budget
   denial or cancellation before the query completes.

## Expected Results

Steps 1 and 2 return `Err(InternalFault)` at S6a with invariants
`model-query-reference-element-expected` and
`model-query-reference-value-expected`, respectively, before any query charge.
Step 3 returns `model-query-option-result-expected` with the ordinary lookup
charge prefix preserved: LookupKey followed by LookupResultRetain, accounting
for two result units for a present empty-mode result and one for an absent
empty-mode result. Wrapping adds no charge. All three faults have code
`runtime_invariant`, category internal failure and no manufactured kernel cause.

Step 4 completes with the proper Reference or Option payload. Step 5 keeps
the existing typed model/kernel refusal, undefined or Incomplete route;
none becomes a shape fault when the shape producer was not reached. The
test does not introduce an extra Option payload-type check absent from the
specified wrapping producer.
