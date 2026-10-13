---
id: TC-241
title: "Kind mapping runs interfaces first, and a port whose interface type is not an Interface refuses wrong-export"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-086
    type: verifies
---
# TC-241: Kind mapping runs interfaces first, and a port whose interface type is not an Interface refuses wrong-export

## Description

Verify two independent FR-086 obligations FR-152's kind-mapping rule states
together: (a) classification charges and resolves every interface before
any part, every part before any port, every port before any connection, and
every connection before any allocation; and (b) a port whose declared
interface-type reference does not itself classify as `Interface` refuses
wrong-export naming required kind `Interface` and the reference's actual
kind. Scope: FR-086-AC-5.

Catches an implementation that (a) charges components before interfaces
(the common, natural iteration order, since parts read more like the
"primary" construct), reversing FR-152's required order in a way invisible
to any test whose ports do not depend on the Interface's classification; and (b) admits a Port whose
interface-type reference resolves to an ordinary object type (or to
nothing) rather than an `Interface`, invisible to a test whose ports always
declare a genuine interface type.

## Test Procedure

1. Declare a domain package whose records, in declaration-key order, are:
   one Part, one Interface, one Port (owned by the Part, with a valid
   direction), all with ascending declaration keys such that the natural
   declaration-key order does *not* already put the Interface first.
2. Run systems classification and inspect the Port's kind and the meter's
   `systems.kind` charge count. The Port classifies only when the Interface
   it names was classified in the earlier phase, so the Port's kind
   observes the order.
3. Declare a second domain package with a Port whose declared interface-type
   reference names an ordinary object type (not an Interface-classified
   export).
4. Classify the second domain package.

## Expected Results

Step 2 charges three `systems.kind` units, one each for the Interface, the
Part and the Port, and the Port classifies `Port`, because the Interface is
charged and resolved before the Part and the Part before the Port,
regardless of their relative declaration-key order — interfaces-first is
independent of key order. Step 4's Port records a no-kind cascade with a wrong-export cause
naming required kind `Interface` and the reference's actual resolved kind
(the ordinary object type has no systems kind, so `none`). A mutant that
classifies in declaration-key order alone (ignoring FR-152's fixed
interfaces-first phase order) refuses the Port in step 2 whenever the fixture's key order
diverges from interfaces-first; a mutant that never checks a port's interface-type reference
against `Interface` admits the Port in step 4, failing the wrong-export
assertion.
