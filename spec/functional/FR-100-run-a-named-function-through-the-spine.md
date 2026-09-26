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
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-038
    type: references
  - target: "ix://agent-ix/quire-specification/FR-301"
    type: depends_on
---
# FR-100: Run a named function of a 1-draft program through the spine

## Description

When an author invokes quire-spec run with a native-run/1 request file, the
run command shall route the program source by the edition its header
declares, as compile does
([FR-027](FR-027-export-compiled-native-package.md)). A `0-draft` source,
or one declaring no edition, runs its selected clause through native run
([FR-026](FR-026-run-standalone-native-workflow.md)). A `1-draft` source
compiles through the spine (S1 to S4) and the command calls the one
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
  the four [FR-001](FR-001-read-exact-source.md) labels, `document` and
  `formal_revision`), with no `clauses` and no `extraction`.
- `models`: zero or more `semantic-ir/2.0.0` domain package documents, each
  read under its source digest and handed to spine `compile` as FR-056's
  package input, exactly as FR-027 does for compile.
- `libraries`: zero or more `{identity, version, source}` objects, read and
  handed to spine `compile` as its dependency input, exactly as FR-027 does
  for compile ([FR-099](FR-099-compile-against-supplied-libraries.md),
  ADR-015 D-1).
- `call`: a closed object naming the call.
  - `function`: the called function's name, a string of one or more
    segments separated by `.`, the separator source paths use. Each segment
    is an identifier: an ASCII letter or `_`, then ASCII letters, digits or
    `_`.
  - `arguments`: an array of `{parameter, value}` objects, one per
    parameter, in any order. `parameter` is the parameter's declared name.
    `value` is a JSON integer in the signed 64-bit range: the canonical
    integer assignment FR-098 replays, read by the same rule. An integer
    type's parameter takes the integer. A `Boolean` parameter takes `0`
    (`false`) or `1` (`true`).
  - `work_units`: optional, a JSON integer from 0 to 18446744073709551615
    (`u64::MAX`). It is the S6a `quire.value.accounting/v1` `work_units`
    limit of the call. An omitted `work_units` is 1,000,000. The call's
    other nine accounting counters are each `u64::MAX`.

A `0-draft` request carries FR-026's members and no `call` or `libraries`.

## Outputs

### Outcome document

For a call that reaches S6a, stdout is one `spine-run-result/1` JSON
document, newline-terminated, and nothing else. Its members are:

- `format`: `"spine-run-result/1"`.
- `request_digest`: the `sha256:` digest of the request file's bytes.
- `package_id`: the compiled package's `package_id`, lowercase hex, equal
  to the one `qsl_replay::spine::compile` computes for the same source and
  inputs.
- `source`: the program source as native-run-result/1 renders one: its four
  FR-001 labels, its `sha256:` source digest and its authored path.
- `function`: the `function` string of the request.
- `outcome`: exactly one of
  - `{"kind": "completed", "value": V}`, where `V` is
    `{"kind": "boolean", "value": true|false}` or
    `{"kind": "integer", "decimal": "<ASCII decimal>"}`, the integer written
    exactly, with a leading `-` when negative and no other sign, leading
    zero or exponent ([FR-038](FR-038-encode-exact-protocol-numbers.md)'s
    integer spelling, at any magnitude);
  - `{"kind": "refused", "code": "<catalog code>", "cause": "<cause>"}` for
    a kernel refusal, with a `details` member
    `{"violation": "below-minimum"|"above-maximum"}` for
    `Refusal::CardinalityOutOfBound` only, and
    `{"kind": "refused", "code": "<catalog code>"}` for a family refusal;
  - `{"kind": "undefined", "reason": "<reason>"}`;
  - `{"kind": "incomplete", "limit": "<accounting counter>"}`, the
    exhausted counter's `quire.value.accounting/v1` member name.

The command renders every S6a outcome through one total mapping from its
ADR-013 O-16 category to this document and an FR-301 exit status:

| S6a outcome | `outcome` | Exit |
|-------------|-----------|------|
| `Outcome::Completed(v)` | `completed`, `value` from `v` | 0, whatever value it completes |
| `Outcome::Refused(r)`, kernel refusal `r` other than `CheckedInvariant` | `refused`, `code` `invalid_runtime_input`, `cause` from the table below | 20 |
| `Outcome::Refused(Refusal::CheckedInvariant)` | `refused`, `code` `runtime_invariant`, `cause` `checked_invariant` | 30 |
| `FamilyResult::Refused(c)` | `refused`, `code` `c`'s catalog code, no `cause` | that code's FR-301 exit status (`Code::exit_code`) |
| `Outcome::Undefined(u)`, kernel reason `u` | `undefined`, `reason` from the table below | 20 |
| `FamilyResult::Undefined(u)` | `undefined`, `reason` the catalog's `UndefinedReason` spelling (`precondition-false`, `absent-key`) | 20 |
| `Outcome::Incomplete(i)` | `incomplete`, `limit` `i`'s counter | 22 |

Each kernel refusal's `cause` is the kernel's own `Refusal::code()`
spelling (`quire-exact`, snake case), the one spelling source for kernel
refusals. Each kernel undefined reason is spelled in kebab case; the kernel
defines no spelling method for `Undefined`.

| Kernel value | Spelling |
|--------------|----------|
| `Refusal::InexactDecimal` | `inexact_decimal` |
| `Refusal::DecimalOutOfDomain` | `decimal_out_of_domain` |
| `Refusal::DivisionPairOutOfDomain` | `division_pair_out_of_domain` |
| `Refusal::ModuloOutOfDomain` | `modulo_out_of_domain` |
| `Refusal::TextLengthOutOfDomain` | `text_length_out_of_domain` |
| `Refusal::IntegerOutOfDomain` | `integer_out_of_domain` |
| `Refusal::RationalOutOfDomain` | `rational_out_of_domain` |
| `Refusal::IeeeNotExact` | `ieee_not_exact` |
| `Refusal::IeeeNanPayloadNotRepresentable` | `ieee_nan_payload_not_representable` |
| `Refusal::IeeeRationalOutOfDomain` | `ieee_rational_out_of_domain` |
| `Refusal::ForeignReference` | `foreign_reference` |
| `Refusal::CardinalityOutOfBound` | `cardinality_out_of_bound`, with `details.violation` its `BoundViolation::as_str` tag (`below-minimum` or `above-maximum`) |
| `Refusal::CheckedInvariant` | `checked_invariant` |
| `Undefined::DivisionByZero` | `division-by-zero` |
| `Undefined::IeeeNotFinite` | `ieee-not-finite` |
| `Undefined::EmptyReduction` | `empty-reduction` |
| `Undefined::NoneValue` | `none-value` |

A kernel refusal other than `CheckedInvariant` is a defined result the
call's value domain does not admit, so it is refused runtime input
(`invalid_runtime_input`, 20). `CheckedInvariant` is a checked-program
invariant failing during evaluation, which is unreachable for an admitted
program (`quire-exact` `Refusal::CheckedInvariant`). It is an internal
failure (ADR-013 O-16's internal-failure row,
`runtime_invariant`/`established-invariant-broken`), and exits 30, the tool
failure status of QSpec FR-301's exit contract (`0` completed without
violation, `10` logical violation, `20` invalid/refused input, `21`
unsupported, `22` incomplete and `30` tool failure).

### Refusals before S6a

Every refusal before S6a writes nothing to stdout. It writes FR-026's
native-run-result/1 command-error envelope to stderr, carrying the request
digest, a stage, a catalog code, a `message` and a `details` object, and
exits with that code's exit status:

| Refusal | Stage | Code | `details` | Exit |
|---------|-------|------|-----------|------|
| A declared edition other than `0-draft` or `1-draft` | `profile` | `unknown_edition` | as FR-027 | 20 |
| A `1-draft` request carrying `selection`, `snapshots`, `invocations`, `package`, a set `validation_work` or `expression_steps`, `clauses`, `extraction`, or a model in a format other than `semantic-ir/2.0.0`; or carrying no `call` | `request` | `invalid-request` | as FR-026 | 20 |
| A `0-draft` request carrying `call` or `libraries` | `request` | `invalid-request` | as FR-026 | 20 |
| A `call` that is not the closed object above; an argument `value` that is not a JSON integer in the signed 64-bit range; a `work_units` that is not a JSON integer from 0 to `u64::MAX` | `request` | `invalid-request` | as FR-026 | 20 |
| A spine compile refusal | the refusing spine stage | that stage's cause code | as FR-027 | that code's |
| A `function` that is empty, holds an empty segment or a segment that is not an identifier, has more than one segment, or names no function of the compiled package | `call` | `missing_declaration` | `{"function": "<the function string>"}` | 20 |
| A function whose declared result is neither `Boolean` nor an integer type | `call` | `unsupported_construct` | `{"function": "<the function string>"}` | 21 |
| An argument naming no parameter, a parameter named twice, or a parameter with no argument | `call` | `invalid_runtime_input` | `{"parameter": "<the parameter name>"}` | 20 |
| A value that is not of its parameter's declared type (`WrongValueKind`) | `call` | `invalid_runtime_input` | `{"position": <the parameter's zero-based position>}` | 20 |

An unsupported result type exits 21, where FR-098's replay refuses a
non-`Boolean` selection as `NotAPredicate` (`invalid_runtime_input`, 20).
The difference is deliberate: replay requires a predicate, so a
non-`Boolean` selection is a bad request; run calls any function, and a
result kind the outcome document cannot express is an unsupported construct,
not bad input.

Broken pipes end quietly; other write and output-serialization failures
exit 30.

## Behavior

- The run command shall read the program source's declared edition once,
  from its header, after the source's digest check and before any model or
  library source is read, with FR-027's edition reader. Each program source
  goes to exactly one runner.
- If the program declares an edition other than `0-draft` or `1-draft`,
  then the run command shall refuse with `unknown_edition` at the edition
  literal, naming the file and the edition.
- If the program declares `0-draft` or no edition, then the run command shall run the program through FR-026's native run.
- If the program declares `0-draft` or no edition and the request carries `call` or `libraries`, then the run command shall refuse the request with `invalid-request`.
- If the program declares `1-draft`, then the run command shall refuse a
  request carrying a member the refusal table names, or carrying no `call`,
  with `invalid-request` before it reads any model or library file.
- If a `call` member, an argument `value` or `work_units` is outside the
  shape Inputs states, then the run command shall refuse with
  `invalid-request` at stage `request`.
- The run command shall hand the `1-draft` source, its domain packages, its
  dependency input, the default spine stage limits, the `call` and its
  accounting limits to `qsl_replay::spine::run`, and render its result by
  the outcome mapping above.
- `qsl_replay::spine::run` shall compile the source with
  `qsl_replay::spine::compile` and carry a compile refusal unchanged, with
  its stage and cause code.
- If `function` is empty, holds an empty segment or a non-identifier
  segment, or has more than one segment, then the spine run entry shall
  refuse with `missing_declaration` at stage `call`.
- `qsl_replay::spine::run` shall resolve a one-segment `function` by name
  lookup in the compiled package's declarations, as `qsl_replay::replay`
  does (FR-098, OQ-5).
- If the name resolves to no function, then `qsl_replay::spine::run` shall
  refuse with `missing_declaration` at stage `call`.
- If the function's declared result is neither `Boolean` nor an integer
  type, then `qsl_replay::spine::run` shall refuse with
  `unsupported_construct` at stage `call` before any call or charge.
- `qsl_replay::spine::run` shall join each argument to the parameter of the
  same declared name, and order the values by declared parameter position,
  whatever order they arrive in.
- If an argument names no parameter, names a parameter already bound, or
  leaves a parameter unbound, then `qsl_replay::spine::run` shall refuse
  with `invalid_runtime_input` at stage `call`, naming the parameter.
- `qsl_replay::spine::run` shall convert each argument's value to a value of
  its parameter's declared type by FR-098's rule: an integer for an integer
  type, and `0` or `1` for `Boolean`.
- If a `Boolean` parameter's value is other than `0` or `1`, or the
  parameter is of a kind neither `Boolean` nor an integer type, then
  `qsl_replay::spine::run` shall refuse `WrongValueKind` before the call,
  naming the parameter's position.
- If a value is outside its parameter's declared domain (`12` for
  `Int[0, 9]`), then `qsl_replay::spine::run` shall refuse `WrongValueKind`
  at S6a admission, naming the parameter's position.
- `qsl_replay::spine::run` shall call the function through
  `CheckedPackage::call` under the given accounting limits and return the
  compiled package's `package_id` with the call's outcome, converted to the
  `outcome` member's category, code, cause, reason and counter by the
  mapping and spellings above.
- The `qsl_replay` crate shall name no `qsl_eval` path in a public item of
  `qsl_replay::spine` or in a re-export. The types `qsl_replay::spine::run`
  takes and returns are `qsl_replay`, `qsl_foundation`, `qsl_semantics` or
  `quire_exact` types.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-100-AC-1 | A native-run/1 request selecting `tests/fixtures/spine-compile.native` (`edition "1-draft"`) with `call` `{"function": "seven", "arguments": []}` exits 0, with empty stderr, and writes one newline-terminated document whose `format` is `spine-run-result/1`, whose `request_digest` is the `sha256:` digest of the request file's bytes, whose `package_id` equals the one `qsl_replay::spine::compile` computes for the same source, whose `source` names the request's four labels, source digest and authored path, whose `function` is `seven`, and whose `outcome` is `{"kind": "completed", "value": {"kind": "integer", "decimal": "7"}}`. | Test (TC-450) |
| FR-100-AC-2 | A `0-draft` native-run/1 request writes the stdout bytes and exit status TC-103 and TC-104 fix for it. A program declaring `edition "7-draft"` refuses `unknown_edition` at stage `profile`, exit 20, empty stdout, with a `message` naming the file and the edition. | Test (TC-450) |
| FR-100-AC-3 | A `1-draft` request carrying each member the refusal table names refuses `invalid-request` at stage `request`, exit 20, empty stdout, whatever state its model files are in; a `1-draft` request with no `call` refuses the same way; a `0-draft` request carrying `call` or `libraries` refuses the same way. A `work_units` of `18446744073709551616`, `-1` or `1.5` refuses the same way. A `1-draft` request whose `libraries` supplies an imported library and whose `models` supplies a domain package the program selects runs, exit 0. | Test (TC-450) |
| FR-100-AC-4 | For a unit declaring `lt(a: Int[0, 9], b: Int[0, 9]): Boolean { a < b }`, `flag(b: Boolean): Boolean { b }`, `id(x: Int[0, 9]): Int[0, 9] { x }` and `px(p: Point): Digit`: `lt` with `b = 3` given before `a = 5` completes `false`, exit 0; `flag(1)` completes `true`; `id(4)` completes integer `"4"`. `flag(2)`, `id(12)` and `px(1)` each refuse `invalid_runtime_input` at stage `call`, exit 20, empty stdout, with `details` `{"position": 0}`. An argument naming `y`, `x` bound twice, and no argument each refuse `invalid_runtime_input` at stage `call`, with `details` `{"parameter": "y"}`, `{"parameter": "x"}` and `{"parameter": "x"}`. A `value` of `true`, `"7"`, `1.5` or `9223372036854775808` refuses `invalid-request` at stage `request`, exit 20. | Test (TC-451) |
| FR-100-AC-5 | `function` `nope`, `module.seven`, `""`, `seven.`, and `7x` each refuse `missing_declaration` at stage `call`, exit 20, empty stdout, with `details` `{"function": <that string>}`. A function whose declared result is a record refuses `unsupported_construct` at stage `call`, exit 21, before any call. A `1-draft` source with a syntax error refuses at stage `source` (`invalid_syntax`), and one declaring `inv(x: Int[0, 9]): Boolean { 1 / x > 0 }` refuses at stage `check` with `ill_typed` (integer `/` with no `Rational` expected type); each exits 20 with empty stdout (FR-027-AC-8). | Test (TC-451) |
| FR-100-AC-6 | `seven` with `work_units` 0 writes outcome `{"kind": "incomplete", "limit": "work_units"}`, exit 22. | Test (TC-451) |
| FR-100-AC-7 | `qsl_replay::spine::run` called directly over each AC-1, AC-4, AC-5 and AC-6 input returns the same `package_id`, outcome category, value, code, reason, counter, and parameter name or position the CLI renders. The root crate names `qsl-eval` in no dependency table (TC-390). | Test (TC-452) |
| FR-100-AC-8 | No public item of `qsl_replay::spine`, and no `qsl_replay` re-export, names a `qsl_eval` path; a `pub use` of a `qsl_eval` item from `qsl_replay`, or a `qsl_eval` type in `spine::run`'s signature, fails the check. | Test (TC-452) |
| FR-100-AC-9 | The outcome mapping converts a constructed `Outcome::Completed` of each value kind, `Outcome::Refused` of each of the thirteen kernel refusals, `Outcome::Undefined` of each of the four kernel reasons, `Outcome::Incomplete`, `FamilyResult::Refused` and `FamilyResult::Undefined` of each family reason into the `outcome` member and exit status the mapping tables state, with each spelling as tabled. | Test (TC-452) |

## Dependencies

- [FR-026](FR-026-run-standalone-native-workflow.md): native-run/1, its
  intake limits and its command-error envelope; the `0-draft` route.
- [FR-027](FR-027-export-compiled-native-package.md): the edition reader,
  domain package and `libraries` intake, and spine compile refusals.
- [FR-098](FR-098-execute-a-replay-request.md): name lookup and the canonical
  integer argument rule.
- [FR-099](FR-099-compile-against-supplied-libraries.md): the dependency
  input.
- [FR-001](FR-001-read-exact-source.md): the four source labels.
- [FR-038](FR-038-encode-exact-protocol-numbers.md): the integer decimal
  spelling.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §5 and OQ-1: spine `run` calls a named checked function.
- ADR-013 O-16: the outcome categories.
- QSpec FR-301: the six exit codes.

## Status

Specified under QSL-271 (A05-1). Not implemented. `quire-exact`'s
`Refusal::code()` gains arms for the nine variants it returns `None` for
today (`InexactDecimal`, `DecimalOutOfDomain`, `DivisionPairOutOfDomain`,
`ModuloOutOfDomain`, `TextLengthOutOfDomain`, `IntegerOutOfDomain`,
`RationalOutOfDomain`, `IeeeNotExact`, `CheckedInvariant`), with the
spellings tabled above, so that it is total and the one spelling source.
