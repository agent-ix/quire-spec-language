---
id: TC-179
title: "A positive proof-result envelope round-trips its backend identity and dispositions exactly"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: verifies
---
# TC-179: A positive proof-result envelope round-trips its backend identity and dispositions exactly

## Description

Verify that constructing a proof-result envelope, serializing it, and
reading it back preserves the `backend` member (the backend identity) and
every per-item disposition byte-for-byte. Scope: FR-069-AC-3.

## Test Procedure

1. Construct a proof-result envelope from an FR-331 record naming a
   specific `backend` identity, with two or more per-item dispositions.
2. Serialize the envelope and read it back through the same reader used in
   TC-177/TC-178.
3. Compare the read-back envelope's `backend` member and every per-item
   disposition against the values constructed in step 1, field by field.

## Expected Results

- The `backend` identity and every per-item disposition are identical, byte
  for byte, between the constructed envelope and the read-back envelope.
