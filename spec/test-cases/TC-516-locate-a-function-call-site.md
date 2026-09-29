---
id: TC-516
title: "call_site names a function's parameters by declared identity, matching what replay accepts"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: verifies
---
# TC-516: call_site names a function's parameters by declared identity, matching what replay accepts

## Description

Verify `qsl_replay::call_site` over real complete-V1 source: it pairs a
named function's declared parameters with their node ids, and the pairing
is the one `qsl_replay::replay` actually accepts, proved by driving `replay`
with a request keyed by `call_site`'s own pairs rather than by re-running
the same node lookup `call_site` uses and comparing. It also refuses a name
resolving to no declared function, pairing it with the package it was
looked up in.

Scope: FR-121-AC-1 and FR-121-AC-2.

## Test Procedure

1. Compile a unit declaring one Boolean predicate `p(x: Int[0, 9]): Boolean
   { x < 5 }` through `call_site`. Assert `parameters` holds exactly one
   pair, whose `Identifier` is `x`. Build a `ReplayRequestWire` for the same
   source and function, keyed by that exact `(Identifier, WireNodeId)` pair
   (the `WireNodeId` as the `CanonicalAssignment::parameter`), and call
   `replay`.
2. Compile the TC-452 fixture unit `F` (`function f(x: Int[0, 9]): Integer`)
   through `call_site` naming a function the compiled package does not
   declare.

Tag the tests `#[trace("TC-516", "FR-121-AC-n")]`.

## Expected Results

- Step 1: `replay` does not refuse `UnknownParameter` or `UnboundParameter`
  -- the node id `call_site` names for `x` is the one `replay`'s own
  argument join accepts.
- Step 2: `call_site` refuses `CallSiteRefusal::UnknownFunction`, pairing
  the given `QualifiedName` with the compiled package's own `package_id`,
  never a bare `QualifiedName`.
