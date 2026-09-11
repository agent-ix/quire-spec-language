---
id: TC-100
title: "Refuse malformed and exhausted native input reads"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-024
    type: verifies
---
## Description

Refuse stale selections, incompatible envelopes and invalid native input bodies.

## Test Procedure

Mutate identity/digest, kind/version, required/unknown/duplicate fields, identifier
and integer/digest forms, and arena indices. Lower byte and structural limits,
then retry with defaults. Include deeply nested malformed input.
Match every selection refusal and both JSON decode phases by Rust variant,
with observed values and code/stage classification checked independently.
Decode each public runtime record and every value variant directly as well as
through artifact readers. Use valid objects as positive controls, then replace
them with field-ordered positional arrays, remove each field, add an unknown
field and duplicate a raw JSON key. Compare explicit null and missing invocation
results. Exercise bare-hex digest length, case, character and prefix refusals.

## Expected Results

Failures retain selected provenance, stage and JSON or structural cause. No
partial artifact is returned. Resource stops remain incomplete; retries are fresh.
