---
id: TC-803
title: "An inherited operation's frame binds only at its declaring type"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-305
    type: verifies
---
# TC-803: An inherited operation's frame binds only at its declaring type

## Description

Scope: FR-305-AC-3.

## Test Procedure

With `Sub` inheriting `op` from `Base` without redeclaring it:

1. check a frame binding keyed `OperationKey { Sub, op }`;
2. check one keyed `OperationKey { Base, op }`.

## Expected Results

1. Refused `missing_declaration`/`missing-name`, naming the key.
2. Checks.
