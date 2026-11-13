---
id: TC-1011
title: "Unit compile checks a body's arity and type against its operation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-576
    type: verifies
---
# TC-1011: Unit compile checks a body's arity and type against its operation

## Description

Verify that a body whose parameter count differs from its operation's
signature, or whose type does not match the operation's declared result,
refuses, whether or not a dispatched call reaches its operation. Scope: FR-302-AC-8, FR-576-AC-10.

## Test Procedure

1. In a `1-draft` unit selecting profile alias `V` and model alias `M` with
   object type `A` and operation `A.add(amount: Integer): Integer`, declare
   `body tooMany using V on M::A::add(amount, extra) { amount }` and call
   `add` in a precondition.
2. Compile the unit.
3. Replace the body with `body wrongType using V on M::A::add(amount) { true }`
   and compile again.
4. Remove the precondition call so no dispatched call reaches `A.add`, and
   compile once with `tooMany` and once with `wrongType`.
5. Add operation `A.touch()` declaring no result type and declare
   `body noResult using V on M::A::touch() { true }`. Compile with and without
   a dispatched call reaching `A.touch`.

## Expected Results

Each of the four compiles in steps 1 to 4, and both compiles in step 5, refuse at S3 as `ill_typed`/`type-mismatch` located at the
body, and produce no checked package and no partial dispatch table. With the body `body ok using V on M::A::add(amount) { amount }`
the compile succeeds.
