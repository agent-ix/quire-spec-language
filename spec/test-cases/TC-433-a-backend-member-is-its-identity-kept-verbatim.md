---
id: TC-433
title: "A backend member is its identity, kept verbatim"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
---
# TC-433: A backend member is its identity, kept verbatim

## Description

Verify that the ADR-013 O-19 `backend` member, the backend identity alone,
reads back the candidate it was written from and keeps its identity
verbatim. Scope: FR-075-AC-6.

## Test Procedure

1. Build a candidate with identity ` Kani/1 `. Write its `backend` member
   and read it back.
2. Read a `backend` member whose identity is empty.

## Expected Results

- Step 1 reads back a candidate equal to the original, with identity
  ` Kani/1 ` unchanged.
- Step 2 refuses the empty identity.
