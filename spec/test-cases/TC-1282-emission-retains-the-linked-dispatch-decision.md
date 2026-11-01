---
id: TC-1282
title: "Emission retains the linked dispatch decision"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-494
    type: verifies
---
# TC-1282: Emission retains the linked dispatch decision

## Description

Verify that QSL emits the exact producer-linked dispatch decision into the
checked package for one dispatched call, including candidate order, selection
keys and effective-precondition identities, while leaving semantic graph
identity unchanged. Scope: FR-494-AC-1, AC-3 and AC-4.

## Test Procedure

1. Compile a unit with a dispatched call to an operation whose linked table
   contains three candidates. Give each candidate a distinct FR-150 selection
   key and checked body function; give two candidates distinct effective
   precondition functions, with one inherited from a redefinition, and give
   the third no effective-precondition body. Choose names and graph traversal
   order that disagree with the linked table's candidate sequence.
2. Capture the S3 `DispatchTable` and `DispatchCandidate` values supplied to
   checked-package emission, including the table's exact selected-operation
   witness, candidate order, selection keys and checked function identities.
3. Emit the checked package and read its `checked_term_data.dispatch_tables`
   through the producer-facing representation. Independently read the
   semantic graph nodes and identities. For every emitted candidate, compare
   its `selected_operation` witness with both the captured `DispatchTable`
   witness, and compare the candidate's remaining fields with its captured
   selection key, body identity and effective-precondition identity.
4. Repeat with a deliberately reversed candidate fixture while holding names
   and graph traversal order fixed.

## Expected Results

- The first package has exactly one dispatch row for the call's local
  application node and exact occurrence. It records the checked selected
  operation and receiver context.
- Every candidate has a `selected_operation` field exactly equal to the
  parent table's captured operation witness. Removing that field or replacing
  it with another valid operation witness fails the field-for-field producer
  comparison.
- Its candidates match the captured producer sequence element for element;
  every row retains its captured selection key, operation-body identity and
  effective-precondition body identity, including the null value for the
  candidate with no effective-precondition body. The inherited precondition
  uses the checked effective function identity rather than the authored
  clause identity.
- Reversing the captured producer sequence reverses the emitted candidate
  sequence, regardless of names or graph traversal order. Names and graph
  order do not replace the producer's candidate sequence or identities.
- The semantic graph nodes and node identities are equal across packages;
  only the producer-captured dispatch data represents the table.
