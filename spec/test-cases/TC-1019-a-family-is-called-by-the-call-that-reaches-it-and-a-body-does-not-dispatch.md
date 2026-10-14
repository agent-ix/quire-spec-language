---
id: TC-1019
title: "A family is called by the call that reaches it, and a body does not dispatch"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: verifies
---
# TC-1019: A family is called by the call that reaches it, and a body does not dispatch

## Description

Verify that a family refuses the compile only when a call reaches it, and that a dispatched call in an operation body is no input to linking. Scope: FR-302-AC-9.

Catches a compile that links by member name alone and refuses over an unrelated same-named family.

## Test Procedure

1. Declare `A.size` with a body and an unrelated type `Other` declaring `size` with no body. Compile a unit whose only `size` call is in a clause on `A`.
2. Add a clause on `Other` that calls `size`, and compile again.
3. Declare a body for `A.other` that calls `self.size()`, and call `other` in a clause.

## Expected Results

1. The compile succeeds.
2. Refused at S3 as `ambiguous_dispatch`/`no-applicable`, naming `Other.size`.
3. Refused at S3 as `ill_typed`/`operator-ineligible`.
