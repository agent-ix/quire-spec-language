---
id: TC-435
title: "`command` compile routes a program by its declared edition"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: verifies
---
# TC-435: `command` compile routes a program by its declared edition

## Description

Verify that `command`'s `compile`, called in process, reads the program
source's declared edition once, compiles a `1-draft` source through the spine
(S1 to S4) and refuses every other declared edition, `0-draft` included, with
`unknown_edition`. This catches a `0-draft` or unknown edition that is
admitted, and a spine refusal reported under the wrong stage or code.

Scope: FR-027-AC-5, FR-027-AC-7 and FR-027-AC-8.

## Test Procedure

1. Run `tests/fixtures/spine-compile.native` (a record, an Integer function
   and a function with parameters) S1 to S4 through `emit_checked` and QSL's
   I2 reader (`qsl-package`).
2. Compile a native-compile/1 request selecting that fixture, no models and no
   clause bindings, and compare the outcome's package bytes with the FR-278 composition over the
   same source and labels. Compile it again with a different `document` and
   `formal_revision`, then with a blank `document`.
3. Compile the `1-draft` fixture relabelled `edition "0-draft"`, the same
   request with a `libraries` entry and `refinement_internals` added, and
   the fixture relabelled `edition "7-draft"`.
4. Compile a source with no header, and one declaring `language "ix:other"`.
5. Compile the `1-draft` fixture with the standalone request's models, whose
   files hold malformed bytes, and clause bindings; then with its clause
   bindings only; then with neither.
6. Compile one `1-draft` source per spine stage refusal: a syntax error
   (source), a `temporal` clause such as
   `temporal Due using v over (view: M::Node) clock "c" on origin { always[0,1] holds(true) }`,
   whose leading token has no dispatch entry (forms; FR-102-AC-4), an
   unresolved type name (assembly), a
   Boolean body under an Integer result (check) and a unit with nothing
   writable (emit); and map an omitting emission to its code.

Tag the tests `#[trace("TC-435", "FR-027-AC-5")]` (steps 1 and 2),
`#[trace("TC-435", "FR-027-AC-7")]` (steps 3 to 5) and
`#[trace("TC-435", "FR-027-AC-8")]` (step 6). The header reader's own tests
carry `#[trace("TC-435", "FR-027-AC-5")]`.

## Expected Results

- Step 1: nothing is omitted; the read is Verified and exports `Point`,
  `seven` and `px`.
- Step 2: exit 0; the package bytes equal the library bytes and names
  `quire.checked-package/v2`; the relabelled request writes identical bytes;
  the blank `document` refuses at the request stage, exit 20.
- Step 3: each refuses `unknown_edition`, stage `profile`, exit 20, no package
  bytes; the message names `program.native` and the declared edition
  (`0-draft`, `0-draft`, `7-draft`), and the span is the literal.
- Step 4: `invalid_syntax` and `unknown_language`, each at stage `source`,
  exit 20, no package bytes.
- Step 5: `invalid-request`, exit 20, naming model sources, then clause
  bindings; the third compiles, exit 0.
- Step 6: stages `source`, `forms`, `assembly`, `check` and `emit` with codes
  `invalid_syntax` (20), `unsupported_construct` (21), `missing_declaration`
  (20), `ill_typed` (20) and `unsupported_projection` (21), each with no package bytes. The message is readable text, not a Debug rendering. The forms,
  assembly and check spans are the declaration, `Nope` and `true`; the emit
  refusal has no span. An omitting emission is `unsupported_projection` at
  `emit`.
