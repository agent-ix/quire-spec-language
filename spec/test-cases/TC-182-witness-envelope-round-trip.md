---
id: TC-182
title: "A positive witness envelope round-trips its transcript and every O-25 member exactly"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: verifies
---
# TC-182: A positive witness envelope round-trips its transcript and every O-25 member exactly

## Description

Verify that a construct → serialize → read round trip of the witness
envelope preserves the admitted transcript byte-for-byte and every ADR-013
O-25 member exactly: obligation identity, clause occurrence key,
`package_id` and every `RawSourceRef` digest, semantic profile selections,
proof bounds and declared domains, the `backend` member, trace position,
and the `ReplaySource` variant with its payload. This also verifies that no
decoding step interprets the transcript's rendered/display text (FR-070
constraint FR-070-CON-1). A wrong implementation this test would catch: a
round trip that re-derives the occurrence key from the node id at read time
instead of carrying the stored key, so two occurrences of a structurally
identical node collapse into one on read-back, silently losing the "keep
occurrences of identical nodes apart" property ADR-013 O-07 requires.
Scope: FR-070-AC-3, FR-070-CON-1.

## Test Procedure

1. Construct a witness envelope with two structurally identical clause
   occurrences distinguished only by occurrence key (same node id, different
   role/ordinal), and with a `RawSourceRef` digest, a set of semantic
   profile selections, a proof bound, a `backend` member, a trace position,
   and a `ReplaySource::Witness` payload.
2. Serialize and read the envelope back.
3. Compare every listed member, and the transcript, field by field between
   the constructed and read-back envelopes.
4. Construct a second envelope on the `ReplaySource::Input` arm (no
   transcript) and repeat steps 2-3 for its members.
5. Confirm that no step in the round trip or in comparing the two
   occurrences from step 1 calls a text-rendering or display-formatting
   function on the transcript.

## Expected Results

- Every O-25 member and the transcript are identical, byte for byte,
  between the constructed and read-back envelopes for both the `Witness`
  arm (step 3) and the `Input` arm (step 4).
- The two structurally identical occurrences from step 1 remain
  distinguishable by occurrence key after the round trip.
- No display-text or rendering function participates in the round trip or
  the comparison (step 5).
