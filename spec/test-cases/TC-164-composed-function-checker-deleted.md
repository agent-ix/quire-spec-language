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

## Status

**Unbacked (PR #262 review, finding F1).** This test case has zero tests
in the delivered code. `Typer`'s declaration-typing pass and
`ValueFunctionFamily::call` remain the only checker either function form
has -- the composed checker's pre-migration entry points this test case
verifies are absent were not removed by #214, so steps 1-4 above have no
implementation to run against yet. This is a rescoping decision recorded
against #262 (see [FR-065](../functional/FR-065-migrate-function-application-to-checked-family.md)'s
own Status section, AC-5 row), not an oversight: FR-065-AC-5's target
design (move the checking into `ValueFunctionFamily::check` and delete it
from `Typer`) is filed as its own ticket, QSL-148, rather than attempted
inside #214.

This is a distinct deferral from #214's *other* still-present legacy
surface, `src/cli.rs`'s `lower` command and `src/package.rs`'s
`NativePackageRef` (the SEAM-1 native-v1 CLI producer path). That surface
is owned by ADR-011 §7.3's M-6a row, not by this test case: its stated
owner is "QSL-8 (this repo's #240) with M-4, before #216", and it is
[#240](https://github.com/agent-ix/quire-spec-language/issues/240)'s
requirement, not QSL-148's. TC-164 (M-6e, the composed checker) and the
M-6a CLI producer cutover are two separate rows of the same ADR-011 §7.3
table, deferred to two separate tickets (QSL-148 and #240
respectively) -- neither is satisfied by, or blocks, the other.
