---
id: TC-184
title: "A family adds a typed witness payload through a typed extension point, not an untyped map"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: verifies
---
# TC-184: A family adds a typed witness payload through a typed extension point, not an untyped map

## Description

Verify that a family-specific witness payload (standing in for #186's state
`forall` payload) attaches through an extension point that is itself typed
— a generic parameter or a trait bound on the common envelope — and not an
untyped, string-keyed map. A "zero lines changed in the common envelope's
module" diff alone does not distinguish a genuine typed extension point
from an untyped `HashMap<String, serde_json::Value>` "extras" bag: a family
using such a bag also changes no line of the common module, since the bag
was already there generically. The falsifier that actually distinguishes
them is the extension point's own type signature, plus whether retrieving
the family payload by an arbitrary string key compiles at all. A wrong
implementation this test would catch: a common envelope carrying a
`family_extras: HashMap<String, serde_json::Value>` field (or equivalent),
which every earlier version of this test case — "inspect the constructors,
attach a payload, diff the module" alone — would have passed. Scope:
FR-070-AC-5.

## Test Procedure

1. Inspect the common witness envelope's public type definition as it
   exists after FR-070 lands, and identify the exact type of its extension
   point (the member or type parameter through which a family attaches its
   payload).
2. Confirm that type is one of: a generic type parameter on the envelope
   (e.g. `Envelope<P>`, monomorphized per family), or a trait bound /
   trait-object field (e.g. `payload: Box<dyn FamilyWitnessPayload>` or
   `payload: P where P: FamilyWitnessPayload`). Confirm it is not
   `HashMap<String, _>`, `BTreeMap<String, _>`, `serde_json::Value`, or any
   other type whose keys or shape are not fixed by the Rust type system.
3. Attempt to compile a call site that retrieves a family payload's field by
   an arbitrary string key (e.g. `envelope.payload["bound_index"]` or
   `envelope.get_extra("bound_index")`) rather than through a named,
   statically-typed accessor.
4. Define a new, family-owned payload type (in a separate module, standing
   in for #186's state-`forall` payload) that attaches to the envelope's
   extension point and carries a fact no other family needs (e.g. the
   quantified variable's bound index), construct an envelope carrying it,
   and read it back.
5. Diff the common envelope's own module against its state before step 4 to
   confirm no line changed.

## Expected Results

- Step 2 confirms the extension point's type is a generic parameter or a
  trait bound, never an untyped or string-keyed map.
- Step 3's string-keyed retrieval attempt fails to compile: no such API
  exists on the envelope type.
- The new family-owned payload type in step 4 compiles and attaches without
  modifying the common envelope's type, constructors, or accessors, and the
  envelope round-trips the payload exactly.
- The diff in step 5 shows zero changes to the common envelope's own
  module, now meaningful because steps 2-3 have already ruled out the
  untyped-bag shape that would trivially pass this diff alone.
