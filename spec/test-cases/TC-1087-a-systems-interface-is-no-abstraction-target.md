---
id: TC-1087
title: "A systems interface is no abstraction target"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: verifies
---
# TC-1087: A systems interface is no abstraction target

## Description

Verify that a binding of a systems interface, as an object type or as the
declaring type of an operation, refuses as a declaration of the wrong kind.
Scope: FR-304-AC-9.

## Test Procedure

1. Check an object binding whose key names a systems interface of the
   domain package.
2. Check a frame binding whose declaring type is a systems interface.

## Expected Results

1. Refused `invalid_model_binding`/`malformed-declaration`, naming the key.
2. The same refusal, naming the key.
