---
id: TC-1011
title: "Unit compile checks a body's arity and type against its operation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: verifies
---
# TC-1011: Unit compile checks a body's arity and type against its operation

## Description

Verify that a body whose parameter count differs from its operation's
signature, or whose type does not match the operation's declared result,
refuses when dispatch linking first reads the body. Scope: FR-302-AC-8.

## Test Procedure

1. In a `1-draft` unit selecting profile alias `V` and model alias `M` with
   object type `A` and operation `A.add(amount: Integer): Integer`, declare
   `body tooMany using V on M::A::add(amount, extra) { amount }` and call
   `add` in a precondition.
2. Compile the unit.
3. Replace the body with `body wrongType using V on M::A::add(amount) { true }`
   and compile again.

## Expected Results

Each compile refuses at S3 and produces no checked package and no partial
dispatch table. With the body `body ok using V on M::A::add(amount) { amount }`
the compile succeeds.
