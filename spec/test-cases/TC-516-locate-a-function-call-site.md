---
id: TC-516
title: "call_site names a function's parameters and an operation's identities, matching what replay accepts"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: verifies
---
# TC-516: call_site names a function's parameters and an operation's identities, matching what replay accepts

## Description

Verify `qsl_replay::call_site` over real complete-V1 source: it pairs a
named function's declared parameters with their node ids, and the pairing
is the one `qsl_replay::replay` actually accepts, proved by driving `replay`
with a request keyed by `call_site`'s own pairs rather than by re-running
the same node lookup `call_site` uses and comparing -- including for a unit
compiled against a dependency input. It locates a named operation's anchor,
frame and state clause identities in a unit compiled against a domain
package, read back against the compiled graph's nodes by semantic form and
against occurrence keys built through the facade's own `OccurrenceKey`,
`Origin` and `Role`. It refuses a function or operation resolving to
nothing, pairing it with the package it was looked up in.

Scope: FR-121-AC-1 to FR-121-AC-5.

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
3. Compile FR-108's ConfigVersion unit through `call_site` with the
   `test/config-version` domain package document, selecting
   `Config::ConfigVersion::attemptUpdate`. Compare the answer with the
   compiled package's `package_id`, the graph's one `state`/`operation_anchor`
   and one `state`/`frame` node, and its one postcondition `state_clause`
   node; build the expected occurrence keys with
   `OccurrenceKey::new(node, Origin::new(Role::new(role), 0))` from the
   facade's re-exports.
4. Compile TC-444 step 7's unit importing `test/units` through `call_site`
   selecting `q`, once with a `DependencyInput` supplying `test/units` and
   once with none. From the first answer alone, key a `ReplayRequestWire`
   carrying `test/units` as its `dependencies` entry (its `package_id` and
   `q`'s one parameter), and call `replay` with the input `x = 3`.
5. Compile the step 3 unit through `call_site` selecting
   `Nope::ConfigVersion::attemptUpdate`, `Config::Nope::attemptUpdate` and
   `Config::ConfigVersion::nope`.

Tag the tests `#[trace("TC-516", "FR-121-AC-n")]`.

## Expected Results

- Step 1: `replay` does not refuse `UnknownParameter` or `UnboundParameter`
  -- the node id `call_site` names for `x` is the one `replay`'s own
  argument join accepts.
- Step 2: `call_site` refuses `CallSiteRefusal::UnknownFunction`, pairing
  the given `QualifiedName` with the compiled package's own `package_id`,
  never a bare `QualifiedName`.
- Step 3: `package_id`, `anchor` and `frame` equal the compiled ones;
  `frame_occurrence` is the frame node's `generated` occurrence at ordinal
  0; `clauses` holds exactly `VersionUnchanged`, at the postcondition node
  and its `claim` occurrence at ordinal 0. `ParentOrder` and `NoCycle` are
  absent.
- Step 4: with the dependency input, the parameter is `x` and the replay
  reproduces without a witness with the value `false`; without it,
  `call_site` refuses `CallSiteRefusal::Compile` naming `missing_import`.
- Step 5: each refuses `CallSiteRefusal::UnknownOperation`, pairing the
  given `OperationName` with the compiled package's own `package_id`.
