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
form: the composed checker's function-check entry point(s) are either absent
from the compiled crate's symbols, or, where the composed checker module is
retained for its other `Value` forms, its function-form match arms are gone
and calling it with a function form fails to compile or is unreachable.
Scope: FR-065-AC-5.

## Test Procedure

1. Search the compiled crate's public and crate-internal symbols for the
   composed linker's pre-migration function-declaration and
   function-application checking entry points (by name, against the
   pre-migration source).
2. If the composed checker module still exists (retained for literals,
   operators, `let`, `if`, records or collections), inspect its dispatch
   `match` for a function-declaration or function-application arm, and
   attempt to construct a call path that reaches the composed checker with a
   function-declaration or function-application form.
3. Construct a source file containing one function declaration and one
   call to it, alongside at least one other `Value` form (for example, a
   `let` binding) that the composed checker still handles. Check the file
   and trace which checker code path each form's checking actually executed
   through.

## Expected Results

- Step 1: the composed checker's pre-migration function-check entry
  point(s) are absent from the compiled crate's symbols.
- Step 2: either the composed checker module itself is absent, or its
  dispatch `match` has no function-declaration or function-application arm;
  no call path reaches it with a function form (a compile failure, or, if
  the module is retained, unreachable code confirmed by the missing arm).
- Step 3: the function declaration and its call are checked only through the
  checked-family contract's `check`/`package` hooks; the `let` binding is
  checked through the (unchanged) composed checker. A change that lands the
  S3 function checker while leaving the composed checker's function-form
  arms reachable, even if this trace does not currently exercise them, does
  not satisfy FR-065-AC-5: step 2's inspection, not this trace, is the
  authority for that criterion.
