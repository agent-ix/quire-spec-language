---
id: TC-196
title: "A duplicate backend identity registration is refused and the original stands"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
---
# TC-196: A duplicate backend identity registration is refused and the original stands

## Description

Verify that registering a `BackendId` already held by the registry is
refused with `invalid_capability`/`duplicate-backend`, naming the identity,
and that the original registration's advertised kinds are unaffected.
Scope: FR-075-AC-4.

Catches an implementation that silently overwrites the first registration
with the second (losing the original advertisement set), and an
implementation that merges the two registrations' advertised-kind sets
instead of refusing the second outright.

## Test Procedure

1. Register Backend A advertising `value-validity` only.
2. Attempt to register a second descriptor under the same `BackendId`
   "Backend A", this time advertising `operation-contract` only.
3. Inspect the result of the second registration attempt.
4. Compute the candidate set for an item requiring `operation-contract`
   with no named backend.
5. Compute the candidate set for an item requiring `value-validity` with no
   named backend.

## Expected Results

Step 3 is a refusal with `invalid_capability`/`duplicate-backend`, naming
"Backend A". Step 4 returns an empty candidate set (the second
registration's `operation-contract` advertisement never took effect). Step
5 returns Backend A (the original registration is unchanged).
