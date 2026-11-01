---
id: TC-1283
title: "Dispatch rows remain paired with their call occurrences"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-494
    type: verifies
---
# TC-1283: Dispatch rows remain paired with their call occurrences

## Description

Verify that two calls with the same operation spelling retain distinct
producer decisions through emission, including each call's context, candidate
identities and effective-precondition identities. Scope: FR-494-AC-2 and AC-3.

## Test Procedure

1. Compile a unit whose checked graph contains two dispatched calls with the
   same operation member spelling but distinct application occurrences and
   receiver contexts. Give the operations distinct selected-operation
   witnesses. Arrange for the linked tables to have distinguishable
   candidate selection keys, body identities and effective-precondition
   identities.
2. Capture each application's S3 linked table, then emit the package and
   inspect the two dispatch rows and their application/occurrence identities.
   For every candidate, compare its `selected_operation` witness with the
   exact operation witness of its own parent row.
3. Permute source declaration order without changing checked identities or
   either linked table, compile again, and inspect row order and contents.
4. In an adverse copy of the emitted carrier, replace one candidate's
   `selected_operation` witness with the other row's valid witness and pass the
   copy to IR-1214's typed dispatch read.

## Expected Results

- There are two rows, each keyed to its own local application and exact
  occurrence. The rows are in QSpec FR-322 canonical application/occurrence
  order.
- Each row's selected operation, receiver context and candidate values match
  only the captured table for that application. No field is paired by member
  spelling, source declaration order or graph traversal.
- Every candidate's `selected_operation` witness is exactly equal to its
  parent row's witness. The adverse copy is refused as
  `invalid_package`/`invalid-value` at
  `/checked_term_data/dispatch_tables/{i}/candidates/{j}/selected_operation`.
- Permuting declaration order leaves both row contents and canonical row
  order unchanged.
