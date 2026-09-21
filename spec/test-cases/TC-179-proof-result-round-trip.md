---
id: TC-179
title: "A positive proof-result envelope round-trips its backend identity, tool pin and dispositions exactly"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: verifies
---
# TC-179: A positive proof-result envelope round-trips its backend identity, tool pin and dispositions exactly

## Description

Verify that constructing a proof-result envelope, serializing it, and
reading it back preserves the `backend` member (provider identity and
manifest digest), the executor/tool pin, and every per-item disposition
byte-for-byte. A wrong implementation this test would catch: a serializer
that re-derives the `backend` member's `manifest_digest` at read time
instead of carrying the stored bytes, so a round trip silently repairs a
digest that should have stayed exactly as produced, masking a real
tool-identity drift. Scope: FR-069-AC-3.

## Test Procedure

1. Construct a proof-result envelope from an FR-331 record naming a
   specific `backend` (identity string and manifest digest) and tool pin,
   with two or more per-item dispositions.
2. Serialize the envelope and read it back through the same reader used in
   TC-177/TC-178.
3. Compare the read-back envelope's `backend` member, tool pin, and every
   per-item disposition against the values constructed in step 1, field by
   field.
4. Mutate one byte of the serialized `manifest_digest` before reading it
   back, and confirm the round trip in step 2 would have detected the
   difference rather than silently recomputing a matching digest.

## Expected Results

- The `backend` member's identity and manifest digest, the tool pin, and
  every per-item disposition are identical, byte for byte, between the
  constructed envelope and the read-back envelope.
- The mutated-digest case in step 4 is distinguishable from the unmutated
  case (the reader either refuses the mutated envelope or the comparison in
  step 3 shows the mismatch); the digest is never silently regenerated to
  match.
