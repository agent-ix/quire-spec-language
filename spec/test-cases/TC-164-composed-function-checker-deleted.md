---
id: TC-164
title: "The composed function checker is deleted in the same change as the S3 function checker"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: verifies
---
# TC-164: The composed function checker is deleted in the same change as the S3 function checker

## Description

Verify that after this requirement's implementation, no composed-linker
(SEAM-2) code path checks a function-declaration or function-application
form: the composed checker's pre-migration function-check entry points are
absent from the compiled crate's symbols, and, where the composed checker
module is retained for its other `Value` forms, its input form-kind enum no
longer carries a function-declaration or function-application variant and
its dispatch `match` carries no `_`/catch-all arm, so a function form is not
a value that type can hold at all -- a compile-time fact, never established
by inspecting reachability. Scope: FR-065-AC-5.

## Test Procedure

1. Search the compiled crate's public and crate-internal symbols for the
   composed linker's pre-migration function-declaration and
   function-application checking entry points (by name, against the
   pre-migration source).
2. Inspect the composed checker's dispatch `match` (if the module is
   retained for its other `Value` forms) for a `_` or catch-all arm.
3. In a fixture built from the post-migration composed checker module,
   reintroduce a function-declaration variant into its input form-kind enum
   without adding a matching arm to the dispatch `match`; attempt to
   compile the fixture.
4. In the same fixture, keep the reintroduced variant and add a
   `_ => refuse(...)` arm to the dispatch `match` so it compiles again.

## Expected Results

- Step 1: the composed checker's pre-migration function-check entry
  point(s) are absent from the compiled crate's symbols.
- Step 2: the dispatch `match` carries no `_` or catch-all arm.
- Step 3: the fixture fails to compile with `E0004` at the composed
  checker's dispatch `match`.
- Step 4: the fixture now compiles, but this does not satisfy FR-065-AC-5:
  the `_ => refuse(...)` arm is itself a catch-all arm, which this
  requirement's own rule forbids on this `match`; the fixture is a
  demonstration of the forbidden shape, not a passing case.
