---
id: TC-141
title: "Preserve opaque semantic-trigger identity through native temporal v2"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-053, type: verifies }
---
# TC-141: Preserve opaque semantic-trigger identity through native temporal v2

## Description

Verify that native-temporal v2 retains the observation-owner semantic trigger as
opaque bytes, has no textual fallback, and remains strictly version-separated
from v1.

## Test Procedure

Construct event-triggered requests with valid UTF-8 and non-UTF-8 trigger byte
populations. Produce, evaluate and strict-read each v2 request/result. Mutate
one byte, exchange request/result triggers, alter a correction trigger, empty
the value, replace its canonical binary encoding, and offer v1 documents to v2
and v2 documents to v1. Attempt substitutions using every non-identity field
the current v1 API exposed.

## Expected Results

Only byte-exact v2 trigger values round-trip. Every mutation changes identity
or refuses at the source boundary. Neither reader translates versions, invents
a string nor returns partial temporal truth.
