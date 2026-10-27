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

The corpus is QSpec's (QSpec FR-336, interface_019), and its wire shape is
the closed `quire.conformance-corpus/v1` schema. QSpec owns each vector's
`id`, `capability_id`, `class`, `subject`, `applicability`, `input` and
`expected` members, and the primary capability entries under
`coverage.primary`. This requirement specifies how QSL executes and compares
them. A vector's `expected.disposition` is `value` or `refusal`; the `value` is
the subject operation's typed JSON result, while a refusal expectation
(including an expected resource-limit refusal) carries its `code`, `cause`
and optional `details`.

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
its `semantic_profiles` is in QSL's definition catalog. A vector SHALL run
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

### Corpus input adapters

The files are test-harness adapters owned by QSpec FR-336. The runner SHALL
map their contents to existing public operations and SHALL NOT add fields to
those operations' product request wires.

| Subject and operation | Required files | Existing contract |
| --- | --- | --- |
| `language` / `compile` | One or more `source` files, plus any existing `model` or `input` files required by that operation | FR-027 and the applicable input owner |
| `tool` / `format` | One `source` file | FR-003 formatter entry point |
| `runtime` / `run` | One `source` file, any existing model/library inputs required by FR-100, exactly one `call` JSON file, optionally one `population` file | `call` maps field-by-field to the typed `Call` owned by FR-100; `population` supplies its object context. |
| `runtime` / `replay` | Exactly one `replay-request` JSON file and the `bytes` files it names | Request fields map one-to-one to FR-071 `ReplayRequestWire`; FR-098 executes it and FR-072 owns the result. |

#### `call` input mapping

The `call` JSON file has exactly `function`, `arguments` and `accounting`,
as QSpec FR-336 `$defs.CallInput` specifies. Its mapping is:

| Corpus field | Typed API field | Mapping |
| --- | --- | --- |
| `function` | `Call.function` | Pass the FR-100 function identity string unchanged. |
| Each `arguments[i]` tagged `boolean` | `CallArgument.parameter`, `CallArgument.value` | Resolve the selected function's declared parameter at zero-based position `i`; use its exact declared name for `parameter` and construct `quire_exact::Value::Boolean(value)`. |
| Each `arguments[i]` tagged `integer` | `CallArgument.parameter`, `CallArgument.value` | Resolve the selected function's declared parameter at zero-based position `i`; use its exact declared name for `parameter`, parse the canonical decimal under FR-038 and construct `quire_exact::Value::Integer`, with no floating-point conversion. |
| Each `arguments[i]` tagged `reference` | `CallArgument.parameter`, `CallArgument.value` | Resolve the selected function's declared parameter at zero-based position `i`; use its exact declared name for `parameter` and construct the existing `ObjectReference` with the supplied universe, effective type and exact object identity. FR-100 admission still runs. |
| Each `arguments[i]` tagged `population` | `CallArgument.parameter`, `CallArgument.value` | Resolve the selected function's declared parameter at zero-based position `i`; use its exact declared name for `parameter`; resolve `name` to the matching admitted population entry in the `population` file, then use its FR-089 `PopulationId` as `quire_exact::Value::Population`. The adapter SHALL NOT mint an id. |
| `accounting` | `Call.accounting` | Parse each canonical decimal string as u64, then map the ten FR-100 `quire.value.accounting/v1` counters by their unchanged names and exact values. Reject overflow before constructing `Call`. Omitted counters use FR-100's defaults. |
| (no corpus field) | `Call.supplied` | Use `SuppliedLimits::default()` under FR-277; this corpus input does not override supplied-value helper ceilings. |

The runner SHALL preserve argument array order and pair `arguments[i]` with
the selected function's declared parameter at the same zero-based position
when constructing `CallArgument`. A value with no corresponding formal is an
input-adapter refusal, not a value to drop or reassign; a shorter array is a
prefix call, and FR-100 settles its remaining unbound parameters. FR-100 then
applies its declared-name binding and refusal precedence. The runner SHALL
NOT route tagged values through native-run/1's signed-i64 assignment encoding.
Population tagged values are corpus-harness references only: native-run/1
itself still has no Population argument wire encoding.

#### `population` object-context mapping

The `population` file is the FR-106 `quire.state.snapshot/v1` document. Its
`populations` array and each `objects` array retain authored order; an
object's existing `key` is its fixture name and its `fields` use FR-106's
closed tagged value grammar. The runner SHALL admit the complete document
under FR-106 and the selected model through the FR-084/FR-089 population
admission path. A tagged `population` call argument's `name` MUST equal
exactly one `populations[*].population` declaration identity in the admitted
document. The `PopulationId` comes from that admitted binding; the harness
file and tag never mint, decode or substitute one. A missing or repeated
name is an owning input refusal before S6a. The admitted object environment
is passed to FR-100 `run` as its `&ObjectEnvironment` context.

#### `replay-request` field mapping

A `replay-request` file SHALL validate against QSpec FR-336's
`$defs.ReplayRequest` before adaptation. It is corpus-v1 harness data, not a
new FR-071 product wire. Its 15 top-level members map to `ReplayRequestWire`
field-for-field:

| Corpus member | FR-071 type field | Adapter mapping |
| --- | --- | --- |
| `profile_selections` | `profile_selections` | Ordered `{profile, value}` `ProfileSelection` records. |
| `package_id` | `package_id` | FR-323 `package.package_id` digest record, split to the typed optional digest domain and digest text. |
| `source_digests` | `source_digests` | Ordered source records with the four FR-001 labels and digest domain/digest. |
| `dependencies` | `dependencies` | Ordered `{identity, package_id, sources}` records; values follow FR-323. |
| `replay_target` | `replay_target` | Request target with FR-323 target members and the request-only fields described below. |
| `source` | `source` | FR-323 `ReplaySource` `witness` or `input` arm. |
| `obligation_identity` | `obligation_identity` | Decode 64 lowercase hex characters to 32 bytes. |
| `backend` | `backend` | Carry the backend identity string unchanged. |
| `state_environment` | `state_environment` | Preserve ordered two-string pairs as stored. |
| `accounting_limits` | `accounting_limits` | Map all ten FR-255 scalar counters; parse canonical unsigned decimal strings to `u64`. |
| `stage_limits` | `stage_limits` | Preserve FR-255 setting-name keys; parse each canonical unsigned decimal value to `u64`, then use the existing closed setting decoder. |
| `supplied_limits` | `supplied_limits` | Map `null` to the typed default/absence input; otherwise parse the complete three-field FR-323 object to `SuppliedLimits`. |
| `object_environment` | `object_environment` | Map `null` or the FR-106 identity labels plus `sha256-jcs:<hex>` digest string to `ObjectEnvironmentRef`. |
| `declared_domains` | `declared_domains` | Map each structural `DeclaredDomain` to the existing `ProofBound` domain, optional kind, and finite bound. |
| `byte_provision` | `byte_provision` | Resolve each `bytes_path`, read raw bytes, and construct the typed `(digest_domain, digest, bytes)` entry. |

`source` accepts `{"kind":"witness","transcript":...}` or
`{"kind":"input","assignments":[...]}`, the FR-323 `ReplaySource`
Witness/Input arms. A witness's transcript is parsed by `Witness::parse`;
its decoded values are not duplicated in the input. An input assignment has
`parameter` (the 64-hex node id) and `value`; values use FR-336's closed
structural `WitnessValue` union and bind by parameter node id, never array
position. Other `ReplaySource` variants are outside this corpus adapter's
v1 replay input.

`replay_target.members` carries exactly one FR-323 target: a function's
qualified name, `frame_operation: {object, operation}`, or a state clause's
qualified name. The `RequestTargetWire` wrapper additionally requires
`arguments` (boolean), `occurrence_key` (null or `{node_id, role, ordinal}`)
and `over` (null or `{population, key}`). Each maps field-for-field; ordinal
is parsed from its canonical unsigned decimal string. FR-323 target
validation and FR-071 refusal precedence remain authoritative.

For scalar accounting limits, each of `integer_bits`, `decimal_digits`,
`scale_expansion`, `text_input_bytes`, `text_scalars`, `normalized_scalars`,
`unit_edges`, `value_occurrences`, `work_units`, and `result_units` is
required and parsed as `u64`. Stage-limit unknown names continue to refuse
through FR-255; the adapter does not discard them. Every structural object
and tagged value is closed, every tagged variant uses its FR-336 `kind`, and
unknown members are refused before constructing `ReplayRequestWire`.

The vector's `expected.value` for replay uses FR-323's result envelope
`results` member with exactly the single result corresponding to the request's
one obligation. The nested result is the FR-072 typed replay result, including
its Witness/Input arm and typed settlement; the adapter compares typed
members, not debug or display text. `expected.disposition: refusal` continues
to use FR-336's ordinary refusal record.

Each `byte_provision` row is exactly `{digest_domain, digest, bytes_path}`.
The path is relative to the vector directory and MUST resolve
to exactly one vector file listed as `kind: bytes`; absolute, missing,
unlisted and parent-traversing paths refuse before request construction. The
adapter reads raw bytes and supplies the tuple keyed by the row's digest.
The path never enters `ReplayRequestWire`; FR-071 digest, completeness and
content validation remain authoritative.
### Settlement

If executing a vector panics, returns an internal fault, or cannot read a
file the vector names, then its outcome SHALL be `tool-failure`, holding
the panic message, the fault or the I/O error. Otherwise the runner SHALL
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
| FR-350-AC-8 | A `run` vector maps every `call` field as the table states, including Boolean, Integer, Reference and named Population arguments, all ten accounting counters/defaults and FR-277 default supplied limits. A canonical accounting decimal above `u64::MAX` fails before Call construction; invalid tagged values retain their input-adapter refusal or tool-failure. | Test (TC-875) |
| FR-350-AC-9 | A `population` file is admitted under FR-106/FR-084, and each named Population argument resolves only to its actual admitted FR-089 binding. A missing or repeated population name refuses before S6a. Native-run/1 remains without a Population argument wire. | Test (TC-875) |
| FR-350-AC-10 | A `replay` vector maps every same-named request member to FR-071 `ReplayRequestWire`; each `byte_provision.bytes_path` resolves to exactly one listed sibling `bytes` file whose raw bytes supply the matching digest entry. The typed request carries no path and reaches FR-098; its result settles under FR-072. Invalid paths and requests retain their owning refusal or per-vector tool-failure. | Test (TC-875) |

## Dependencies

- QSpec FR-311 (qualification behaviour), FR-336 and interface_019 (the
  corpus and its vectors).
- [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)
  (stage limits and catalog-coded refusals the comparison reads).
- [FR-351](FR-351-settle-each-capability-from-its-executed-vectors.md)
  settles each capability from these outcomes.
