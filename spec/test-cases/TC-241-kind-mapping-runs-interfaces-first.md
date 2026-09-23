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

Known gap: `qsl-semantics/src/model/systems.rs::classify` does not yet implement either
half. Interface types are collected inline while scanning every record and
are never charged through `charge_kind`, so they are not their own first
charged phase; and the endpoint loop never checks whether an endpoint's
declared interface-type reference classifies as `Interface`. This test case
fails against current code; see FR-086's Dependencies note.

Known gap (instrumentation): step 2's "instrumented to record each record's
classification order" also names an observability hook that does not exist
today. `Meter::admitted_charges()` records admission order, but every
`classify` charge uses the single `ChargePoint::SystemsKind` variant
regardless of which construct (interface, part, port, connection,
allocation) is being classified, so the recorded sequence is an
undifferentiated run of `SystemsKind` entries — it cannot distinguish "an
Interface was charged" from "a Part was charged." Asserting step 2 requires
either a per-kind charge point or a separate classification-order recorder,
neither of which exists yet. This test case's assertion is kept as written,
not weakened to a charge count (a count cannot detect an ordering defect at
all); the hook itself is owed by the implementation. Remaining work: #120.

Catches an implementation that (a) charges components before interfaces
(the common, natural iteration order, since parts read more like the
"primary" construct), reversing FR-152's required order in a way invisible
to any test that does not inspect charge order; and (b) admits a Port whose
interface-type reference resolves to an ordinary object type (or to
nothing) rather than an `Interface`, invisible to a test whose ports always
declare a genuine interface type.

## Test Procedure

1. Declare a domain package whose records, in declaration-key order, are:
   one Part, one Interface, one Port (owned by the Part, with a valid
   direction), all with ascending declaration keys such that the natural
   declaration-key order does *not* already put the Interface first.
2. Run systems classification, instrumented to record each record's
   classification order (or inspect the meter's charge sequence for
   `systems.kind`).
3. Declare a second domain package with a Port whose declared interface-type
   reference names an ordinary object type (not an Interface-classified
   export).
4. Classify the second domain package.

## Expected Results

Step 2's classification order charges and resolves the Interface record
before the Part record, and the Part before the Port, regardless of their
relative declaration-key order — interfaces-first is independent of key
order. Step 4's Port records a no-kind cascade with a wrong-export cause
naming required kind `Interface` and the reference's actual resolved kind
(the ordinary object type has no systems kind, so `none`). A mutant that
classifies in declaration-key order alone (ignoring FR-152's fixed
interfaces-first phase order) passes step 2 only when key order happens to
already put interfaces first, failing whenever the fixture's key order
diverges; a mutant that never checks a port's interface-type reference
against `Interface` admits the Port in step 4, failing the wrong-export
assertion.
