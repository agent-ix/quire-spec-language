---
id: TC-184
title: "A family adds a typed witness payload through the extension point without changing the common envelope"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: verifies
---
# TC-184: A family adds a typed witness payload through the extension point without changing the common envelope

## Description

Verify that a family-specific witness payload (standing in for #186's state
`forall` payload) can be built as a typed consumer of the witness
envelope's extension point without adding a variant, field, or special case
to the common envelope type or its constructors. A wrong implementation
this test would catch: a common envelope whose only way to carry a
family-specific fact is an untyped `HashMap<String, Value>` "extras" bag or
a hard-coded closed enum of known families, either of which would force an
edit to the shared type (or an unsafe stringly-typed lookup) for every new
family instead of a genuinely open extension point. Scope: FR-070-AC-5.

## Test Procedure

1. Inspect the common witness envelope's public type definition and
   constructors as they exist after FR-070 lands.
2. Define a new, family-owned payload type (in a separate module, standing
   in for #186's state-`forall` payload) that attaches to the envelope's
   extension point and carries a fact no other family needs (e.g. the
   quantified variable's bound index).
3. Construct an envelope carrying this new payload, and read it back.
4. Diff the common envelope's own module against its state before step 2 to
   confirm no line changed.

## Expected Results

- The new family-owned payload type in step 2 compiles and attaches without
  modifying the common envelope's type, constructors, or accessors.
- The envelope from step 3 round-trips the family-owned payload exactly.
- The diff in step 4 shows zero changes to the common envelope's own
  module.
