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
signature, whose type does not match the operation's declared result, or that
is declared on an operation with no result type, refuses. The two mismatch
cases refuse whether or not a dispatched call reaches the operation; an
operation with no result cannot be called (FR-151). Scope: FR-302-AC-8, FR-576-AC-10.

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
5. Start from the `ok` body on `A.add`. Add operation `A.touch()` declaring no
   result type and declare `body noResult using V on M::A::touch() { true }`, with no clause calling
   `touch`. Compile the unit.
6. Take the unit of step 5 with `noResult` removed. Compile it and read `operation_bodies`.
7. Give `A.touch` the declared result `Boolean` and the body
   `body touchOk using V on M::A::touch() { true }`. Compile and read
   `operation_bodies`.

## Expected Results

Each of the four compiles in steps 1 to 4, and the compile in step 5, refuse at
S3 as `ill_typed`/`type-mismatch` located at the body, and produce no checked
package and no partial dispatch table. With the body
`body ok using V on M::A::add(amount) { amount }` the compile succeeds. In
step 6 `operation_bodies` holds a `no_body` row for `A.touch`. In step 7 it
holds a row for `A.touch` whose `body` references the body function node.
