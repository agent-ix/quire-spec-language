---
id: TC-450
title: "`command` run routes a program by its declared edition and calls a 1-draft function"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: verifies
---
# TC-450: `command` run routes a program by its declared edition and calls a 1-draft function

## Description

Verify that `command`'s `run`, called in process, reads the program source's
declared edition once, runs a `1-draft` source through
`qsl_replay::spine::run` and refuses every other declared edition, `0-draft`
included, with `unknown_edition`. This catches a `0-draft` or unknown edition
that is admitted, and a `1-draft` request whose native-only members are
silently ignored.

Scope: FR-100-AC-1 to FR-100-AC-3.

## Test Procedure

1. Run a native-run/1 request selecting `tests/fixtures/spine-compile.native`,
   no models, and `call` `{"function": "seven", "arguments": []}`. Compile the
   same source with the FR-278 composition under the same labels.
2. Run the `1-draft` fixture relabelled `edition "0-draft"`, then the same
   request with a `call` and a `libraries` entry, then a source with no
   header.
3. Run the `1-draft` fixture relabelled `edition "7-draft"`.
4. Run the `1-draft` fixture request once with each of `selection`,
   `snapshots`, `invocations`, `limits.validation_work`,
   `limits.expression_steps`, program `clauses`, program `extraction`, and a
   `native-rule-model/1` model whose file holds malformed bytes added; then
   with `call` removed; then with a `call.accounting` member `work_units` of
   `18446744073709551616`, `-1`, `1.5` and `null`, with an `accounting`
   member `depth`, with an `accounting` of `5`, and with a top-level
   a legacy `work_units` member directly in `call`. Run the program
   `extraction` request twice more: through the command built with no
   features and through the command built with `--all-features`.
5. Run a `1-draft` program that imports `test/geometry` and declares
   `model M` by the identity of
   `tests/fixtures/spine-model.semantic-ir.json`, and calls a function using
   both, with `libraries` supplying `test/geometry` and `models` supplying the
   document.

Tag the tests `#[trace("TC-450", "FR-100-AC-1")]` (step 1),
`#[trace("TC-450", "FR-100-AC-2")]` (steps 2 and 3) and
`#[trace("TC-450", "FR-100-AC-3")]` (steps 4 and 5).

## Expected Results

- Step 1: exit 0 and no refusal envelope; the result document is one document
  whose `format` is `spine-run-result/1`, whose `source` names the request's two labels
  and authored path, whose `function` is `seven`, and whose `outcome` is
  `{"kind": "completed", "value": {"kind": "integer", "decimal": "7"}}`.
- Step 2: the two `0-draft` requests refuse `unknown_edition`, stage
  `profile`, exit 20, no result document, with a `message` naming the
  program file and `0-draft`; the source with no header refuses
  `invalid_syntax` at stage `source`, exit 20, no result document.
- Step 3: `unknown_edition`, stage `profile`, exit 20, no result document; the
  `message` names the program file and `7-draft`.
- Step 4: each ordinary function-run request refuses `invalid-request` at
  stage `request`, exit 20, no result document, and no model file is read.
  The `extraction` request refuses `invalid-request` naming `extraction` in the
  no-feature build, and `extraction-requires-run` in the feature-enabled build.
- Step 5: exit 0 and a `completed` outcome.
