---
id: TC-451
title: "Spine run binds arguments by name and maps each outcome and refusal to its exit code"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: verifies
---
# TC-451: Spine run binds arguments by name and maps each outcome and refusal to its exit code

## Description

Verify that CLI `run` of a `1-draft` program joins each argument to its
parameter by declared name, converts it by FR-098's canonical integer rule,
renders the S6a outcome in `spine-run-result/1`, and refuses each pre-call
case on stderr with its stage, code, `details` and FR-301 exit status.

Scope: FR-100-AC-4 to FR-100-AC-6.

## Test Procedure

The unit declares `edition "1-draft"`, `type Digit = Int[0, 9]`,
`record Point { x: Digit; y: Digit; }`,
`lt(a: Digit, b: Digit): Boolean { a < b }`,
`flag(b: Boolean): Boolean { b }`, `id(x: Digit): Digit { x }`,
`px(p: Point): Digit { p.x }`, `origin(): Point` (a function whose declared
result is a record) and `seven(): Digit { 7 }`.

1. Run `lt` with `b = 3` given before `a = 5`; `flag(1)`; `id(4)`.
2. Run `flag(2)`, `id(12)` and `px(1)`.
3. Run `id` with an argument naming `y`; with `x` bound twice; with no
   argument.
4. Run `id` with `value` `true`, `"7"`, `1.5` and `9223372036854775808`.
5. Run `function` `nope`, `module.seven`, `""`, `seven.` and `7x`; run
   `origin`.
6. Run a `1-draft` source with a syntax error; run a `1-draft` source
   declaring `inv(x: Digit): Boolean { 1 / x > 0 }`.
7. Run `seven` with `work_units` 0.

Tag the tests `#[trace("TC-451", "FR-100-AC-4")]` (steps 1 to 4),
`#[trace("TC-451", "FR-100-AC-5")]` (steps 5 and 6) and
`#[trace("TC-451", "FR-100-AC-6")]` (step 7).

## Expected Results

- Step 1: exit 0 each; outcomes `completed` with
  `{"kind": "boolean", "value": false}`,
  `{"kind": "boolean", "value": true}` and
  `{"kind": "integer", "decimal": "4"}`.
- Step 2: each refuses `invalid_runtime_input` at stage `call`, exit 20,
  empty stdout, `details` `{"position": 0}`; `flag(2)` and `px(1)` refuse
  before the call and `id(12)` at S6a admission.
- Step 3: each refuses `invalid_runtime_input` at stage `call`, exit 20,
  empty stdout, `details` `{"parameter": "y"}`, `{"parameter": "x"}` and
  `{"parameter": "x"}` in turn.
- Step 4: each refuses `invalid-request` at stage `request`, exit 20, empty
  stdout.
- Step 5: each name refuses `missing_declaration` at stage `call`, exit 20,
  empty stdout, `details` `{"function": <that string>}`; `origin` refuses
  `unsupported_construct` at stage `call`, exit 21, empty stdout,
  `details` `{"function": "origin"}`, before any call.
- Step 6: the syntax error refuses `invalid_syntax` at stage `source`; `inv`
  refuses `ill_typed` at stage `check` (integer `/` with no `Rational`
  expected type); each exits 20 with empty stdout.
- Step 7: outcome `{"kind": "incomplete", "limit": "work_units"}`, exit 22.

## Status

Passed locally under QSL-271. FND-012: `origin` is a reserved keyword
(`qsl-cst`'s token table), so the fixture's record-result function is
named `corner`, not `origin`, in `tests/fixtures/spine-run.native`; every
other fixture name above is exact. Nothing about the scenario changes --
still a function whose declared result is a record, still refusing
`unsupported_construct` before any call.
