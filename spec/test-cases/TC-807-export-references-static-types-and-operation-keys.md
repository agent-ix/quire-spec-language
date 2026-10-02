---
id: TC-807
title: "The export references the receiver's static type and the operation key"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-307
    type: verifies
---
# TC-807: The export references the receiver's static type and the operation key

## Description

Scope: FR-307-AC-4, FR-307-AC-5.

## Test Procedure

1. A clause reads `f`, declared on `Base`, through a receiver of static
   type `Sub`. Request its item with only `Base` bound, then with `Sub`
   bound.
2. Request the frame item and a postcondition item of `attemptUpdate` with
   its frame bound, then without it.

## Expected Results

1. With only `Base` bound the item refuses naming `Sub`; with `Sub` bound
   it returns `Sub`'s binding.
2. With the frame bound both items return the `FrameBinding`; without it
   both refuse naming `OperationKey { ConfigVersion, attemptUpdate }`.
