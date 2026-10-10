---
id: TC-739
title: "The v2 read refuses an inline nested term and names its limits' settings"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-264
    type: verifies
---
# TC-739: The v2 read refuses an inline nested term and names its limits' settings

## Description

Verify that a body outside the stratified grammar is a malformed wire, and
that the I2 read limits report their setting and clear when raised.

Scope: FR-264-AC-3, FR-264-AC-4, FR-264-AC-5.

## Test Procedure

1. Take an emitted v2 artifact, replace one `application` argument with an
   inline `application` term, and read it.
2. Read an emitted artifact with `i2.nodes` at `B`, below its node count;
   then raise `i2.nodes` through the v2 read limits' builder and through
   FR-255's settings operation given `i2.nodes=<n>`.
3. On a thread with a 512 KiB stack, read bytes that are 100,000 arrays deep
   under the default limits.

Tag the tests `#[trace("TC-739", "FR-264-AC-3")]`, `#[trace("TC-739", "FR-264-AC-4")]`, `#[trace("TC-739", "FR-264-AC-5")]`.

Verification status: step 2's limit kind, bound, count, setting `i2.nodes`,
`Locus::Artifact` and raise through the v2 read limits are tested here, and
the raise through FR-255's settings operation by TC-721.

## Expected Results

- Step 1: refused with code `malformed_wire`, with no limit outcome.
- Step 2: `StageFailure::Limit` with kind node count, bound `B`, IR's
  consumed count, setting `i2.nodes` and `Locus::Artifact`; each raised
  read verifies.
- Step 3: refused with code `malformed_wire`, with no limit outcome and no
  outcome naming a depth.

