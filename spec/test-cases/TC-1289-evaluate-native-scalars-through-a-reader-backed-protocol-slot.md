---
id: TC-1289
title: "Evaluate native String, UUID and Timestamp values through protocol slots"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: verifies
---
# TC-1289: Evaluate native String, UUID and Timestamp values through protocol slots

## Description

Verify that the evaluator reaches native `String`, `UUID` and `Timestamp`
values from a reader-backed checked package and preserves their existing exact
equality semantics. This exercises FR-390-AC-24.

## Test Procedure

Compile a fixture whose protocol `finish` expression reads a record with
`label: String`, `token: UUID` and `at: Timestamp` fields, plus a nested record
containing the same three fields. Emit the checked package, admit its bytes
through the checked-package reader, and select the `finish.constraint` slot.
Supply equal and unequal canonical values for each field, then repeat each
comparison through the nested record. Evaluate the same slot from the
in-process checked package and from the reader-backed view. Also compile
cross-kind equality (`String` against `UUID`) and ordering (`<` on two
Timestamps) expressions in the fixture's source-language check lane.

## Expected Results

String equality compares Unicode scalar sequences exactly; UUID equality
compares canonical 128-bit payloads; Timestamp equality compares signed
128-bit nanosecond counts. Equal pairs return true and unequal pairs return
false, including when nested in records. Each reader-backed evaluation has the
same result and ordered charge log as its in-process evaluation. Cross-kind
equality and ordering of these native kinds are refused during type checking,
before an evaluator view or meter charge is produced.
