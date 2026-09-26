---
id: TC-450
title: "CLI run routes a program by its declared edition and calls a 1-draft function"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: verifies
---
# TC-450: CLI run routes a program by its declared edition and calls a 1-draft function

## Description

Verify that CLI `run` reads the program source's declared edition once and
sends each source down exactly one runner: `1-draft` through
`qsl_replay::spine::run`, `0-draft` through native run. This catches a
`1-draft` source run natively, a `0-draft` result that changes, an unknown
edition that is admitted, and a `1-draft` request whose native-only members
are silently ignored.

Scope: FR-100-AC-1 to FR-100-AC-3.

## Test Procedure

1. Run a native-run/1 request selecting `tests/fixtures/spine-compile.native`,
   no models, and `call` `{"function": "seven", "arguments": []}`. Compile the
   same source with `qsl_replay::spine::compile` under the same labels, and
   take the SHA-256 digest of the request file's bytes.
2. Run the `0-draft` standalone requests of TC-103 and TC-104
   (`tests/it/standalone.rs`) and compare stdout and exit status with the
   results those tests assert.
3. Run the `1-draft` fixture relabelled `edition "7-draft"`.
4. Run the `1-draft` fixture request once with each of `selection`,
   `snapshots`, `invocations`, `package`, `limits.validation_work`,
   `limits.expression_steps`, program `clauses`, program `extraction`, and a
   `native-rule-model/1` model whose file holds malformed bytes added; then
   with `call` removed; then with `work_units` `18446744073709551616`, `-1`
   and `1.5`.
5. Run a `0-draft` standalone request with a `call` added; then with a
   `libraries` entry added.
6. Run a `1-draft` program that imports `test/geometry` and declares
   `model M` by the `sha256-jcs` digest of
   `tests/fixtures/spine-model.semantic-ir.json`, and calls a function using
   both, with `libraries` supplying `test/geometry` and `models` supplying the
   document.

Tag the tests `#[trace("TC-450", "FR-100-AC-1")]` (step 1),
`#[trace("TC-450", "FR-100-AC-2")]` (steps 2 and 3) and
`#[trace("TC-450", "FR-100-AC-3")]` (steps 4 to 6).

## Expected Results

- Step 1: exit 0 and empty stderr; stdout is one newline-terminated document
  whose `format` is `spine-run-result/1`, whose `request_digest` is the
  request file's `sha256:` digest, whose `package_id` equals the compiled
  package's, whose `source` names the request's four labels, source digest
  and authored path, whose `function` is `seven`, and whose `outcome` is
  `{"kind": "completed", "value": {"kind": "integer", "decimal": "7"}}`.
- Step 2: stdout and exit status equal the TC-103 and TC-104 assertions.
- Step 3: `unknown_edition`, stage `profile`, exit 20, empty stdout; the
  `message` names the program file and `7-draft`.
- Step 4: each request refuses `invalid-request` at stage `request`, exit 20,
  empty stdout, and no model file is read.
- Step 5: each refuses `invalid-request` at stage `request`, exit 20, empty
  stdout.
- Step 6: exit 0 and a `completed` outcome.

## Status

Passed locally under QSL-271.
