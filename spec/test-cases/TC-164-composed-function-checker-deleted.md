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

**Still zero tests against this test case's own procedure (QSL-148,
2026-09-21) -- superseded, not satisfied.** QSL-148 moves the checking this
test case's target design names: `Typer::call` is deleted
(`qsl-semantics/src/check/check.rs`), and `check/mod.rs`'s inline per-declaration
typing/definedness pass is replaced by a call to
`check::family::check_declaration_body`
(`qsl-semantics/src/check/family.rs`) -- the composed checker's pre-migration
function-check entry points genuinely are absent now. But this test case's
own procedure (a symbol-search step and an `E0004` compile-fail fixture) is
not implemented, on the explicit
[testing-policy ruling](https://linear.app/agent-ix/issue/QSL-148#comment-2a4d2837)
delivered during QSL-148 (Peter, 2026-09-22, relayed by the QSL team
lead): test what the family check accepts and refuses, never a symbol's
presence/absence or a match arm's shape. `TC-377`'s behavioral tests of
`check_declaration_body` are genuine coverage of the checking-decision
half of this migration, but they do not demonstrate this test case's own
enum-shape/symbol-absence procedure, and `Expression::Call` remains
present in `Expression` regardless -- so FR-065-AC-5 itself stays
unbacked, not backed by substitution; see FR-065's own Status section,
AC-5 row, and `TC-377` for the tests themselves and the full reasoning.

This test case is kept, as written, as a record of AC-5's originally
specified verification method; it is not a currently-planned piece of
work. A future change that wants the symbol-absence/`E0004` guarantee this
procedure describes can still implement it against today's code, which
would satisfy it -- nothing above makes the procedure itself wrong, only
non-mandatory under the current testing policy.

**Prior history (PR #262 review, finding F1; superseded by the paragraph
above).** Before QSL-148, this test case had zero tests because `Typer`'s
declaration-typing pass and `Typer::call` were still the only checker
either function form had -- the composed checker's pre-migration entry
points this test case verifies were absent had not yet been removed. That
was a rescoping decision recorded against #262 (see
[FR-065](../functional/FR-065-migrate-function-application-to-checked-family.md)'s
own Status section, AC-5 row), not an oversight: FR-065-AC-5's target
design (move the checking into `ValueFunctionFamily::check`-adjacent family
code and delete it from `Typer`) was filed as its own ticket, QSL-148,
rather than attempted inside #214.

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
