---
id: TC-185
title: "The replay request carries exactly the O-26 members and round-trips them exactly"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: verifies
---
# TC-185: The replay request carries exactly the O-26 members and round-trips them exactly

## Description

Verify that the typed replay request carries exactly the ADR-013 O-26
members (the O-25 packet plus the FR-070 envelope's state environment,
accounting limits, and S1-to-S4 stage limits) with no invented member, and
that a construct → serialize → read round trip preserves the package
reference, replay members, byte provision and limits exactly. A wrong
implementation this test would catch: a request builder that fills a
missing stage limit with a hard-coded default instead of requiring it from
the proving run, so two requests built from different proving runs with
different stage limits would silently converge on the same request identity.
Scope: FR-071-AC-1.

## Test Procedure

1. Build a replay request from a packet and an envelope whose S1-to-S4
   stage limits differ from QSL's own compiled-in defaults.
2. Inspect the request type's public fields/accessors and confirm each maps
   to a named ADR-013 O-26 member, with none unaccounted for.
3. Serialize the request and read it back.
4. Compare every member (package reference, `ReplaySource`, `QualifiedName`,
   arguments keyed by parameter node id, profile selections, proof bounds
   and domains, trace position, `backend`, state environment, accounting
   limits, stage limits, byte provision) field by field against the values
   from step 1.
5. Build a second request identical to the first except for one stage
   limit, and confirm the two requests' identities (RFC 8785 encoding of
   the request-identity members) differ.

## Expected Results

- Step 2 finds no request member outside the named O-26 set.
- The stage limits from step 1 are exactly the ones read back in step 4,
  not QSL's defaults.
- Every member from step 4 matches byte-for-byte.
- The two requests in step 5 have distinct identities.
