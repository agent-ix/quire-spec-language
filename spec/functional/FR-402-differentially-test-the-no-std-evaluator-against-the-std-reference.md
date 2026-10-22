---
id: FR-402
title: "Differentially test the no_std evaluator against the std reference over one evaluator corpus"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-401
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
interface, hosted in `quire-eval`
([FR-401](FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md)),
is built behind it. The two implementations must agree on outcome, refusal,
location, losses and accounting for shared inputs, and the reference may be
switched out only once the second implementation passes the shared
differential suite. This requirement defines that suite: which inputs it
runs, the record it compares, how an input bundle is encoded and loaded,
what the loader refuses, how the harness runs and what counts as a failure.

The suite runs one corpus of evaluator cases. A case is one checked
application: a QSL source unit and its library dependencies, compiled through
the layer-6 facade, a function of that unit, completed arguments and limits.
Each case runs through both implementations with a fresh meter, and the two
compared records must be equal. QSL owns the case encoding, the loader and
the compared record. The facade module `qsl_replay::corpus` holds the case
encoding and the loader and evaluates nothing, so the code generator (CG)
can export its generated-closure cases in QSL's encoding without defining an
encoding of its own and without any evaluation entry in the facade
([FR-060](FR-060-check-qsl-api-surface-boundary.md) T12-A, "T12-A: the
evaluator corpus producer"). Only QSL's harness evaluates cases.

## Inputs

- A corpus bundle: a directory holding one file, `cases.jsonl`.
- The QSL-owned corpus: the checked-in bundle
  `tests/fixtures/evaluator-corpus/cases.jsonl` and the generated cases.
- Optionally, an external bundle named by `EVALUATOR_CORPUS_DIR`, such as the
  one CG's `make evaluator-corpus` writes.

## Outputs

- From `corpus::encode`: one case record, or one `CorpusRefusal`.
- From `corpus::load`: every case of a bundle, or one `CorpusRefusal` for the
  whole bundle.
- From the harness: a pass, or one report line per divergent case and a
  failing exit.

## Behavior

### JSON encoding

Every JSON text this requirement defines SHALL be produced by
`quire-canonical`, the one RFC 8785 (JCS) encoder (ADR-013 §2). Every
integer that is a `u64`, a `usize` or a kernel `Integer` (limits, counters,
charge sizes, `Incomplete` amounts, parameter indices) SHALL be written as a
JSON string holding its canonical decimal form (no leading `+`, no leading
zeros, `-` only for a negative value), so no JSON number above 2^53 occurs.

### The case record

A bundle's `cases.jsonl` SHALL be UTF-8 JSON Lines: each line is one case
record, written as its JCS text followed by one `\n`, and the file holds no
other bytes. Records SHALL appear in strictly ascending UTF-8 byte order of
`case_id`.

A source object is a JSON object with exactly the members `identity` (an
object with exactly the four FR-001 labels `authority`, `identity`,
`revision_namespace` and `revision`, each a string, as `SourceIdentity`
holds them), `path` (the display path, a string) and `text` (the unit's
source, a string).

A case record SHALL be a JSON object with exactly these members:

| Member | Content |
| --- | --- |
| `format` | the string `quire.evaluator-case/v1` |
| `case_id` | a string of 1 to 128 bytes drawn from `a`-`z`, `0`-`9`, `.`, `_`, `-` and `/`, unique in the bundle |
| `source` | a source object: the unit under test |
| `dependencies` | an array of objects with exactly `identity` (the library identity string) and `source` (a source object), strictly ascending in UTF-8 byte order of `identity`; empty for a unit with no library dependency |
| `package_id` | the `package_id` that `qsl_replay::compile_package` returns for `source` and `dependencies`, in the digest-record shape the `quire.checked-package/v2` bytes declare it |
| `function` | the declared name of a function of the compiled unit that is callable by name |
| `arguments` | an array of FR-071 `WitnessValue`s in their wire shape, one per declared parameter, in declared parameter order |
| `object_environment` | `null` or a JSON object |
| `limits` | an object mapping `quire.value.accounting/v1` counter names to canonical decimal strings; a counter with no entry runs at its published default |
| `generated_bodies` | an array of declared function names of the compiled unit, strictly ascending and unique, naming the functions the exporter lowered to generated bodies; empty for a QSL-owned case |
| `expected` | `null`, or a compared record |

A case's checked input is `source` compiled through
`qsl_replay::compile_package` with the dependency input built from
`dependencies` as FR-098 builds it from a replay request's entries (each
entry's `identity`, and its source's labels, path and text), with no domain
packages, under the published stage limits. The four labels enter the
emitted package, so `package_id` binds the checked closure the loader
admits to the one the exporter admitted.

A case runs over `ObjectEnvironment::with_unresolved_references`, as a replay
request without `object_environment` does (FR-098). Version 1 carries no
object environment and no domain package: a non-null `object_environment`
refuses `unsupported_member`, and a unit whose `model` selection names a
domain package refuses `compile_refused` with the compile's
`missing_import` refusal.

Arguments become values of their parameters' declared types by FR-098's
`WitnessValue` conversion. A converted argument that admission then refuses
(arity, kind, domain) is not a loader refusal: it is the case's result,
`CallFailure::Input`, and it is compared like any other result.

`generated_bodies` records which callables the exporter lowered; the
harness runs every callable interpreted, with no registered body.

### The compared record

The compared record of one run SHALL be a JSON object with exactly the
members `result`, `consumed` and `charges`, built from the run's
`Result<Evaluation, CallFailure>` (the types FR-401 places in `quire-eval`)
and its meter. Every form below is defined here from the public members of
the named types; where a type has no public member that this record names,
the record carries no payload for it, as stated.

`result` SHALL be one of:

- `{"kind": "evaluation", "outcome": <outcome>, "location": <location or
  null>, "losses": [<loss>, ...]}`, with `location` from `Evaluation.location`
  and `losses` from `Evaluation.losses` in evaluation order.
- `{"kind": "call_failure", "arm": <arm>, ...}`, with `arm` the
  `CallFailure` variant in snake case and these payload members: `input`
  carries `code` and `cause`, the `InputRefusal`'s catalog code and cause
  strings; `fault` carries `stage` and `invariant`, the `InternalFault`'s;
  `storage` carries `parameter`; `cancelled`, `supplied` and
  `unsupported_helper` carry no payload member.

`<outcome>` SHALL be one of:

| Arm | Members besides `arm` | Source |
| --- | --- | --- |
| `completed` | `value`: the value's canonical JSON form, the form whose RFC 8785 text is the value's simulation state key (FR-262-AC-2) | `Outcome::Completed` |
| `incomplete` | `limit_kind`, `charge_point` (their `as_str` names), `limit`, `consumed`, `next_charge` | `Outcome::Incomplete`'s `Incomplete` |
| `refused` | `code`, `cause`: the kernel `Refusal`'s `code()` and `cause()`, each `null` where it returns none | `Outcome::Refused` |
| `undefined` | `reason`: the kernel `Undefined` variant in snake case | `Outcome::Undefined` |
| `family_refused` | `code`, `cause`: the `CatalogCoded` cause's `catalog_code()` code and cause | `FamilyResult::Refused` |
| `family_undefined` | `reason`, `fields`: the `UndefinedCoded` cause's `undefined_record()` reason and fields (an object of strings) | `FamilyResult::Undefined` |

`<location>` SHALL be `{"origin": <origin>, "path": [...]}`: `origin` is
`{"kind": <Origin variant in snake case>}` plus that variant's fields under
their SV field names, and `path` is `Location::path()` as decimal strings.

`<loss>` SHALL be `{"location": <location>, "kind": <ValueLoss variant in
snake case>}` (`decimal`, `ieee_exact` or `ieee_flags`). Version 1 compares a
loss's kind and location, not its payload, because no encoding of the SV
loss payloads exists.

`consumed` SHALL map each of the ten `quire.value.accounting/v1` counter
names (`integer_bits`, `decimal_digits`, `scale_expansion`,
`text_input_bytes`, `text_scalars`, `normalized_scalars`, `unit_edges`,
`value_occurrences`, `work_units`, `result_units`) to `Meter::consumed` for
that counter after the run.

`charges` SHALL be the run's ordered charge log or `null`. The kernel meter
keeps a charge log only under `quire-exact`'s `test-support` feature, which a
normal or build dependency never enables, and holds at most its capacity of
4096 admitted charges (`CHARGE_LOG_CAPACITY`). Where the run's meter keeps a
log and the log is not truncated, `charges` is the list of admitted charges
in order, each `{"point": <charge-point name>, "sizes": {<counter>:
<amount>, ...}}`, with one `sizes` member for each size counter
`AdmittedCharge` records for that charge. Otherwise `charges` is `null`: in a
build without the log, and in a run whose log was truncated. A case of more
than 4096 charges, such as FR-262-AC-1's 100,000 calls, is therefore
compared on `result` and `consumed` only.

Two compared records SHALL be equal exactly when their JCS bytes are equal.
A difference SHALL be reported as the RFC 6901 JSON pointer of the first
member, in JCS member order, at which the two records differ, with both
values at that pointer.

### The corpus interface in the facade

`qsl-replay` SHALL carry a public module `corpus` that holds the case
encoding and the loader:

- `corpus::CaseInput`: a case record's members other than `format`,
  `package_id` and `expected`.
- `corpus::encode(&CaseInput) -> Result<CaseRecord, CorpusRefusal>`:
  compiles `source` with `dependencies` through `compile_package`, fills
  `package_id` from the compile, sets `expected` to `null`, and admits the
  record by the loader's rules.
- `CaseRecord::line()`: the record's JCS text followed by `\n`.
- `corpus::load(dir) -> Result<Vec<CaseRecord>, CorpusRefusal>`: the loader.

The module SHALL evaluate nothing and SHALL expose no checked closure, no
evaluator handle, no session and no compared-record producer. A record that
`encode` returns SHALL be admitted by `load` unchanged. `encode` SHALL refuse
a `CaseInput` with the code the loader would give the record it would
write.

### Loader refusals

`corpus::load` SHALL admit a bundle whole or refuse it whole, and it SHALL
run no evaluation. It applies the bundle checks first, then checks each line
in file order, applying the line checks in the table's order, and returns
the first refusal. A bundle refusal names no line and no member. A line
refusal names the 1-based line number and the `case_id` when that member
decoded; a refusal of a member also names the member's JSON pointer, and
`malformed_line` and `noncanonical_line` name none.

| Order | Code | Scope | Refused when |
| --- | --- | --- | --- |
| 1 | `missing_bundle` | bundle | the directory or its `cases.jsonl` does not exist or cannot be read |
| 2 | `empty_bundle` | bundle | `cases.jsonl` has zero bytes |
| 3 | `malformed_line` | line | the file does not end in `\n`, or a line is empty, is not UTF-8 or is not one JSON object |
| 4 | `noncanonical_line` | line | a line's bytes differ from the JCS text of the object it decodes to |
| 5 | `unknown_format` | member | `format` is absent or is not `quire.evaluator-case/v1` |
| 6 | `member_set` | member | a member is missing, a member is not in the case-record table, or a member, a source object member or a `dependencies` entry member has a JSON type other than its row states |
| 7 | `invalid_case_id` | member | `case_id` is empty, longer than 128 bytes or holds a byte outside its set |
| 8 | `case_order` | member | `case_id` is not strictly greater, in UTF-8 byte order, than the previous line's (this covers a repeated `case_id`) |
| 9 | `invalid_limit` | member | a `limits` key is not one of the ten counter names, or its value is not a canonical decimal string of a `u64` |
| 10 | `unsupported_member` | member | `object_environment` is a JSON object |
| 11 | `dependency_order` | member | `dependencies` is not strictly ascending by `identity` |
| 12 | `invalid_package_id` | member | `package_id` is not a digest record of the `quire.checked-package/v2` shape |
| 13 | `compile_refused` | member | `compile_package` refuses the case's source and dependencies; the refusal carries the `ReplayRefusal` and its catalog code, and its pointer is `/source` |
| 14 | `package_mismatch` | member | the compiled `package_id` differs from `package_id` |
| 15 | `unknown_function` | member | `function` names no function of the compiled unit that is callable by name |
| 16 | `unknown_generated_body` | member | a `generated_bodies` entry names no function of the compiled unit, or the list is not strictly ascending |
| 17 | `malformed_argument` | member | an `arguments` entry is not an FR-071 `WitnessValue` wire shape |
| 18 | `malformed_expected` | member | `expected` is neither `null` nor a compared record of the form above |

A file holding only `\n` is non-empty and refuses `malformed_line` at line 1.

### The corpus

The QSL-owned corpus SHALL be:

1. The checked-in bundle `tests/fixtures/evaluator-corpus/cases.jsonl`,
   holding exactly these cases (each a `function` call; a vector written as
   an expression becomes a nullary function whose body is that expression):
   - `fr262/ac1/count-100000`, `fr262/ac1/count-100000-short` and
     `fr262/ac1/count-100000-exact`: FR-262-AC-1's `count` called with
     `100000`, at a `work_units` limit that fits, at one unit below the
     reference spend and at exactly the reference spend;
   - `fr262/ac2/list-100000`: FR-262-AC-2's recursive `List` built 100,000
     long by a recursive function;
   - `fr262/ac5/two-arguments` and `fr262/ac5/literal`: FR-262-AC-5's source
     call with two argument applications, and a standalone literal;
   - `fr402/count-3`, `fr402/count-3-short` and `fr402/count-3-boolean`: the
     `count` program called with `3` at default limits, at one unit below
     its reference spend, and with the argument `WitnessValue::Boolean(true)`;
   - `tc069/01` to `tc069/14`: TC-069's fourteen signed `div`, `rem`, `+`,
     `-`, `*` and negation vectors, in their listed order.
   A case joins the corpus by adding its record to this file, in `case_id`
   order. A checked-in `expected` is `null` or a compared record the
   harness's std build wrote for that case.
2. The generated cases, from QSL's own deterministic generator (no
   third-party random or property-testing dependency). The generator is
   SplitMix64 seeded with `0x5153_4C37`; case `i` (0 to 255) draws from the
   stream after the `i`th reseed with `0x5153_4C37 + i`. Each case is a unit
   of one to three functions `f0` to `fn`; a function calls only
   higher-numbered functions, so calls are acyclic and need no `decreases`
   measure. Each function has one to three parameters, each `Int[-1000,
   1000]` or `Boolean`, and a declared result `Integer` or `Boolean`. An
   `Integer` body combines parameters and literals in `[-1000, 1000]` with
   `+`, `-`, `*`, and `div` and `rem` by a nonzero literal from 1 to 9, to an
   expression depth of at most 4; a `Boolean` body combines comparisons of
   such expressions and `Boolean` parameters with `and`, `or` and `not`;
   either may be an `if`-`then`-`else` on a `Boolean` expression. A call's
   argument to an `Int[-1000, 1000]` parameter is a parameter of that type
   or a literal in its domain, and to a `Boolean` parameter a `Boolean`
   parameter or literal. The case calls `f0` with arguments drawn from its
   parameters' domains; its `limits` is empty for an even `i` and sets
   `work_units` to a draw from 0 to 2000 for an odd `i`. The same seed and
   count SHALL yield byte-identical case records. A generated unit that
   `compile_package` refuses fails the run as a generator defect; it is not
   skipped.

An external bundle named by `EVALUATOR_CORPUS_DIR` SHALL be loaded with
`corpus::load` and run with the QSL-owned corpus. CG's generated-closure
bundle joins the corpus this way.

FR-262-AC-3, AC-4, AC-6, AC-7 and AC-8 (registry results, registered
generated bodies, stepping, cancellation and resumption) cannot be written
as a case, because a case registers no body and cancels nothing. The suite
compares them by running those criteria's tests against both builds: each
test SHALL pass against the std reference and against the `no_std`
implementation, and each asserts the reference's result and charge
sequence.

### The harness and its pass criterion

The std reference and the `no_std` implementation SHALL run as two separate
builds of the harness's runner, because one Cargo build unifies features and
cannot hold both. Each build evaluates every case on a thread with a 512 KiB
stack, with a fresh meter at the case's limits and every callable
interpreted, and writes one compared record per `case_id`. Both builds are
test builds of the runner, so each keeps the kernel charge log. The
comparator then pairs the records by `case_id`. How the runner is built
twice is the `no_std` implementation's concern.

A case diverges when its two records differ, when either build produced no
record for it, or when its `expected` is not `null` and the std record
differs from it in `result` or `consumed`, or in `charges` when neither is
`null`. A run that panics or overflows the host stack in either build
produces no record for that case.

The harness SHALL run every case even after a divergence, and SHALL report
each divergent case on one line: its `case_id`, its bundle, which pair
differed (`std/no_std` or `expected/std`) and the first differing JSON
pointer with both values, or the build that produced no record.

The pass criterion SHALL be zero divergences. The harness SHALL accept no
option, list or configuration that tolerates, skips or excludes a case.

### Where the harness runs

The harness SHALL be the `xtask` command `evaluator-differential`, run by
`make evaluator-differential`, which passes `EVALUATOR_CORPUS_DIR` through
when it is set.

While QSL ships both the std reference and the `no_std` implementation,
`make ci` SHALL run `evaluator-differential` over the QSL-owned corpus.
`make ci` does not run CG's bundle, because QSL never builds CG (ADR-011
FB-11). A run over CG's bundle uses a CG checkout built against the QSL
revision under test: CG's `make evaluator-corpus` writes the bundle, and
`make evaluator-differential EVALUATOR_CORPUS_DIR=<dir>` runs it. The corpus
interface and its refusals need no second implementation, and their criteria
run in `cargo test --workspace`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-402-AC-1 | `corpus::encode` of the `fr402/count-3` input (FR-262-AC-1's `count`, argument `3`, default limits, no dependencies) returns a record with `expected` `null`, `package_id` equal to `compile_package`'s for the same source identity, path and text, and a `line()` that is the record's `quire-canonical` text followed by `\n`; `corpus::load` of a bundle holding that line returns the record unchanged. The same input with a changed `source.identity.revision` encodes a different `package_id`. | Test (TC-1112) |
| FR-402-AC-2 | A case whose `source` imports a library supplied in `dependencies` encodes and loads, and its `package_id` equals `compile_package`'s with that dependency input; with the dependency entry removed, `encode` refuses `compile_refused` carrying `missing_import`; with two entries out of `identity` order, `load` refuses `dependency_order`. | Test (TC-1112) |
| FR-402-AC-3 | Each bundle below makes `corpus::load` refuse the whole bundle with the stated code, and no load runs an evaluation. Bundle refusals, naming no line: no `cases.jsonl` (`missing_bundle`); a zero-byte file (`empty_bundle`). Line refusals at line 1 with no pointer: a file holding only `\n`, and AC-1's line without its `\n` (`malformed_line`); AC-1's line with members reordered (`noncanonical_line`). Member refusals at line 1 naming the pointer, each one mutation of AC-1's record: `format` `quire.evaluator-case/v2` (`unknown_format`, `/format`); an extra member `note` (`member_set`, `/note`); `object_environment` `true` (`member_set`, `/object_environment`); `case_id` `A` (`invalid_case_id`, `/case_id`); `limits` `{"depth": "1"}` and `limits` `{"work_units": "-1"}` (`invalid_limit`, `/limits/depth` and `/limits/work_units`); `object_environment` `{}` (`unsupported_member`, `/object_environment`); `package_id` `"x"` (`invalid_package_id`, `/package_id`); a source that does not parse (`compile_refused`, `/source`, with the compile's catalog code); one changed digest byte of `package_id` (`package_mismatch`, `/package_id`); `function` `nope` (`unknown_function`, `/function`); `generated_bodies` `["nope"]` (`unknown_generated_body`, `/generated_bodies/0`); an argument `{"nope": 1}` (`malformed_argument`, `/arguments/0`); `expected` `{}` (`malformed_expected`, `/expected`). A two-line bundle repeating AC-1's line refuses `case_order` at line 2. | Test (TC-1112) |
| FR-402-AC-4 | `corpus::encode` refuses each `CaseInput` built from AC-3's member mutations of the case members it takes with the code the loader gives the record it would write, and `qsl_replay::corpus` has no public function that evaluates and no public checked-closure, evaluator, session or compared-record type. | Test (TC-1112) |
| FR-402-AC-5 | The std build's compared record for `fr402/count-3` has `result.outcome` `completed` with `value` the canonical JSON form of `3`, a `charges` list with exactly four entries whose `point` is `function.call`, and `consumed` with all ten counters as decimal strings. For `fr402/count-3-short` it has `outcome` `incomplete` with `limit_kind` `work_units`, `charge_point` naming the denied point, and no `charges` entry for the denied charge. For `fr402/count-3-boolean` it has `result.kind` `call_failure`, `arm` `input` and the `WrongValueKind` code. For `fr262/ac1/count-100000` it has `charges` `null` and `outcome` `completed` with value `100000`. | Test (TC-1112) |
| FR-402-AC-6 | A harness run over a bundle holding `fr402/count-3` with an `expected` whose `consumed.work_units` is one less than the std record's fails, reports that `case_id` with pair `expected/std` and pointer `/consumed/work_units`, and still runs and reports every other case; the same run with the std build's own record as `expected` passes for that case. A comparator fed two record sets that differ only in one `charges` entry's `sizes` reports pair `std/no_std` and a pointer under `/charges`; a record set missing one `case_id` reports that case and the build that produced no record. | Test (TC-1112) |
| FR-402-AC-7 | The checked-in bundle loads and holds exactly the `case_id`s the corpus list names; the generator, run twice with seed `0x5153_4C37` and count 256, produces byte-identical lines that all load; every generated unit compiles; and the QSL workspace's `Cargo.lock` gains no random-number or property-testing crate for the generator. | Test (TC-1112) |
| FR-402-AC-8 | `make ci` runs `evaluator-differential` over the QSL-owned corpus as a separate std build and `no_std` build, and the run reports zero divergences; FR-262-AC-3, AC-4, AC-6, AC-7 and AC-8's tests pass against both builds; and the harness's command line, environment and configuration offer no setting that tolerates, skips or excludes a case. | Test (TC-1112) and Inspection |
| FR-402-AC-9 | Given a bundle that CG's `make evaluator-corpus` writes from a CG checkout built against the QSL revision under test, `make evaluator-differential EVALUATOR_CORPUS_DIR=<dir>` loads every record, every record's `expected` is `null`, and the run reports zero divergences. | Test (TC-1112) |

## Dependencies

- [FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md): the interface
  both implementations serve, its charge rules and its programs.
- [FR-401](FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md):
  `Evaluation` and `CallFailure` in `quire-eval`.
- [FR-060](FR-060-check-qsl-api-surface-boundary.md) T12-A: limits CG's use
  of `qsl_replay::corpus` to its corpus producer.
- [FR-098](FR-098-execute-a-replay-request.md) and FR-071: the dependency
  input built from entries, the `WitnessValue` wire shape and its
  conversion, and the run over an environment with unresolved references.
- [AD-001](../assurance/AD-001-shared-evaluation-core-interface.md): the
  slices that add the `no_std` implementation and switch the default.
- agent-ix/quire-contract-codegen FR-044 (IR-591): CG's producer writes its
  bundle with `qsl_replay::corpus::encode`. FR-044 as written conflicts
  with this requirement in four places, which CG resolves: it names the
  format `quire.generated-closure-evaluation/v1` (the loader refuses it as
  `unknown_format`); it records an expected evaluation from QSL's std
  reference (CG writes `expected` `null`, and the facade evaluates
  nothing); it requires object environments (version 1 refuses a non-null
  one); and it reaches "the public checked-closure and application
  interface" (CG reaches QSL only through `compile_package` and
  `qsl_replay::corpus`). The generated-body registration identity is the
  `generated_bodies` list of function names.
