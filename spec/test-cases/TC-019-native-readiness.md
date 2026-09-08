---
id: TC-019
title: "Bounded malformed source corpus"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-002
    type: verifies
---

## Description

Bounded malformed source corpus. Type: Property; priority P1. Traces: FR-002-AC-6.

## Test Procedure

Enumerate truncations of a valid unit and the existing three-byte malformed alphabet, both alone and inserted into a valid unit. Observe each actual parser result.

## Expected Results

Every call terminates without panic. Successful variants retain exact submitted bytes; failures preserve source identity and bounded diagnostic spans. No claim that every mutation is invalid is made.

