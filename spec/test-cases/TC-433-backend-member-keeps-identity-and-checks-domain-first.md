---
id: TC-433
title: "A backend member keeps its identity and checks its digest domain first"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
---
# TC-433: A backend member keeps its identity and checks its digest domain first

## Description

Verify that the ADR-013 O-19 `backend` member reads back the candidate it was
written from, keeps its identity verbatim, and checks the digest domain before
the digest string. Scope: FR-075-AC-6.

## Test Procedure

1. Build a candidate with identity ` Kani/1 ` and a
   `quire.tool-manifest.jcs/v1` manifest digest. Write its identity, domain
   label and digest string, and read them back.
2. Read a member with a valid 64-hex digest under `quire.source.bytes/v1`,
   then with the digest string `not-hex` under the same domain.
3. Read a member with no domain, then with the unknown label `tool-manifest`.
4. Under `quire.tool-manifest.jcs/v1`, read a member whose digest is 64
   uppercase hex digits, then one whose digest is `ab`.

## Expected Results

- Step 1 reads back a candidate equal to the original, with identity
  ` Kani/1 ` unchanged.
- Step 2 refuses both reads for the wrong domain; the `not-hex` digest is not
  reported.
- Step 3 refuses for an absent domain, then for an unknown domain.
- Step 4 refuses for a non-lowercase-hex digest, then for a digest of length 2.
