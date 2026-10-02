---
id: TC-178
title: "The proof-result reader refuses an oversized envelope"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: verifies
---
# TC-178: The proof-result reader refuses an oversized envelope

## Description

Verify that the proof-result reader refuses an envelope whose encoded size
exceeds the configured reader bound without returning a truncated or
partially-populated envelope. A wrong implementation this test would catch:
a reader that streams and returns whatever it decoded up to the bound,
silently truncating `results`/`dispositions`. Scope: FR-069-AC-4.

## Test Procedure

1. Construct a well-formed FR-331 envelope whose encoded size exceeds the
   reader's configured bound by padding `dispositions` with repeated valid
   entries until the bound is crossed, and read it.

## Expected Results

- The oversized envelope refuses with a bound-exceeded cause, and the reader
  returns no envelope value at all.
