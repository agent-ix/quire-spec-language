---
id: TC-802
title: "A frame binding checks without a naming clause and relates to the frame and anchor when one exists"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-305
    type: verifies
---
# TC-802: A frame binding checks without a naming clause and relates to the frame and anchor when one exists

## Description

Scope: FR-305-AC-1, FR-305-AC-2.

## Test Procedure

1. Compile a ConfigVersion unit with no clause naming `attemptUpdate` and a
   relation binding `attemptUpdate`'s frame.
2. Compile a unit whose clause names `attemptUpdate`, with the same binding.

## Expected Results

1. The unit compiles; the checked relation holds the `FrameBinding` under
   `OperationKey { ConfigVersion, attemptUpdate }`.
2. The binding is related to exactly the `state`/`frame` and
   `state`/`operation_anchor` node keys of `attemptUpdate`'s
   `CheckedOperationFrame`.
