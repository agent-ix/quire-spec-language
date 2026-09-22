---
id: TC-251
title: "Qualified-name resolution is confined to the check stage (R-06)"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: verifies
---
# TC-251: Qualified-name resolution is confined to the check stage (R-06)

## Description

Verify that no function in the crate resolves a `QualifiedName` (or a bare
string) to a node id or declaration from outside the check stage, with the
one documented exception being the `replay` facade's E9 lookup (a separate
module this requirement does not build). This is R-06's adverse test,
applied to this requirement's own `QualifiedName` type and the checker's
resolution function. Scope: FR-088-AC-5.

## Test Procedure

1. Search the whole compiled crate's call graph for every call site of the
   checker's name-resolution function (the function that takes a
   `QualifiedName` and returns a node id or declaration).
2. Confirm every call site found lies inside the check stage module, except
   for exactly one exception: the `replay` module's own E9 lookup.
3. Confirm no other module (a post-S3 stage, a backend-facing module such as
   `route` or `command`, or a CLI entry point) contains a function whose
   signature accepts a `QualifiedName` or `&str` and returns a node id or
   declaration.
4. Adverse test: attempt to resolve a name after the check stage has
   completed, from `package` or `value::expression`, using only
   `CheckedPackage`'s public accessors, and confirm no such accessor
   exists — only node-id-keyed accessors are exposed. This step's claim is
   about `CheckedPackage`'s own accessor surface; it carves out `replay`'s
   own E9 lookup (step 2's documented exception, R-06), which resolves a
   `QualifiedName` against the recompiled package's declarations through
   its own separate mechanism, not through a `CheckedPackage` accessor, and
   is out of this step's scope.
5. Confirm `QualifiedName` itself is used only as a declared component of
   an identity preimage (for example inside a `DeclarationKey`'s preimage),
   never compared or hashed as if it were an identity in its own right,
   where a node id or digest is available instead.

## Expected Results

- Step 2: exactly one exception (E9's `replay` lookup); any other call site
  outside the check stage fails this step and names the location.
- Step 3: no post-check name-resolution function exists anywhere in the
  crate.
- Step 4: `CheckedPackage`'s public accessors are node-id-keyed only; a
  name-keyed accessor fails this step. `replay`'s separate E9 lookup is out
  of this step's scope and does not fail it.
- Step 5: `QualifiedName` never substitutes for an identity comparison.
