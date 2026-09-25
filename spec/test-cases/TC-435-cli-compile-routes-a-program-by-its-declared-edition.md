---
id: TC-435
title: "CLI compile routes a program by its declared edition"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: verifies
---
# TC-435: CLI compile routes a program by its declared edition

## Description

Verify that CLI `compile` reads the program source's declared edition once and
sends each source down exactly one compiler: `1-draft` through the spine (S1 to
S4), `0-draft` through native compile. This catches a `1-draft` source compiled
natively, a `0-draft` source whose bytes change, an unknown edition that is
admitted, and a spine refusal reported under the wrong stage or code.

Scope: FR-027-AC-5 to FR-027-AC-8.

## Test Procedure

1. Run `tests/fixtures/spine-compile.native` (a record, an Integer function
   and a function with parameters) S1 to S4 through `emit_checked` and QSL's
   I2 reader (`qsl-package`).
2. Compile a native-compile/1 request selecting that fixture, no models and no
   clause bindings, and compare stdout with `command::spine::compile` over the
   same source and labels.
3. Compile the `0-draft` standalone fixtures and compare the bytes with the
   native static pipeline's (TC-105).
4. Compile a `0-draft` fixture relabelled `edition "7-draft"`.
5. Compile the `1-draft` fixture with the standalone request's models and
   clause bindings, then with its clause bindings only, then with neither.
6. Compile one `1-draft` source per spine stage refusal: a syntax error
   (source), a `predicate` (forms), an unresolved type name (assembly), a
   Boolean body under an Integer result (check) and a unit with nothing
   writable (emit); and map an omitting emission to its code.

Tag the tests `#[trace("TC-435", "FR-027-AC-5")]` (steps 1 and 2),
`#[trace("TC-435", "FR-027-AC-6")]` (step 3),
`#[trace("TC-435", "FR-027-AC-7")]` (steps 4 and 5) and
`#[trace("TC-435", "FR-027-AC-8")]` (step 6). The header reader's own tests
carry `#[trace("TC-435", "FR-027-AC-5")]`.

## Expected Results

- Step 1: nothing is omitted; the read is Verified and exports `Point`,
  `seven` and `px`.
- Step 2: exit 0; stdout equals the library bytes and names
  `quire.checked-package/v2`.
- Step 3: exit 0; the bytes and digest equal the native pipeline's.
- Step 4: `unknown_edition`, stage `profile`, exit 20, empty stdout; the
  message names `program.native` and `7-draft`, and the span is the literal.
- Step 5: `invalid-request`, exit 20, naming model sources, then clause
  bindings; the third compiles, exit 0.
- Step 6: stages `source`, `forms`, `assembly`, `check` and `emit` with codes
  `invalid_syntax` (20), `unsupported_construct` (21), `missing_declaration`
  (20), `ill_typed` (20) and `unsupported_projection` (21), each with empty
  stdout; an omitting emission is `unsupported_projection` at `emit`.

## Status

Passed locally (QSL-8).
