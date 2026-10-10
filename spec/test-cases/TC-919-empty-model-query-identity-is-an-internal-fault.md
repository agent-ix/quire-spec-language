---
id: TC-919
org: agent-ix
title: "An empty model-query object identity is an internal fault"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-919: An empty model-query object identity is an internal fault

## Description

Exercise the real model-query identity bridge, including the public path
through an admitted population binding. Scope: FR-090-AC-13, FR-090-AC-16,
FR-090-AC-17. An empty raw key is not an empty `ObjectId`: no test may invent
an admitted ObjectReference that the ObjectId constructor forbids.

## Test Procedure

1. Admit a population binding with one conforming selected member whose
   object identity is the empty string, a matching universe and maximum one.
   Call and evaluate a checked `allInstances<T>(p)` through the public package
   APIs with a recorded binding identity and matching maximum. Observe the
   actual bridge failure, diagnostic and meter events.
2. Reach that same bridge from the selected-key path and the successful
   lookup-key path using private raw-key fixtures. For lookup use an empty
   `ReferenceKey` at the raw query/bridge seam, not a forged ObjectId. Record
   the bare and Option result retention variants separately.
3. Repeat with a nonempty object identity as a completing control. Repeat
   step 1 with unknown population identity and mismatched maximum through
   public admission. Repeat with selected cardinality above maximum.
4. Repeat the empty-key runs with cancellation or a denied charge at each
   earlier query charge point. For every reached bridge run compare its event
   list with the identical successful query prefix through retention.
5. Drive the empty-key public evaluation fault through the existing replay
   fault path and inspect its terminal diagnostic and replay basis.

## Expected Results

Step 1 reaches S6a and returns `CallFailure::Fault`, code `runtime_invariant`,
category internal failure, stage `S6a`, invariant
`model-query-object-identity-empty`. There is no Evaluation, refusal record,
partial successful collection, fabricated identity or synthetic kernel cause.
The public allInstances run charges its entry FunctionCall, PopulationVisit,
CollectionBound and CollectionResultRetain in the ordinary order. For its
single selected member, retention accounts for the two result units (the
collection and member), exactly once, before the bridge fails.

Step 2 returns the same S6a fault. Lookup preserves LookupKey and the already
performed LookupResultRetain charge (one result unit bare, two for Option),
without rerunning lookup or charging bridge conversion. Direct bridge calls
with no query prefix add no meter event. Step 3's valid key completes; unknown
identity and maximum mismatch remain `CallFailure::Input` before S6a, and
above-maximum remains a model family refusal, not an identity fault.

Step 4 retains every earlier Incomplete outcome and its existing budget or
cancellation data when the bridge is not reached. Reached faults retain only
the ordinary charge prefix, with no fault-specific event or later retention.
Step 5 produces `TerminalValue::Failed` with the same stage and invariant,
internal-failure diagnostic and unavailable replay basis, not Inconclusive.
