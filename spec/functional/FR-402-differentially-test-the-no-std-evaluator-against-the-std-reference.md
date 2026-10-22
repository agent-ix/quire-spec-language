---
id: FR-402
title: "Differentially test the no_std evaluator against the std reference over one evaluator corpus"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/AD-001
    type: traces_to
---
# FR-402: Differentially test the no_std evaluator against the std reference over one evaluator corpus

## Description

[AD-001](../assurance/AD-001-shared-evaluation-core-interface.md) keeps QSL's
std evaluator as the semantic reference while a second, `no_std` + `alloc`
implementation of the [FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md)
interface is built behind it (IR-590). The two implementations must agree on
outcome, refusal, location, losses and charge sequence for shared inputs, and
the reference may be switched out only once the second implementation passes
the shared differential suite (IR-593). This requirement defines that suite:
which inputs it runs, the record it compares, how an input bundle is encoded
and loaded, what the loader refuses, where the harness runs and what counts
as a failure.

The suite runs one corpus of evaluator cases. A case is one checked
application: a QSL source unit compiled through the layer-6 facade, a
function of that unit, completed arguments and limits. Each case runs through
both implementations with a fresh meter, and the two compared records must
be equal. QSL owns the case encoding, the loader and the compared record,
and exposes them through the facade module `qsl_replay::corpus`, so the code
generator (CG) can export its generated-closure cases in this encoding
without defining an encoding of its own and without reaching any QSL module
other than the facade ([FR-060](FR-060-check-qsl-api-surface-boundary.md)
T12-A, "T12-A: the evaluator corpus producer").

## Inputs

- A corpus bundle: a directory holding one file, `cases.jsonl`.
- The QSL-owned corpus: the checked-in bundle
  `tests/fixtures/evaluator-corpus/cases.jsonl` and the generated cases.
- Optionally, an external bundle named by `EVALUATOR_CORPUS_DIR`, such as the
  one CG's `make evaluator-corpus` writes.

## Outputs

- From the loader: every case of a bundle, or one `CorpusRefusal` for the
  whole bundle.
- From the facade's reference entry: one complete case record whose
  `expected` member is the std reference's compared record.
- From the harness: a pass, or one report line per divergent case and a
  failing exit.

## Behavior

### The case record

A bundle's `cases.jsonl` SHALL be UTF-8 JSON Lines: each line is one case
record, written as its RFC 8785 (JCS) canonical JSON followed by one `\n`,
and the file holds no other bytes. Records SHALL appear in strictly ascending
UTF-8 byte order of `case_id`.

A case record SHALL be a JSON object with exactly these members:

| Member | Content |
| --- | --- |
| `format` | the string `quire.evaluator-case/v1` |
| `case_id` | a string of 1 to 128 bytes drawn from `a`-`z`, `0`-`9`, `.`, `_`, `-` and `/`, unique in the bundle |
| `source` | an object with exactly `path` (the display path, a string) and `text` (the unit's source, a string) |
| `package_id` | the `package_id` that `qsl_replay::compile_package` returns for `source`, in the digest-record shape the `quire.checked-package/v2` bytes declare it |
| `function` | the declared name of a function of the compiled unit that is callable by name |
| `arguments` | an array of FR-071 `WitnessValue`s in their wire shape, one per declared parameter, in declared parameter order |
| `object_environment` | `null` |
| `limits` | an object mapping `quire.value.accounting/v1` limit names to non-negative integers; a limit with no entry runs at its published default |
| `generated_bodies` | an array of declared function names of the compiled unit, strictly ascending and unique, naming the functions the exporter lowered to generated bodies; empty for a QSL-owned case |
| `expected` | `null`, or the compared record the std reference produced for this case |

A case's checked input is its `source` compiled through
`qsl_replay::compile_package` under the published stage limits, with no
domain packages and no dependencies, so the checked closure the loader
admits is the one the exporter admitted, and the `package_id` binds the two.
A case runs over `ObjectEnvironment::with_unresolved_references`, as a replay
request without `object_environment` does (FR-098). Arguments become values
of their parameters' declared types by FR-098's `WitnessValue` conversion.
A converted argument that admission then refuses (arity, kind, domain) is not
a loader refusal: it is the case's result, `CallFailure::Input`, and it is
compared like any other result.

`generated_bodies` records which callables the exporter lowered; the harness
runs every callable interpreted, with no registered body. FR-262-AC-4
requires a generated-body chain to produce the reference result and charge
sequence, so a record's `expected` holds for the interpreted run as well.

### The compared record

The compared record of one run SHALL be a JSON object with exactly the
members `result`, `charges` and `consumed`, built from the run's
`Result<Evaluation, CallFailure>` and the meter's charge log:

- `result` is either `{"kind": "evaluation", "outcome": ..., "location":
  ..., "losses": [...]}` or `{"kind": "call_failure", "arm": ..., "record":
  ...}`.
- `outcome` names the arm of `Evaluation.outcome`: `completed` with `value`
  (the result as an FR-071 `WitnessValue` of the function's declared result
  type); `incomplete` with `limit_kind`; `refused` and `family_refused`, each
  with `record`; `undefined` and `family_undefined`, each with `record`.
- Each `record` is the refusal or undefined record the evaluator's own
  record functions produce for that arm (`Evaluation::refusal_record` and the
  `CallFailure` arm's record), in its `quire.native.diagnostics/v1` shape:
  catalog code, cause and fields.
- `location` is `Evaluation.location` and each `losses` entry is one
  `LocatedLoss`, both in their `quire.native.diagnostics/v1` locus and loss
  shapes, in evaluation order.
- `arm` names the `CallFailure` variant in snake case.
- `charges` is the ordered list of charges the meter recorded, each
  `{"point": <charge-point name>, "amount": <integer>}`, using the
  `quire.value.accounting/v1` charge-point names (such as `function.call`).
  A denied charge is not recorded, as FR-262 states.
- `consumed` maps each `quire.value.accounting/v1` counter name to the
  meter's final value.

Two compared records SHALL be equal exactly when their JCS bytes are equal.
A difference SHALL be reported as the RFC 6901 JSON pointer of the first
member, in JCS member order, at which the two records differ, with both
values at that pointer.

### The corpus interface in the facade

`qsl-replay` SHALL carry a public module `corpus` that holds the whole corpus
interface, so that one implementation encodes, produces and loads cases:

- `corpus::CaseInput`: a case record without `format` and `expected`,
  built by its constructor from the members above.
- `corpus::reference_record(&CaseInput) -> Result<CaseRecord,
  CorpusRefusal>`: compiles the source, admits the case by the loader's
  rules below and runs it through the std reference with a fresh meter at
  the case's limits; the returned record's `expected` is that run's compared
  record.
- `CaseRecord::line()`: the record's JCS bytes followed by `\n`.
- `corpus::load(dir) -> Result<Vec<CaseRecord>, CorpusRefusal>`: the loader.

The module SHALL expose no checked closure, no evaluator handle and no
session: a caller gets only records. The std reference is the only
implementation `reference_record` runs. A record that `reference_record`
returns SHALL be admitted by `load` unchanged.

### Loader refusals

`corpus::load` SHALL admit a bundle whole or refuse it whole, and it SHALL
run no evaluation before every record is admitted. It checks the records in
file order and returns the first refusal, naming the 1-based line number,
the `case_id` when that member decoded, and the JSON pointer of the refused
member. Its refusals, in the order the loader checks them for one line:

| Code | Refused when |
| --- | --- |
| `missing_bundle` | the directory or its `cases.jsonl` does not exist or cannot be read |
| `empty_bundle` | `cases.jsonl` holds no record |
| `malformed_line` | a line is not UTF-8, is not one JSON object, is empty, or the file does not end in `\n` |
| `noncanonical_line` | a line's bytes differ from the JCS encoding of the object it decodes to |
| `unknown_format` | `format` is absent or is not `quire.evaluator-case/v1` |
| `member_set` | a member is missing, a member is not in the table above, or a member has the wrong JSON type |
| `invalid_case_id` | `case_id` is empty, longer than 128 bytes or holds a byte outside its set |
| `case_order` | `case_id` is not strictly greater, in UTF-8 byte order, than the previous line's (this covers a repeated `case_id`) |
| `unsupported_member` | `object_environment` is not `null` |
| `compile_refused` | `qsl_replay::compile_package` refuses `source`; the refusal carries the `ReplayRefusal` and its catalog code |
| `package_mismatch` | the compiled `package_id` differs from `package_id` |
| `unknown_function` | `function` names no function of the compiled unit that is callable by name |
| `unknown_generated_body` | a `generated_bodies` entry names no function of the compiled unit, or the list is not strictly ascending |
| `malformed_argument` | an `arguments` entry is not an FR-071 `WitnessValue` wire shape |
| `unknown_limit` | a `limits` key is not a `quire.value.accounting/v1` limit name |
| `malformed_expected` | `expected` is neither `null` nor a compared record of the shape above |

`reference_record` SHALL refuse a `CaseInput` with the same code a loader
would give the record it would write, so no exporter can produce a record
the loader refuses.

### The corpus

The QSL-owned corpus SHALL be:

1. The checked-in bundle `tests/fixtures/evaluator-corpus/cases.jsonl`. It
   holds the FR-262-AC-1 to FR-262-AC-8 programs at their criteria's inputs
   and limits, and the evaluation vectors of QSL's runtime evaluation
   conformance cases that a `function` call can express. A case joins the
   corpus by adding its record to this file, in `case_id` order; its
   `expected` is `null` or the output of `reference_record`.
2. The generated cases: 256 cases from a `proptest` strategy run with the
   fixed seed `0x5153_4C37`, each a unit of value functions over
   `Int[-1000, 1000]` and `Boolean` parameters whose bodies combine integer
   arithmetic (`+`, `-`, `*`, `div`, `rem`), comparisons, `if`-`then`-`else`
   and calls among up to three of the unit's functions, with arguments drawn
   from the parameters' domains and `work_units` limits drawn from 0 to
   twice the case's reference spend. The same seed and count SHALL yield
   byte-identical case records. A generated unit that `compile_package`
   refuses fails the run as a generator defect; it is not skipped.

An external bundle named by `EVALUATOR_CORPUS_DIR` SHALL be loaded with
`corpus::load` and run with the QSL-owned corpus. CG's generated-closure
bundle joins the corpus this way.

### The harness and its pass criterion

The harness SHALL run every case of the corpus through the std reference
and through the `no_std` implementation, each built as it ships (the `no_std`
implementation with its `std` feature disabled), each with a fresh meter at
the case's limits, and SHALL build each run's compared record. A case
diverges when the two records differ, or when its `expected` is not `null`
and differs from the std reference's record.

The harness SHALL run every case even after a divergence, and SHALL report
each divergent case on one line: its `case_id`, its bundle, which pair
differed (`std/no_std` or `expected/std`) and the first differing JSON
pointer with both values. A run that panics or overflows the host stack in
either implementation is a divergence of that case, reported as such.

The pass criterion SHALL be zero divergences. No list of known or tolerated
divergences exists, and no case may be excluded to make a run pass.

### CI placement

The harness SHALL be the `xtask` command `evaluator-differential`, run by
`make evaluator-differential` (which passes `EVALUATOR_CORPUS_DIR` through
when it is set). It SHALL join `make ci`'s lanes in the change that adds the
`no_std` implementation (IR-590), the first change with a second
implementation, and SHALL stay in `make ci` until IR-593 retires or demotes
the std path. The corpus interface, its loader refusals and the reference
entry do not need a second implementation; their criteria run in `cargo
test --workspace` from the change that adds `qsl_replay::corpus`.

`make ci` runs the QSL-owned corpus only, because QSL never builds CG
(ADR-011 FB-11). The run over CG's bundle SHALL use a CG checkout built
against the QSL revision under test: CG's `make evaluator-corpus` writes the
bundle, and QSL's `make evaluator-differential EVALUATOR_CORPUS_DIR=<dir>`
runs it. That run SHALL pass in the change that switches the default
implementation (IR-593).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-402-AC-1 | For a case of the FR-262-AC-1 `count` program called with `3` at default limits, `corpus::reference_record` returns a record whose `expected.result` is an `evaluation` with `outcome` `completed` and `value` `3`, whose `expected.charges` holds exactly four entries with `point` `function.call`, and whose `line()` is the JCS encoding of the record followed by `\n`; `corpus::load` of a bundle holding that line returns that record unchanged. | Test (TC-1112) |
| FR-402-AC-2 | Let `w` be AC-1's `expected.consumed` `work_units`. The same case with `work_units` limit `w - 1` produces `outcome` `incomplete` with `limit_kind` `work_units`, `consumed` `work_units` at most `w - 1`, and no recorded charge for the denied point; with limit `w` it produces AC-1's record. The same case with the argument `WitnessValue::Boolean(true)` produces `result.kind` `call_failure` with `arm` `input` and a `WrongValueKind` record, and is admitted by the loader, not refused. | Test (TC-1112) |
| FR-402-AC-3 | Starting from AC-1's one-line bundle, each of these single mutations makes `corpus::load` refuse the whole bundle with the stated code, naming line 1 and the refused member's pointer: no `cases.jsonl` (`missing_bundle`); an empty file (`empty_bundle`); a trailing line without `\n` (`malformed_line`); members reordered (`noncanonical_line`); `format` `quire.evaluator-case/v2` (`unknown_format`); an extra member `note` (`member_set`); `case_id` `A` (`invalid_case_id`); a non-null `object_environment` (`unsupported_member`); a source that does not parse (`compile_refused`, with the spine's catalog code); one changed byte of `package_id` (`package_mismatch`); `function` `nope` (`unknown_function`); `generated_bodies` `["nope"]` (`unknown_generated_body`); an argument `{"Integer": "3"}` that is not a `WitnessValue` wire shape (`malformed_argument`); a `limits` key `depth` (`unknown_limit`); `expected` `{}` (`malformed_expected`). A two-line bundle repeating AC-1's line refuses `case_order` at line 2. No refusal runs an evaluation. | Test (TC-1112) |
| FR-402-AC-4 | `corpus::reference_record` refuses each `CaseInput` that AC-3's member mutations describe with the code the loader gives the record it would write; `qsl_replay::corpus` exposes no checked closure, evaluator handle or session type in its public items. | Test (TC-1112) |
| FR-402-AC-5 | The harness run over a bundle whose single record is AC-1's with one `function.call` entry removed from `expected.charges` fails, reports that `case_id` with pair `expected/std` and a pointer under `/charges`, and still runs and reports every other case of the corpus. The same run with AC-1's unmodified record passes for that case. | Test (TC-1112) |
| FR-402-AC-6 | Two generations of the corpus's generated cases with seed `0x5153_4C37` and count 256 produce byte-identical `cases.jsonl` lines, each admitted by `corpus::load`. | Test (TC-1112) |
| FR-402-AC-7 | From the change that adds the `no_std` implementation until IR-593, `make ci` runs `evaluator-differential` over the QSL-owned corpus, and it reports zero divergences between the std reference and the `no_std` implementation for every case; the harness has no configuration that tolerates or skips a divergent case. | Test (TC-1112) |
| FR-402-AC-8 | Given a bundle written by CG's `make evaluator-corpus` from a CG checkout built against the QSL revision under test, `make evaluator-differential EVALUATOR_CORPUS_DIR=<dir>` loads every record and reports zero divergences, both `std/no_std` and `expected/std`; this run passes in the IR-593 switch change. | Test (TC-1112) |

## Dependencies

- [FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md): the interface
  both implementations serve, its charge rules and its programs.
- [FR-060](FR-060-check-qsl-api-surface-boundary.md) T12-A: limits CG's use
  of `qsl_replay::corpus` to its corpus producer.
- [FR-098](FR-098-execute-a-replay-request.md) and FR-071: the
  `WitnessValue` wire shape and its conversion, and the run over an
  environment with unresolved references.
- [AD-001](../assurance/AD-001-shared-evaluation-core-interface.md): slices
  S6 (IR-590), S7 (IR-591) and S9 (IR-593).
- agent-ix/quire-contract-codegen FR-044 (IR-591): CG's producer writes its
  bundle with `qsl_replay::corpus`.
