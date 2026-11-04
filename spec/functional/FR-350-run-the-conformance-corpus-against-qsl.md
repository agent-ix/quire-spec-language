---
id: FR-350
title: "Run the complete-V1 conformance corpus against QSL"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-035
    type: implements
  - target: ix://agent-ix/quire-specification/FR-311
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-336
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-003
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-323
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: depends_on
---
# FR-350: Run the complete-V1 conformance corpus against QSL

## Description

When a maintainer runs the `qualify` repository gate (FR-353) over a
conformance corpus,
the runner SHALL execute every vector whose applicability names a QSL
subject through the QSL public entry point for that subject, and SHALL
compare the typed result with the vector's expected disposition.

QSpec FR-336 owns the corpus schema, vector vocabulary, applicability,
expected-value encoding and input adapters. This requirement specifies how
QSL selects its applicable vectors, invokes its public operations and settles
their outcomes against the corpus oracle.

## Inputs

- The corpus, read from the location the caller names (the QSpec
  repository's published corpus). The runner reads the corpus in place.
- An optional capability scope (FR-351).
- Resource limits for each vector: the stage limits (FR-096) and the
  evaluation budget the compiler and runtime already take, each with its
  published default, each raisable by the caller.

## Outputs

One outcome for each executed vector: `passed`, `failed`, `unsupported`,
`incomplete` or `tool-failure`, each holding the vector id, its capability
id and, unless `passed`, the expected and actual typed result. A `passed`
outcome holds the output bytes its entry point returned (Determinism).
These bytes are a runner output, not part of the FR-352 report. `incomplete`
and `tool-failure` are QSpec FR-311 `unexecuted` results, kept apart so the
report names why the vector did not run to a result.

## Behavior

### Subjects and applicability

The QSL subjects are three QSpec FR-311 consumer types, each reached through
the entry point a caller of that subject uses:

| Consumer type | QSL subject | Entry point |
| --- | --- | --- |
| `language` | the compiler (S0 to S4 and the checked package) | `compile` |
| `runtime` | the reference runtime, and the replay facade for a vector whose input is a replay request | `run`, `replay` |
| `tool` | the formatter | `format` |

QSL reads capability applicability from each
`coverage.primary[*].applicability` and vector applicability from each
`vectors[*].applicability`. A primary capability is in the default scope
(FR-351) when its `consumer_types` includes a QSL consumer type and one of
its `semantic_profiles` is a profile identity supported by QSL under FR-110,
FR-230, or FR-361. The supported set is resolved from those closed typed
profile roles; this requirement does not define or compare a second definition
catalog. A vector SHALL run
when its `capability_id` selects that primary row, its own
`applicability.consumer_types` includes its `subject.consumer_type`, and
that subject maps to a QSL entry point in the table above. Vectors of an
out-of-scope capability SHALL NOT execute. The runner calls no internal
function a caller cannot reach.
If applicability filtering leaves the scope empty, then the runner SHALL
refuse before vector execution under
[FR-353](FR-353-settle-the-complete-v1-qualification-verdict.md).

### Corpus and limits ownership

The runner SHALL read the selected QSpec corpus and the files named by each
vector's `input.files` without rewriting its vectors, `expected`
dispositions, `results` or `coverage`. Expected dispositions come from the
corpus's independent oracle; the runner SHALL NOT replace them with results
calculated by QSL. A syntactically valid file path that is not readable is
that vector's `tool-failure`, not a corpus preflight refusal.

The runner SHALL resolve `--limit` operands through the existing
[FR-255 settings operation](FR-255-name-the-setting-that-raises-a-reached-limit.md).
Q1 owns no additional setting names or defaults. The subject receives those
effective limits at its public entry point; a setting for a stage that the
subject does not run does not cause that stage to run. Evaluation budgets
retain the subject's accounting contract, including any override the
FR-255 settings operation defines.

### QSL public-operation adapters

The corpus and its adapter encodings are defined by QSpec FR-336 (AC-4 to
AC-8). QSL SHALL consume the input adapters applicable to its public
operations under that contract and invoke only the corresponding QSL public
operations: `compile` (FR-027), `format` (FR-003), `run` (FR-100) and
`replay` (FR-071/FR-098). QSL adds no product request wire or corpus-owned
encoding. Existing model and object-context admission (FR-084, FR-089,
FR-106), supplied-value limits (FR-277), replay result semantics (FR-072),
and all other owning admission and refusal rules remain authoritative.
### The replay-request adapter

The runner reads a `replay-request` file with its own decoder in `xtask`,
because QSL has no byte reader for a replay request (FR-071's wire is an
in-process shape, and the corpus encoding is corpus-owned, not a product wire).
The decoder is test-support: it produces a `ReplayRequestWire` and decides
nothing about the request. A document that does not match the closed schema is
a defect of the corpus, so it is a per-vector `tool-failure`; a request that
matches the schema and that QSL refuses is a QSL result and is settled as one.
The adapter is needed independently of any one request member: FR-350-AC-10
requires every in-scope `replay` vector to run, and `xtask` decodes no
`replay-request` file without it.

The published `$defs.ReplayRequest` has 14 properties, and `package` (the
proved FR-322 checked package, required) is one of them (QSpec FR-336-AC-7).
The adapter follows the schema, which is authoritative for the corpus
encoding, and FR-071 stays unchanged: it maps `package.checked_package` to the
request's `package_id`, `package.sources` to the package reference's `sources`,
`package.dependencies` to its `dependencies` entries (`identity`, `package_id`,
`sources`) and an optional `package.refinement_internals` to the member of that
name. It compares `package.contract_version` with the corpus contract version
and stores it nowhere in the request, because FR-071 carries no version member
and the version check belongs to the decoder. A `replay-request` file without
`package`, whose `package` is not a checked-package document, or whose
`contract_version` differs, is a `corpus-decode` `tool-failure`.
The decoder refuses, as the consumer of QSpec FR-336-AC-8, a
`bytes_path` that is absent, not listed as `kind: bytes` in the vector, absolute
or parent-traversing, and a `stage_limits` key that names no FR-255 setting,
each before the request is constructed.

### Settlement

If executing a vector panics, returns an internal fault, or cannot read a
file the vector names, then its outcome SHALL be `tool-failure`, holding
the panic message, the fault or the I/O error. If an input file the vector
names is not a valid document of its kind (a `replay-request` file that does
not match `$defs.ReplayRequest`), then its outcome SHALL be `tool-failure`
holding the cause `corpus-decode` and the JSON pointer of the first defect. Otherwise the runner SHALL
settle the vector by the first of these, in this order, that holds:

1. `passed`: the actual result matches the expected disposition. A value,
   verdict or checked package identity matches when it is equal. A refusal
   matches when it has the same catalog code and cause, and the same locus
   when the vector names one. This includes an expected limit refusal
   (a boundary or `incomplete`-class vector) and an expected
   `unknown_required_feature` or `unsupported_projection` refusal.
2. `unsupported`: QSL returned `unsupported_projection` or
   `unknown_required_feature`, holding the code and cause QSL returned.
3. `incomplete`: the vector reached a resource limit, retaining its kind,
   bound and actual charge/count, and the setting or request field through
   which the subject's public contract raises it.
4. `failed`: any other result, holding the expected and the actual result.
   A refusal with the right code and a different cause is `failed`, as is a
   success where a refusal was expected and the reverse.

### Isolation

The runner SHALL settle each vector independently: a `tool-failure`,
`incomplete` or `failed` vector SHALL change no other vector's outcome, and
the runner SHALL continue with the next vector.
Each vector SHALL start with fresh resource accounting at its effective
limits; a previous vector's consumption cannot reduce its remaining budget.

### Determinism

The runner SHALL give the same outcome for each vector on every run over
the same corpus with the same QSL build and limits. It reads no clock,
environment variable or search path to decide an outcome.

The runner SHALL execute the selected vectors one after another on the
calling thread; it starts no thread and runs no two vectors concurrently.
Thread count is therefore not a runner input. Each vector's outcome, and
for a `passed` vector the bytes its subject's entry point returned, SHALL be
the same whatever number of other threads the calling process runs, so
state shared between vectors or between threads cannot change a result.

Settlement compares the subject's typed result with `expected.value`. The
bytes used for determinism are a separate observation and are never substituted
for that expected value. For replay, the adapter SHALL map the FR-072
`ReplayResult` into the FR-323 envelope `{"results":[one FR-072 result]}` for
settlement. Separately, determinism compares the raw FR-286 outcome-document
bytes returned by two executions of the same replay request; those bytes are
compared only with each other, never with `expected.value`.

For a `passed` vector, settlement and determinism use these representations:

| Entry point | Value compared with `expected.value` for settlement | Output bytes compared between repeated executions for determinism |
| --- | --- | --- |
| `compile` | the encoded checked package | the encoded checked package |
| `run` | the typed result defined by its public entry point | the [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md) outcome document |
| `replay` | the FR-323 `{"results":[one FR-072 result]}` envelope mapped from the typed FR-072 `ReplayResult` | the raw FR-286 outcome-document bytes |
| `format` | the formatted text | the formatted text |

Vector ids are ordered bytewise on their id strings, as FR-075 orders
`BackendId`s. This is also the ascending vector-id order of the FR-352
report.

The determinism sample of the published QSpec corpus is chosen by rule,
not by a recorded list. Within each capability of the default scope, for
each vector class, it takes the smallest id among the vectors of that class
that the runner executes, that is, those whose own applicability names a QSL
consumer type. A class with no such vector contributes none.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-350-AC-1 | Over a fixture corpus holding one positive compile vector, one positive `run` vector, one `replay` vector and one `format` vector, each with the expected result QSL returns, each vector is `passed` and each ran through its subject's public entry point. | Test |
| FR-350-AC-2 | A refusal vector expecting `ill_typed`/`unit-mismatch` is `passed` when QSL returns that pair, and `failed` when QSL returns `ill_typed`/`type-mismatch`, the failure holding the expected and the actual code and cause. A refusal vector over source QSL admits is `failed`, holding the expected refusal and the actual success. | Test |
| FR-350-AC-3 | A vector that expects a success, which QSL answers with `unknown_required_feature`/`unknown-feature`, is `unsupported`, holding that pair; the same answer to a vector that expects that pair is `passed`. A vector that expects a success, run with an evaluation budget of zero, is `incomplete`, retaining its budget kind, bound, actual count and the subject's setting or request field that raises it, and with the default budget is `passed`. A boundary vector that expects the zero-budget limit refusal, run with a budget of zero, is `passed`. A vector expecting `ill_typed`/`type-mismatch` that QSL answers with `unknown_required_feature` is `unsupported`. A positive compile vector checked under `--limit s3.nodes=0` is `incomplete` naming `s3.nodes`; raising that setting to fit yields `passed`, with actual typed results equal to direct public calls under the same effective limits. | Test |
| FR-350-AC-4 | A vector naming a source file absent from the corpus is `tool-failure` holding the I/O error, and the vectors before and after it in the corpus keep the outcomes they have when it is removed. | Test |
| FR-350-AC-5 | Two runs over the same fixture corpus with the same build and limits give equal per-vector outcomes. Two identical successful vectors, each requiring the full configured budget, both pass in either corpus order. The corpus and its named input bytes are unchanged after execution, including its recorded results and coverage. | Test |
| FR-350-AC-6 | One test starts four child `cargo test` processes over the determinism sample of the published QSpec corpus. Two run with `RUST_TEST_THREADS=1`. Two run with `RUST_TEST_THREADS=2`, and in each of those a second test runs the runner over the same sample at the same time, both starting at one barrier. Each child writes its FR-352 JSON report, and the output bytes of each `passed` outcome, to its own output directory. The parent finds the four reports byte-equal, and for each vector `passed` in all four children, finds its four output byte strings byte-equal. | Test |
| FR-350-AC-7 | The runner's source starts no thread, uses no thread pool or async executor, and calls each vector's entry point only after the previous vector has settled. | Inspection |
| FR-350-AC-8 | For each in-scope `run` vector, QSL adapts the corpus input to the existing FR-100 `Call` and invokes the public runtime with the effective evaluation and supplied-value limits. Arguments retain their authored order and bind to the selected function's declared parameters; a value that cannot be adapted is refused under the corpus adapter or FR-100 admission, never silently dropped. | Test (TC-875) |
| FR-350-AC-9 | For an in-scope `run` vector with object context, QSL admits the selected model and object environment through FR-084/FR-089/FR-106 and resolves any population argument to the binding established by that admission before S6a. Native-run/1 does not gain a population argument wire from qualification. | Test (TC-875) |
| FR-350-AC-10 | For each in-scope `replay` vector, QSL adapts the corpus input to the existing FR-071 `ReplayRequestWire`, supplies its bytes to FR-098, and settles the FR-072 typed result. Byte-path resolution follows QSpec FR-336-AC-8; malformed request data retains the owning refusal and an unreadable named file is a per-vector `tool-failure`. | Test (TC-875) |
| FR-350-AC-11 | The `replay-request` adapter of the runner decodes a vector's `replay-request` file, a closed document of QSpec FR-336 `$defs.ReplayRequest`, into `ReplayRequestWire` member by member as FR-336's mapping states, and gives the result to `qsl_replay::replay`; a corpus whose replay vectors the runner could not decode before this adapter now executes them. | Test (TC-1325) |
| FR-350-AC-12 | The adapter maps each `sources` entry, of the package and of each `dependencies` entry, to its authority and identity as the file states them, with no revision label, and maps the `byte_provision` rows by reading each sibling `bytes_path` file as raw bytes. | Test (TC-1325) |
| FR-350-AC-13 | The adapter maps `object_environment` to the closed `{authority, identity}` identity and refuses a file whose `object_environment` identity carries any other member. | Test (TC-1325) |
| FR-350-AC-14 | A `replay-request` file that does not match `$defs.ReplayRequest` (an unknown member, a missing required member such as a `sources` entry's `identity`, a wrong JSON kind) is that vector's `tool-failure` with cause `corpus-decode` naming the JSON pointer of the defect, and not a QSL refusal. | Test (TC-1325) |
| FR-350-AC-15 | **RETIRED (source-byte integrity removed).** Withdrawn with no replacement. | Test (TC-1325), retired |
| FR-350-AC-16 | A `replay-request` file whose `package` holds `checked_package` for the package `orders`, the corpus `contract_version`, one `sources` entry and one `dependencies` entry `{identity: "lib/money", checked_package, sources}` decodes to a request whose package reference holds `package_id` `orders`, that one source, and one dependency entry with `identity` `lib/money` and its own `package_id` and sources, and the request holds no version or package-name member; the same file with `package` removed, with `package` set to the string `"orders"`, and with `contract_version` altered is each a `tool-failure` with cause `corpus-decode` and pointer `/package`, `/package` and `/package/contract_version` respectively. | Test (TC-1325) |
| FR-350-AC-17 | A `byte_provision` row whose `bytes_path` is `missing.bin` (absent from the vector directory), `unlisted.bin` (present but not listed as `kind: bytes`), `/etc/hostname` (absolute) or `../x.bin` (parent-traversing) is a `tool-failure` with cause `corpus-decode` and pointer `/byte_provision/0/bytes_path`, and no request is constructed; a listed `kind: bytes` path is read as raw bytes. | Test (TC-1325) |
| FR-350-AC-18 | A `stage_limits` map holding the key `depth` (no FR-255 setting of that name) is a `tool-failure` with cause `corpus-decode` and pointer `/stage_limits/depth`; the key `s1.input_bytes` with the value `"1024"` decodes to the entry `("s1.input_bytes", 1024)`. | Test (TC-1325) |

## Dependencies

- QSpec FR-311 (qualification behaviour), FR-336 and interface_019 (the
  corpus and its vectors).
- [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)
  (stage limits and catalog-coded refusals the comparison reads).
- [FR-351](FR-351-settle-each-capability-from-its-executed-vectors.md)
  settles each capability from these outcomes.
