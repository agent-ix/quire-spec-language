---
id: TC-206
title: "The registry module lint gate finds no static, OnceLock or thread_local"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-080
    type: verifies
---
# TC-206: The registry module lint gate finds no static, OnceLock or thread_local

## Description

Verify that a lint gate scans the registry module specifically (not the
whole crate, and not skipping it) for `static`, `OnceLock` and
`thread_local!` items, and fails when one is present. Scope: FR-080-AC-3.

Catches a lint rule scoped to the wrong module (so ambient state introduced
in the registry module itself is never scanned), and a lint rule that
detects `static` but not `OnceLock` or `thread_local!` — each is a
distinct way to introduce hidden global state, and a rule catching only one
of the three would pass an implementation using either of the other two.

## Test Procedure

1. Run the lint gate against the unmodified registry module and confirm it
   passes.
2. Temporarily add a `static REGISTRY: ...` item to a copy of the registry
   module (not committed) and re-run the gate.
3. Repeat step 2, instead adding a `static REGISTRY: OnceLock<...> = ...`
   item.
4. Repeat step 2, instead adding a `thread_local! { static REGISTRY: ... }`
   block.
5. Confirm the gate's configured scope includes the registry module's file
   path or module path explicitly.

## Expected Results

Step 1 passes. Steps 2, 3 and 4 each fail the gate, each naming the
specific injected item and its location. Step 5 confirms the scope is not
accidentally excluding the registry module.
