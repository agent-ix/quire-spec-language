---
id: TC-446
title: "Spine compile resolves imports against supplied libraries"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: verifies
---
# TC-446: Spine compile resolves imports against supplied libraries

## Description

Verify that spine `compile` resolves each `import` against the supplied
libraries, compiles each library from source, binds the import to the
recomputed `package_id`, types an imported function call from the
library's checked graph and emits it as a `dependency_reference`, and
refuses each ADR-015 D-1 and D-2 case with no package.

Scope: FR-099-AC-1 to FR-099-AC-6, FR-027-AC-10.

## Test Procedure

Library `test/geometry` version `1` is a complete-V1 unit, admitted as
authority `a`, identity `geometry`, revision (`git`, `1`), declaring
`function f using v(x: Int[0, 9]): Boolean pure { x < 5 }`. Let `d` be the
`package_id` of its spine compile. The importing unit, admitted as `a`,
`u`, (`git`, `1`), declares
`import "test/geometry" version "1" digest "<d>" as g;`.

1. Compile the importing unit, holding one function that references
   nothing of `g`, with `test/geometry` supplied; then with `test/geometry`
   and an unimported `test/other` supplied. Read the first package back
   through QSL's I2 read.
2. Spell the import's digest `sha256:<d>`, then in uppercase hex, then
   with 63 and with 65 hex characters. Then supply `test/geometry` from
   a source whose `f` is `x < 6`.
3. Supply no library; supply `test/geometry` at version `2`; supply two
   libraries as `test/geometry`; supply a library whose source has the
   unit's authority and identity; supply a library with an empty identity;
   supply `test/a` and `test/b`, each importing the other, and import
   `test/a`: the cycle's import digests are arbitrary, because the cycle
   refusal precedes any digest comparison; import `test/geometry` version
   `1` and then `test/a`, in that order, with `test/a` importing
   `test/geometry` version `2`;
   import `test/a` with `test/a` importing a `test/missing` no library
   supplies.
4. Build `LibraryName` from `test/geometry`, `a.b`, `L` and the empty
   string. Import `test/b` then `test/a`, and `test/a` then `test/b`.
5. Declare `function p using v(y: Int[0, 9]): Boolean pure { g::f(y) }`
   and compile; then replace the call with `g::f(true)`; then give
   `test/geometry` a record `R`, a function `mk` returning `R`, a function
   over `Set<R>`, a function over a tuple holding `R` and a function over
   `Reference<M::T>` for a model type `M::T`, and call each of them; then
   use `g::R` in a type position; then compile a unit whose only call is
   `g::f(3)`.
6. Supply `test/geometry` from the `x < 6` source, with the import's digest
   updated to that compile's `package_id`, and compile step 5's unit again.
7. Run CLI `compile` over a native-compile/1 request whose `1-draft`
   program is step 5's unit and whose `libraries` names `test/geometry`,
   version `1`, by file, source digest and the labels above; then the same
   request with a `0-draft` program.

Tag the tests `#[trace("FR-099-AC-n", "TC-446")]` with the AC each backs.

## Expected Results

- Step 1: the lock and the identity preimage each hold the one
  `DependencySelection` `{test/geometry, 1, d}`; the I2 read admits it;
  both compiles emit identical bytes.
- Step 2: `invalid-digest` at S1 at the digest string, four times; then
  `DependencyIdentityMismatch` (`stale_dependency`/`byte-digest-mismatch`)
  at the import, naming `test/geometry`, `d` and the recompiled
  `package_id`. No package.
- Step 3: `missing_import`/`missing-selection` at the import's identity
  string, stage `intake`; `stale_dependency`/`revision-mismatch` at the import;
  `invalid_package`/`conflicting-definition` from the dependency input
  naming both libraries, twice; `invalid_identifier`;
  `invalid_package`/`definition-cycle`, unwrapped, naming `test/a` and
  `test/b` at `test/b`'s import of `test/a`;
  `invalid_package`/`conflicting-definition`, unwrapped, naming both
  dependency paths to `test/geometry`; `CompileRefusal::Dependency` with
  path `[test/a]` carrying `missing_import`/`missing-selection` in
  `test/a`'s source. No package.
- Step 4: the three non-empty identities are admitted and the empty one
  refused; both import orders give the closure `test/a`, `test/b`.
- Step 5: `p` checks; its body is a `quire.op.function.call` application
  whose callee is `dependency_reference` `{package: d, node: f's node id}`,
  whose `result_type` is the Boolean type node, and whose node
  `dependencies` do not list `f`. `g::f(true)` refuses `ill_typed` at the
  argument, and each of the four calls and the use `g::R` refuses
  `ill_typed`/`operator-ineligible` at the use. The `g::f(3)` unit's
  package holds the `Int[0, 9]` type node typing the conversion of `3` to
  `f`'s parameter, as a local call's does.
- Step 6: `p`'s call node id and the package's `package_id` both differ
  from step 5's.
- Step 7: stdout is exactly the bytes spine `compile` returns for step 5's
  unit and library, exit 0; the `0-draft` request, and a `libraries` entry
  with an empty identity or version, refuse with `invalid-request`, exit
  20, empty stdout.

## Status

Steps 1 to 6 pass locally (QSL-255 part b): `qsl-replay`
`spine::dependency_tests`, and `qsl-cst` `an_import_digest_is_bare_lowercase_hex`
for step 2's digest spellings. Step 5's `g::f(3)` result was amended to
expect the `Int[0, 9]` conversion node (QSL-262). Step 7
(the CLI `libraries` member, FR-027-AC-10) passes locally:
`tests/it/compile_command.rs`
`a_complete_v1_request_supplies_its_libraries_to_the_spine`,
`malformed_libraries_refuse_as_invalid_request`,
`a_library_refusal_renders_over_the_library_source`,
`a_cycle_inside_the_libraries_renders_over_the_library_source` and
`sixty_four_libraries_exceed_the_file_limit`.
