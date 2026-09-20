---
id: TC-166
title: "The replay executor selects a function by typed QualifiedName, never by string"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: verifies
---
# TC-166: The replay executor selects a function by typed QualifiedName, never by string

## Description

Verify that the layer-6 `replay` facade's executor entry, reached through a
family's widened `evaluate` hook, selects the function to call by a typed
`QualifiedName` resolved against the recompiled package's declarations, that
no bare `&str` overload exists, and that an unresolvable name refuses rather
than falling back to a display-name string comparison. Scope: FR-062-AC-9,
FR-062-AC-10, FR-065-AC-6.

## Test Procedure

1. Inspect the layer-6 `replay` facade's executor entry point's signature
   for a parameter accepting a bare `&str` in place of `QualifiedName`, and
   attempt to compile a call site that passes a `&str` literal where the
   entry point expects its function-selection argument.
2. Build a checked package with two functions whose display names differ
   only by case (or another cosmetic variation) and construct a replay
   request naming one of them by its exact `QualifiedName`; invoke the
   executor entry.
3. Construct a replay request naming a `QualifiedName` that does not resolve
   against the recompiled package's declarations (a name with no matching
   declaration); invoke the executor entry.
4. Recompile the package from its digest-addressed source (as `replay`
   does) and confirm the executor entry resolves the request's
   `QualifiedName` against the recompiled package's own declaration table,
   not against any cached or pre-linked table from a prior compile.

## Expected Results

- Step 1: no overload or implicit conversion accepts a bare `&str` for
  function selection; the attempted call site fails to compile.
- Step 2: the executor resolves and calls exactly the named function,
  distinguishing it from the cosmetically similar one by its `QualifiedName`,
  not by a display-string comparison.
- Step 3: the request refuses with a typed cause naming the unresolved
  `QualifiedName`; it does not fall back to matching any function by a
  display-name string comparison.
- Step 4: resolution reads only the recompiled package's declaration table.
