---
id: TC-177
title: "The proof-result envelope maps every FR-331 outcome to its exact O-16 category"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: verifies
---
# TC-177: The proof-result envelope maps every FR-331 outcome to its exact O-16 category

## Description

Verify that the typed proof-result envelope's reader maps each of the eight
ADR-013 O-16 outcome categories from its FR-331 terminal record with no
collapsing, that a vacuous `Proved` (a Kani run with zero SUCCESS checks)
maps to `inconclusive`/`kani_vacuous_proof` rather than `proved`, and that a
`tested` backend result is never promoted to `proved`. A wrong
implementation this test would catch: mapping `declined` and `unsupported`
to the same category (collapsing refusal into unsupported), or reading a
vacuous `Proved` run as ordinary success because the reader only inspects
the outer `Proved` tag and never the SUCCESS-check count. Scope:
FR-069-AC-1.

## Test Procedure

1. Construct one minimal FR-331 terminal record for each of: `proved` (with
   at least one SUCCESS check), `proved` with zero SUCCESS checks
   (vacuous), `tested`, `refuted`, `declined` (with a typed refusal cause),
   `unsupported` (with a typed unavailability cause), `incomplete` (timeout
   cause), `incomplete` (cancellation cause), and `failed`.
2. Read each record into the proof-result envelope.
3. Compare the resulting category against the ADR-013 O-16 category table
   row for that FR-331 value.
4. Re-run step 2 with the vacuous `proved` record substituted for an
   ordinary `proved` record's typed cause omitted, to confirm the reader
   inspects the SUCCESS-check count and not merely the outer disposition
   tag.

## Expected Results

- Each of the eight records maps to its distinct O-16 category with no two
  distinct FR-331 inputs mapping to the same category incorrectly, and the
  `tested` record's category is never `success`/`proved`.
- The vacuous `proved` record maps to `inconclusive` with cause
  `kani_vacuous_proof`, never to `success`.
- A timeout and a cancellation both map to `incomplete` but keep their
  distinct causes, never collapsing into one undifferentiated "incomplete"
  value.
