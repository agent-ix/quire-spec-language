---
id: FR-100
title: "Run a named function of a 1-draft program through the spine"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-014
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: traces_to
  - target: "ix://agent-ix/quire-specification/FR-301"
    type: depends_on
---
# FR-100: Run a named function of a 1-draft program through the spine

## Description

When an author invokes quire-spec run with a native-run/1 request file, the
run command shall route the program source by the edition its header
declares, as compile does
([FR-027](FR-027-export-compiled-native-package.md)). A `0-draft` source
runs its selected clause through native run
([FR-026](FR-026-run-standalone-native-workflow.md)), unchanged. A `1-draft`
source compiles through the spine (S1 to S4) and the command calls the one
function the request names through S6a (`CheckedPackage::call`), with the
arguments the request supplies. This is ADR-011 §5's spine `run`.

The spine run entry is `qsl_replay::spine::run`. It compiles the source with
`qsl_replay::spine::compile`, selects the function by name lookup in the
compiled package, binds the arguments, calls the function and returns the
call's outcome. The root crate reaches it through `qsl_replay` and names
`qsl-eval` in none of its dependency tables (TC-390).

## Inputs

A native-run/1 request (FR-026's closed envelope and shared limits). For a
`1-draft` program the request carries:

- `program`: the program's source selection (file, `sha256:` source digest,
  the four FR-001 labels, `document` and `formal_revision`), with no
  `clauses` and no `extraction`.
- `models`: zero or more `semantic-ir/2.0.0` domain package documents, each
  read under its source digest and handed to spine `compile` as FR-056's
  package input, exactly as FR-027 does for compile.
- `libraries`: zero or more `{identity, version, source}` objects, read and
  handed to spine `compile` as its dependency input, exactly as FR-027 does
  for compile (FR-099, ADR-015 D-1).
- `call`: a closed object naming the call.
  - `function`: the called function's name, a string.
  - `arguments`: an array of `{parameter, value}` objects, one per
    parameter, in any order. `parameter` is the parameter's declared name.
    `value` is a JSON integer in the signed 64-bit range: the canonical
    integer assignment FR-098 replays, read by the same rule. An integer
    type's parameter takes the integer. A `Boolean` parameter takes `0`
    (`false`) or `1` (`true`).
  - `work_units`: optional, a non-negative integer. It is the S6a
    `quire.value.accounting/v1` `work_units` limit of the call. An omitted
    `work_units` is 1,000,000. The call's other nine accounting counters are
    each `u64::MAX`.

A `0-draft` request carries FR-026's members and no `call` or `libraries`.

## Outputs

For a call that reaches S6a, stdout is one `spine-run-result/1` JSON
document, newline-terminated, and nothing else. Its members are:

- `format`: `"spine-run-result/1"`.
- `request_digest`: the request file's `sha256:` byte digest.
- `package_id`: the compiled package's `package_id`, lowercase hex, equal
  to the one `qsl_replay::spine::compile` computes for the same source and
  inputs.
- `source`: the program source as native-run-result/1 renders one: its four
  FR-001 labels, its `sha256:` source digest and its authored path.
- `function`: the called function's name.
- `outcome`: exactly one of
  - `{"kind": "completed", "value": V}`, where `V` is
    `{"kind": "boolean", "value": true|false}` or
    `{"kind": "integer", "decimal": "<ASCII decimal>"}`, the integer written
    exactly, with a leading `-` when negative and no other sign, leading
    zero or exponent (FR-038's integer spelling, at any magnitude);
  - `{"kind": "refused", "code": "<catalog code>"}`;
  - `{"kind": "undefined", "reason": "<undefined reason>"}`;
  - `{"kind": "incomplete", "limit": "<accounting counter>"}`.

On FR-301's six-code contract the outcome's exit status is: `completed` 0,
whatever value it completes; `refused` its catalog code's exit status (20
or 21); `undefined` 20; `incomplete` 22.

Every refusal before S6a writes nothing to stdout. It writes FR-026's
native-run-result/1 command-error envelope to stderr, carrying the request
digest, a stage and a catalog code, and exits with that code's exit status:

| Refusal | Stage | Code | Exit |
|---------|-------|------|------|
| A declared edition other than `0-draft` or `1-draft` | `profile` | `unknown_edition` | 20 |
| A `1-draft` request carrying `selection`, `snapshots`, `invocations`, `package`, a set `validation_work` or `expression_steps`, `clauses`, `extraction`, or a model in a format other than `semantic-ir/2.0.0`; or carrying no `call` | `request` | `invalid-request` | 20 |
| A `0-draft` request carrying `call` or `libraries` | `request` | `invalid-request` | 20 |
| A `call` that is not the closed object above, or an argument `value` that is not a JSON integer in the signed 64-bit range | `request` | `invalid-request` | 20 |
| A spine compile refusal | the refusing spine stage | that stage's cause code | that code's |
| A `function` naming no function of the compiled package | `call` | `missing_declaration` | 20 |
| A function whose declared result is neither `Boolean` nor an integer type | `call` | `unsupported_construct` | 21 |
| An argument naming no parameter, a parameter named twice, or a parameter with no argument | `call` | `invalid_runtime_input` | 20 |
| A value that is not of its parameter's declared type (`WrongValueKind`) | `call` | `invalid_runtime_input` | 20 |

Broken pipes end quietly; other write and output-serialization failures
exit 30.

## Behavior

- The run command shall read the program source's declared edition once,
  from its header, after the source's digest check and before any model or
  library source is read, with FR-027's edition reader. Each program source
  goes to exactly one runner.
- If the program declares `1-draft`, then the run command shall refuse a
  request carrying a member the table above names before it reads any model
  or library file.
- The run command shall hand the `1-draft` source, its domain packages, its
  dependency input, the default spine stage limits, the `call` and its
  accounting limits to `qsl_replay::spine::run`, and render its result.
- `qsl_replay::spine::run` shall compile the source with
  `qsl_replay::spine::compile` and carry a compile refusal unchanged, with
  its stage and cause code.
- `qsl_replay::spine::run` shall resolve `function` by name lookup in the
  compiled package's declarations, as `qsl_replay::replay` does (FR-098,
  OQ-5). A name with more than one segment resolves to no function.
- `qsl_replay::spine::run` shall refuse a function whose declared result is
  neither `Boolean` nor an integer type before any call or charge.
- `qsl_replay::spine::run` shall join each argument to the parameter of the
  same declared name, and order the values by declared parameter position,
  whatever order they arrive in.
- `qsl_replay::spine::run` shall convert each argument's value to a value of
  its parameter's declared type by FR-098's rule: an integer for an integer
  type, and `0` or `1` for `Boolean`. Any other value for a `Boolean`
  parameter, and any value for a parameter of another kind, refuses
  `WrongValueKind` before the call. A value outside the parameter's declared
  domain (`12` for `Int[0, 9]`) refuses `WrongValueKind` at S6a admission.
  Each names the parameter's position.
- `qsl_replay::spine::run` shall call the function through
  `CheckedPackage::call` under the given accounting limits and return the
  compiled package's `package_id` with the call's outcome in its ADR-013
  O-16 category: a completed value, a refusal with its catalog code, an
  undefined result with its reason (kernel `Undefined` or a family
  `FamilyResult::Undefined`), or an incomplete result with the exhausted
  counter. A family `FamilyResult::Refused` is a refusal with its catalog
  code.
- The types `qsl_replay::spine::run` takes and returns shall be
  `qsl_replay`, `qsl_foundation`, `qsl_semantics` or `quire_exact` types.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-100-AC-1 | A native-run/1 request selecting `tests/fixtures/spine-compile.native` (`edition "1-draft"`) with `call` `{"function": "seven", "arguments": []}` exits 0 and writes one `spine-run-result/1` document whose `outcome` is `{"kind": "completed", "value": {"kind": "integer", "decimal": "7"}}`, whose `package_id` equals the one `qsl_replay::spine::compile` computes for the same source, and whose `source` names the request's four labels and source digest. | Test (TC-450) |
| FR-100-AC-2 | A `0-draft` native-run/1 request writes the same native-run-result/1 bytes and exit status it wrote before this change. A program declaring `edition "7-draft"` refuses `unknown_edition` at stage `profile`, exit 20, empty stdout, naming the file and the edition. | Test (TC-450) |
| FR-100-AC-3 | A `1-draft` request carrying each member the Outputs table names refuses `invalid-request`, exit 20, empty stdout, whatever state its model files are in; a `1-draft` request with no `call` refuses the same way; a `0-draft` request carrying `call` or `libraries` refuses the same way. A `1-draft` request whose `libraries` supplies an imported library and whose `models` supplies a domain package the program selects runs, exit 0. | Test (TC-450) |
| FR-100-AC-4 | For a unit declaring `lt(a: Int[0, 9], b: Int[0, 9]): Boolean { a < b }`, `flag(b: Boolean): Boolean { b }`, `id(x: Int[0, 9]): Int[0, 9] { x }` and `px(p: Point): Digit`: `lt` with `b = 3` given before `a = 5` completes `false`, exit 0; `flag(1)` completes `true`; `id(4)` completes integer `"4"`. `flag(2)`, `id(12)` and `px(1)` each refuse `invalid_runtime_input` at stage `call`, exit 20, empty stdout, naming parameter position 0. An argument naming `y`, `x` bound twice, and no argument each refuse `invalid_runtime_input`, naming the parameter. A `value` of `true`, `"7"`, `1.5` or `9223372036854775808` refuses `invalid-request`, exit 20. | Test (TC-451) |
| FR-100-AC-5 | `function` `nope` and `module.seven` each refuse `missing_declaration` at stage `call`, exit 20, empty stdout, naming the name. A function whose declared result is a record refuses `unsupported_construct` at stage `call`, exit 21, before any call. A `1-draft` source that a spine stage refuses exits with that stage's cause code and reports the stage (FR-027-AC-8), with empty stdout. | Test (TC-451) |
| FR-100-AC-6 | `seven` with `work_units` 0 writes outcome `{"kind": "incomplete", "limit": "work_units"}`, exit 22. A call whose evaluation is undefined writes outcome `undefined` with its reason, exit 20. | Test (TC-451) |
| FR-100-AC-7 | `qsl_replay::spine::run` called directly over each AC-1, AC-4, AC-5 and AC-6 input returns the same `package_id`, outcome category, value, code and parameter the CLI renders. The root crate names `qsl-eval` in no dependency table (TC-390). | Test (TC-452) |

## Dependencies

- [FR-026](FR-026-run-standalone-native-workflow.md): native-run/1, its
  intake limits and its command-error envelope; the `0-draft` route.
- [FR-027](FR-027-export-compiled-native-package.md): the edition reader,
  domain package and `libraries` intake, and spine compile refusals.
- [FR-098](FR-098-execute-a-replay-request.md): name lookup and the canonical
  integer argument rule.
- [FR-099](FR-099-compile-against-supplied-libraries.md): the dependency
  input.
- [FR-038](FR-038-encode-exact-protocol-numbers.md): the integer decimal
  spelling.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §5 and OQ-1: spine `run` calls a named checked function.
- QSpec FR-301: the six exit codes.

## Status

Specified under QSL-271 (A05-1). Not implemented.
