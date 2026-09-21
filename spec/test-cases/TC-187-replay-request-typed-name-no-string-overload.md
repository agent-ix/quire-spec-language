---
id: TC-187
title: "The replay request's function selection accepts only a typed QualifiedName, never a bare string"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: verifies
---
# TC-187: The replay request's function selection accepts only a typed QualifiedName, never a bare string

## Description

Verify that no public constructor or decoder of the replay request accepts
a bare `&str` or `String` in the function-selection position, and that no
implicit conversion from a string to `QualifiedName` exists. This is
narrower than TC-166 (which covers the *executor's* resolution behavior,
#243's scope): here the check is only that the request *type itself*
structurally forecloses a string-typed selection, independent of any
executor. A wrong implementation this test would catch: a request builder
convenience method `with_function_name(name: &str)` added "for ergonomics"
that internally wraps the string in a single-segment `QualifiedName`,
silently reintroducing string-based selection and making a
cosmetically-different function name resolve to the wrong declaration.
Scope: FR-071-AC-3.

## Test Procedure

1. Enumerate every public constructor and decoder entry point of the replay
   request type that sets the function-selection member.
2. For each entry point found in step 1, inspect its parameter type for the
   selection argument.
3. Attempt to compile a call site that passes a string literal (`&str`) or
   an owned `String` where the selection argument is expected.
4. Confirm a `QualifiedName` value constructed from a multi-segment path
   still round-trips through the same entry points unchanged.

## Expected Results

- Every entry point found in step 1 types its selection parameter as
  `QualifiedName`, with none typed `&str`, `String`, `Cow<str>`, or any type
  with an `impl From<&str>`/`impl From<String>` that the entry point
  accepts implicitly.
- The compile attempt in step 3 fails to compile.
- The multi-segment `QualifiedName` in step 4 round-trips with its full
  segment sequence intact.
