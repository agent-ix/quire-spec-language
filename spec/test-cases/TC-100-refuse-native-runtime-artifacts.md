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

## Expected Results

Failures retain selected provenance, stage and JSON or structural cause. No
partial artifact is returned. Resource stops remain incomplete; retries are fresh.
