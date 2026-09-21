---
id: TC-195
title: "An unregistered named backend yields a distinct unknown-backend marker"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
---
# TC-195: An unregistered named backend yields a distinct unknown-backend marker

## Description

Verify that a request naming a `BackendId` the registry does not hold
produces a distinct unknown-backend marker carrying that identity, and is
never reported the same way as an empty candidate set caused by a
registered backend not advertising the item's kind. Scope: FR-075-AC-3.

Catches an implementation that collapses both cases into a plain empty
candidate set (losing the caller's ability to distinguish "you named a
backend that doesn't exist" from "no backend advertises this kind"), and an
implementation that panics or errors instead of returning a typed marker.

## Test Procedure

1. Build a registry with one backend, Backend A, advertising
   `value-validity`.
2. Compute the candidate set for an item requiring `value-validity`,
   naming `BackendId` "backend-z", which is not registered.
3. Compute the candidate set for an item requiring `operation-contract`
   (which Backend A does not advertise), naming the registered
   `BackendId` "Backend A".
4. Inspect the two results' types/variants.

## Expected Results

Step 2 returns the unknown-backend marker carrying "backend-z", not a plain
empty candidate set. Step 3 returns a plain empty candidate set. Step 4
shows the two results are distinguishable by type or variant, not only by
an out-of-band string comparison of an error message.
