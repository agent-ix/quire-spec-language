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

1. Construct two witness envelopes, A and B, whose clause occurrence keys
   name the same node id but a different role/ordinal (the envelope carries
   one occurrence key each, per FR-070's Behavior; the two envelopes stand
   in for two source occurrences of one structurally identical node). Give
   both the same `RawSourceRef` digest, semantic profile selections, proof
   bound, `backend` member, trace position, and a `ReplaySource::Witness`
   payload — identical in every member except the occurrence key.
2. Serialize and read both envelopes back.
3. Compare every listed member, and the transcript, field by field between
   each envelope's constructed and read-back form.
4. Compare envelope A's read-back occurrence key against envelope B's: they
   name the same node id but must remain distinguishable by role/ordinal.
5. Construct a third envelope on the `ReplaySource::Input` arm (no
   transcript) and repeat steps 2-3 for its members.
6. Confirm that no step in the round trip, or in the comparison in step 4,
   calls a text-rendering or display-formatting function on the transcript
   or on any member.

## Expected Results

- Every O-25 member and the transcript are identical, byte for byte,
  between each envelope's constructed and read-back form (step 3), for both
  the `Witness` arm (envelopes A and B) and the `Input` arm (step 5).
- Envelopes A and B's occurrence keys, identical in node id, remain
  distinguishable by role/ordinal after the round trip (step 4) — a
  read-back that re-derived the occurrence key from the node id alone,
  losing the role/ordinal distinction, would make A and B compare equal on
  this member and fail this check.
- No display-text or rendering function participates in the round trip or
  the comparison (step 6).
