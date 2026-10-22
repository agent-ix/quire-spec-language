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
interface is built behind it. That implementation lives in `quire-eval`, in
the repository `agent-ix/quire-eval`, beside the interface
[FR-401](FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md)
hosts there; `qsl-eval` consumes it. The suite compares QSL's existing std
evaluation path with the `quire-eval` implementation until that std path is
retired. The two implementations must agree
on outcome, refusal, location, losses and accounting for shared inputs, and
the reference may be switched out only once the second implementation passes
the shared differential suite. This requirement defines that suite: which
inputs it runs, the record it compares, how an input bundle is encoded,
loaded and admitted, what is refused, how the harness runs and what counts
as a failure.

The suite runs one corpus of evaluator cases. A case is one checked
application: a QSL source unit and its library dependencies, a function of
that unit, completed arguments and limits. Each case runs through both
implementations with a fresh meter, and the two compared records must be
equal.

The case record, its loader and the compared record live beside the
evaluation interface, in the crate `quire-eval-corpus` in the repository
`agent-ix/quire-eval`, not in a QSL crate. The code generator (CG) depends on
it to write its generated-closure cases, and on `quire-eval` to record
expected evaluations once `quire-eval` carries its implementation,
so neither the corpus nor CG's expectations need any QSL entry beyond the
`qsl-replay` facade CG already uses. Turning a case into a checked input is
the implementation's job, through an admission contract the corpus crate
defines; QSL's harness supplies that admission for the std reference.

## Inputs

- A corpus bundle: a directory holding one file, `cases.jsonl`.
- The QSL-owned corpus: the checked-in bundle
  `tests/fixtures/evaluator-corpus/cases.jsonl` and the generated cases.
- Optionally, an external bundle named by `EVALUATOR_CORPUS_DIR`, such as the
  one CG's `make evaluator-corpus` writes.

## Outputs

- From `quire_eval_corpus::load`: every case record of a bundle, or one
  load refusal for the whole bundle.
- From a case admission: every case's checked input, or one admission
  refusal for the whole bundle.
- From the harness: a pass, or one report line per divergent case and a
  failing exit.

## Behavior

### The corpus crate

The crate `quire-eval-corpus` SHALL live in the repository
`agent-ix/quire-eval`, beside `quire-eval` and `quire-checked` (FR-401). It
is a `std` crate and forbids unsafe code. Its `[dependencies]` SHALL be drawn
only from `quire-eval`, `quire-semantic-value`, `quire-exact`,
`quire-canonical`, `serde` and `serde_json`, and SHALL name no crate of the
QSL repository. It SHALL evaluate nothing. Its feature `charge-log` enables
`quire-exact`'s `test-support` feature and nothing else, and, like that
feature, only a dev-dependency enables it.

It SHALL hold:

- `CaseRecord`, with `line()` (the record's JCS text followed by `\n`) and a
  constructor from its members;
- `load(dir) -> Result<Vec<CaseRecord>, LoadRefusal>`;
- the trait `CaseAdmission`, whose implementation turns a `CaseRecord` into
  that implementation's checked input (the type FR-401 lets an
  implementation supply) or an `AdmissionRefusal`;
- `ComparedRecord`, built from a run's `Result<Evaluation, CallFailure>`
  and its `quire_exact::Meter`, with its JCS text and the comparison below.

### JSON encoding

Every JSON text this requirement defines SHALL be produced by
`quire-canonical`, the one RFC 8785 (JCS) encoder (ADR-013 §2). Every
integer that is a `u64`, a `usize` or a kernel `Integer` (limits, counters,
charge sizes, `Incomplete` amounts, parameter indices, path steps, integer
values) SHALL be written as a JSON string holding its canonical decimal form
(no leading `+`, no leading zeros, `-` only for a negative value), so no JSON
number above 2^53 occurs.

### The case record

A bundle's `cases.jsonl` SHALL be UTF-8 JSON Lines: each line is one case
record, written as its JCS text followed by one `\n`, and the file holds no
other bytes. Records SHALL appear in strictly ascending UTF-8 byte order of
`case_id`.

A source object is a JSON object with exactly the members `identity` (an
object with exactly the four FR-001 labels `authority`, `identity`,
`revision_namespace` and `revision`, each a string, as QSL's
`SourceIdentity` holds them), `path` (the display path, a string) and `text`
(the unit's source, a string).

A case record SHALL be a JSON object with exactly these members:

| Member | Content |
| --- | --- |
| `format` | the string `quire.evaluator-case/v1` |
| `case_id` | a string of 1 to 128 bytes drawn from `a`-`z`, `0`-`9`, `.`, `_`, `-` and `/`, unique in the bundle |
| `source` | a source object: the unit under test |
| `dependencies` | an array of objects with exactly `identity` (the library identity string) and `source` (a source object), strictly ascending in UTF-8 byte order of `identity`; empty for a unit with no library dependency |
| `package_id` | the `package_id` QSL's compile of `source` with `dependencies` declares, in the digest-record shape of `quire.checked-package/v2` |
| `function` | the declared name of a function of the unit that is callable by name |
| `arguments` | an array of FR-071 `WitnessValue`s in their wire shape, one per declared parameter, in declared parameter order |
| `object_environment` | `null` or a JSON object |
| `limits` | an object mapping `quire.value.accounting/v1` counter names to canonical decimal strings; a counter with no entry runs at its published default |
| `generated_bodies` | an array of declared function names of the unit, strictly ascending and unique: the callables whose generated bodies were registered when `expected` was recorded; empty when none were |
| `expected` | `null`, or a compared record |

`expected` `null` means that no expectation was recorded for the case: the
harness then compares the two implementations only. A non-null `expected`
is the compared record of one run of the case through a `quire-eval`
implementation, with the bodies `generated_bodies` names registered and
every other callable interpreted.

The `package_id` binds the checked input. The emitted package's source
references carry each source's `authority`, `identity` and the digest of its
bytes, so a change to any of those, in `source` or in a dependency's source,
changes `package_id`, and a case admits only to the closure its writer
compiled. `revision_namespace` and `revision` do not enter the package and do
not bind `package_id`; they identify the source for its writer and for
`compile_package`'s input only.

Version 1 carries no object environment and no domain package: a case runs
over an environment with unresolved references, as a replay request without
`object_environment` does (FR-098), a non-null `object_environment` refuses
`unsupported_member`, and a unit whose `model` selection names a domain
package refuses `compile_refused` with the compile's `missing_import`
refusal.

### Load refusals

`load` SHALL admit a bundle whole or refuse it whole. It applies the bundle
checks first, then checks each line in file order, applying the line checks
in the table's order, and returns the first refusal. A bundle refusal names
no line and no member. A line refusal names the 1-based line number and the
`case_id` when that member decoded; a member refusal also names the member's
JSON pointer, and `malformed_line` and `noncanonical_line` name none.

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
| 11 | `dependency_order` | member | `dependencies` is not strictly ascending by `identity`, or `generated_bodies` is not strictly ascending |
| 12 | `invalid_package_id` | member | `package_id` is not a digest record of the `quire.checked-package/v2` shape |
| 13 | `malformed_expected` | member | `expected` is neither `null` nor a compared record of the form below |

A file holding only `\n` is non-empty and refuses `malformed_line` at line 1.

### Admission refusals

A `CaseAdmission` implementation SHALL admit every loaded case before any
case is evaluated, and SHALL refuse the whole run on the first case, in key
order ("The corpus"), that it cannot admit, naming the case's bundle,
`case_id` and the member's JSON pointer:

| Order | Code | Refused when |
| --- | --- | --- |
| 1 | `compile_refused` | the compile of `source` with `dependencies` refuses; the refusal carries the compile's catalog code; pointer `/source` |
| 2 | `package_mismatch` | the compiled `package_id` differs from `package_id` |
| 3 | `unknown_function` | `function` names no function of the compiled unit that is callable by name |
| 4 | `unknown_generated_body` | a `generated_bodies` entry names no function of the compiled unit |
| 5 | `malformed_argument` | an `arguments` entry is not an FR-071 `WitnessValue` wire shape |

QSL's harness SHALL implement `CaseAdmission` for the std reference with
`qsl_replay::compile_package`, building the dependency input from
`dependencies` as FR-098 builds it from a replay request's entries (each
entry's `identity`, and its source's labels, path and text), with no domain
packages and the published stage limits, and converting arguments by
FR-098's `WitnessValue` conversion. A converted argument that call admission
then refuses (arity, kind, domain) is not an admission refusal: it is the
case's result, `CallFailure::Input`, compared like any other result.

Admission happens once per case, in the runner. Both implementations
evaluate the same admitted `CheckedPackage` in the runner's process: the
std reference through `CheckedPackageEvaluation`, and the `no_std`
implementation through `qsl-eval`'s `quire-eval` entry over the package's
checked graph
([FR-403](FR-403-implement-the-no-std-evaluator-in-quire-eval.md)). No byte
form carries an evaluable checked graph, so the checked input never leaves
that process. An admitted case for which either implementation produces no
record is a divergence.

### The compared record

The compared record of one run SHALL be a JSON object with exactly the
members `result`, `consumed` and `charges`, built from the run's
`Result<Evaluation, CallFailure>` (the `quire-eval` types) and its meter.
Each form below is defined from the public members of the named types;
where a type has no public member this record names, the record carries no
payload for it, as stated.

`result` SHALL be one of:

- `{"kind": "evaluation", "outcome": <outcome>, "location": <location or
  null>, "losses": [<loss>, ...]}`, with `location` from `Evaluation.location`
  and `losses` from `Evaluation.losses` in evaluation order.
- `{"kind": "call_failure", "arm": <arm>, ...}`, with `arm` the
  `CallFailure` variant in snake case and these payload members: `input`
  carries `code` and `cause`, the `InputRefusal`'s catalog code and cause
  strings; `fault` carries `stage` and `invariant`, the `InternalFault`'s;
  `storage` carries `parameter`; `cancelled`, `supplied` and
  `unsupported_helper` carry no payload member. `cancelled` is only a
  pre-charge Begin failure. A charge that a cancelled handle denies is
  recorded as the evaluation it returns, the `incomplete` outcome at that
  charge (FR-262).

`<outcome>` SHALL be one of:

| Arm | Members besides `arm` | Source |
| --- | --- | --- |
| `completed` | `value`: `{"boolean": <true or false>}` for a `Value::Boolean` and `{"integer": <decimal string>}` for a `Value::Integer` | `Outcome::Completed` |
| `incomplete` | `limit_kind`, `charge_point` (their `as_str` names), `limit`, `consumed`, `next_charge` | `Outcome::Incomplete`'s `Incomplete` |
| `refused` | `code`, `cause`: the kernel `Refusal`'s `code()` and `cause()`, each `null` where it returns none | `Outcome::Refused` |
| `undefined` | `reason`: the kernel `Undefined` variant in snake case | `Outcome::Undefined` |
| `family_refused` | `code`, `cause`: the `CatalogCoded` cause's `catalog_code()` code and cause | `FamilyResult::Refused` |
| `family_undefined` | `reason`, `fields`: the `UndefinedCoded` cause's `undefined_record()` reason and fields (an object of strings) | `FamilyResult::Undefined` |

Version 1 encodes `Boolean` and `Integer` results only. A run that completes
with a value of any other kind has no compared record: the builder refuses it
as `unencodable_value`, naming the value's kind, and the harness reports
that case as failing.

`<location>` SHALL be `{"origin": <origin>, "path": [...]}`: `origin` is
`{"kind": <Origin variant in snake case>}` plus that variant's fields under
their `quire-semantic-value` field names (a `String` as a JSON string, a
`usize` as a decimal string, and `ModelOperationClause`'s `kind`, an
`OperationClauseKind`, as its variant in snake case: `requires` or
`ensures`), and `path` is `Location::path()`.

`<loss>` SHALL be `{"location": <location>, "kind": <ValueLoss variant in
snake case>}` (`decimal`, `ieee_exact` or `ieee_flags`). Version 1 compares a
loss's kind and location, not its payload, because no encoding of the loss
payloads exists.

`consumed` SHALL map each of the ten `quire.value.accounting/v1` counter
names (`integer_bits`, `decimal_digits`, `scale_expansion`,
`text_input_bytes`, `text_scalars`, `normalized_scalars`, `unit_edges`,
`value_occurrences`, `work_units`, `result_units`) to `Meter::consumed` for
that counter after the run.

`charges` SHALL be the run's ordered charge log or `null`. The kernel meter
keeps a charge log only under `quire-exact`'s `test-support` feature, which
`quire-eval-corpus`'s `charge-log` feature forwards and only a
dev-dependency enables, and holds at most its capacity of 4096 admitted
charges (`CHARGE_LOG_CAPACITY`). Where the run's meter keeps a log and the
log is not truncated, `charges` is the list of admitted charges in order,
each `{"point": <charge-point name>, "sizes": {<counter>: <amount>, ...}}`,
with one `sizes` member for each size counter `AdmittedCharge` records for
that charge. Otherwise `charges` is `null`: in a build without the log, and
in a run whose log was truncated. A case of more than 4096 charges, such as
FR-262-AC-1's 100,000 calls, is therefore compared on `result` and
`consumed` only.

Two compared records SHALL be equal exactly when their JCS bytes are equal.
A difference SHALL be reported as the RFC 6901 JSON pointer of the first
member, in JCS member order, at which the two records differ, with both
values at that pointer.

### The corpus

The QSL-owned corpus SHALL be:

1. The checked-in bundle `tests/fixtures/evaluator-corpus/cases.jsonl`,
   holding exactly these cases (each a `function` call; a vector written as
   an expression becomes a nullary function whose body is that expression),
   each with `expected` `null`:
   - `fr262/ac1/count-100000`, `fr262/ac1/count-100000-short` and
     `fr262/ac1/count-100000-exact`: FR-262-AC-1's `count` called with
     `100000`, at a `work_units` limit that fits, at one unit below the
     reference spend and at exactly the reference spend;
   - `fr262/ac5/two-arguments` and `fr262/ac5/literal`: FR-262-AC-5's source
     call with two argument applications, and a standalone literal;
   - `fr402/count-3`, `fr402/count-3-short` and `fr402/count-3-boolean`: the
     `count` program called with `3` at default limits, at one unit below
     its reference spend, and with the argument `WitnessValue::Boolean(true)`;
   - `tc069/01` to `tc069/14`: TC-069's fourteen signed `div`, `rem`, `+`,
     `-`, `*` and negation vectors, in the order
     `tests/runtime_evaluation_cases/scalars.rs` lists them.
   A case joins the corpus by adding its record to this file, in `case_id`
   order.
2. The generated cases, from QSL's own deterministic generator (no
   third-party random or property-testing dependency). The generator is
   SplitMix64; case `i` (0 to 255) draws from the stream seeded with
   `0x5153_4C37 + i`, and its `case_id` is `gen/` followed by `i` as three
   decimal digits (`gen/000` to `gen/255`). Each case is a unit of one to
   three functions, named `f0`, `f1` and `f2` in order; a function calls only higher-numbered functions, so calls are
   acyclic and need no `decreases` measure. Each function has one to three
   parameters, each `Int[-1000, 1000]` or `Boolean`, and a declared result
   `Integer` or `Boolean`. An `Integer` body combines parameters and literals
   in `[-1000, 1000]` with `+`, `-`, `*`, and `div` and `rem` by a nonzero
   literal from 1 to 9, to an expression depth of at most 4; a `Boolean` body
   combines comparisons of such expressions and `Boolean` parameters with
   `and`, `or` and `not`; either may be an `if`-`then`-`else` on a `Boolean`
   expression. A call's argument to an `Int[-1000, 1000]` parameter is a
   parameter of that type or a literal in its domain, and to a `Boolean`
   parameter a `Boolean` parameter or literal. The case calls `f0` with
   arguments drawn from its parameters' domains; its `limits` is empty for an
   even `i` and sets `work_units` to a draw from 0 to 2000 for an odd `i`.
   Its source identity labels are `authority` `qsl`, `identity`
   `generated/<i>`, `revision_namespace` `none` and `revision` `0`, and it has
   no dependencies. The same seed and count SHALL yield byte-identical case
   records. A generated unit that the admission refuses fails the run as a
   generator defect; it is not skipped.

An external bundle named by `EVALUATOR_CORPUS_DIR` SHALL be loaded with
`load`, admitted and run with the QSL-owned corpus.

One run holds up to three bundles, named `checked-in`, `generated` and
`external`. A case's key in a run is the pair of its bundle name and its
`case_id`; a `case_id` is unique within its bundle, and the same `case_id`
in two bundles names two cases. The harness admits, runs, pairs and reports
cases by key, in the order `checked-in`, `generated`, `external`, and by
`case_id` within a bundle. CG's generated-closure
bundle joins the corpus this way.

FR-262-AC-2 (a 100,000-long `List` value), AC-3, AC-4, AC-6, AC-7, AC-8,
AC-10 and AC-11 (registry results, registered generated bodies, stepping,
cancellation, resumption and child-request validation) cannot be written as a version-1 case, because a case's result
must be `Boolean` or `Integer`, and a case registers no QSL-side body and
cancels nothing. The suite compares them by running those criteria's tests
against both implementations: each test SHALL pass against the std reference and
against the `no_std` implementation, and each asserts the reference's result
and charge sequence.

### The harness and its pass criterion

The std reference and the `no_std` implementation SHALL run in one test
build of the harness's runner; `quire-eval` has no `std` feature, so one
build holds both. The runner evaluates every admitted case once with each
implementation, each run on its own thread with a 512 KiB stack and a fresh
meter at the case's limits, with every callable interpreted, and writes one
compared record per key and implementation. The runner is a Cargo test
target, so its build is a test build in which the dev-dependency on
`quire-eval-corpus` enables `charge-log`, and both runs keep the kernel
charge log. The comparator then pairs the records by key. That `quire-eval`
builds without `std` is FR-401-AC-1's `thumbv7em-none-eabi` gate, not this
run.

A case diverges when its two records differ, when either implementation produced no
record for it, or when its `expected` is not `null` and the std record
differs from it in `result` or `consumed`, or in `charges` when neither is
`null`. A run that panics, overflows the host stack or refuses
`unencodable_value` in either implementation's run produces no record for
that implementation.

The harness SHALL run every case even after a divergence, and SHALL report
each divergent case on one line: its bundle and `case_id`, which pair
differed (`std/no_std` or `expected/std`) and the first differing JSON
pointer with both values, or the implementation that produced no record and why.

The pass criterion SHALL be zero divergences. The harness SHALL accept no
option, list or configuration that tolerates, skips or excludes a case.

### Where the harness runs

The harness SHALL be the `xtask` command `evaluator-differential`, run by
`make evaluator-differential`, which passes `EVALUATOR_CORPUS_DIR` through
when it is set. The command evaluates no case itself: it runs the runner's
test build, collects its std and `no_std` compared records and runs
the comparator.

While QSL ships both the std reference and the `no_std` implementation,
`make ci` SHALL run `evaluator-differential` over the QSL-owned corpus.
`make ci` does not run CG's bundle, because QSL never builds CG (ADR-011
FB-11). A run over CG's bundle uses a bundle that CG's
`make evaluator-corpus` wrote, and
`make evaluator-differential EVALUATOR_CORPUS_DIR=<dir>` runs it. The corpus
crate's loader, refusals and compared-record forms need no second
implementation; their criteria run in `quire-eval-corpus`'s own tests and
the admission criteria in QSL's `cargo test --workspace`.

### After the switch

This suite's zero-divergence result is the switch criterion of
[FR-405](FR-405-retire-the-std-reference-evaluator-for-quire-eval.md), and
the switch deletes the std reference. Until the switch, every criterion
below holds as written. After it, the corpus, its loader, admission and
compared record stay, the harness runs one implementation as
`evaluator-regression` (FR-405 "The corpus after the switch"), and the
criteria that name the std implementation hold in this form:

- AC-6: the records are `quire-eval`'s, each equal to the case's recorded
  `expected`, with the same values.
- AC-7: the pair is `expected/quire-eval`, and a case with `expected`
  `null` reports `missing_expected`. A comparator fed expectations and a
  `quire-eval` record set that differ only in one `charges` entry's `sizes`
  reports pair `expected/quire-eval` and a pointer under `/charges`; a
  record set missing one `case_id` reports that case and that `quire-eval`
  produced no record (FR-405-AC-8).
- AC-9: `make ci` runs `evaluator-regression` over the QSL-owned corpus with
  zero divergences; a comparator fed a `quire-eval` record set that lacks
  an admitted case reports that case as a divergence; and FR-262-AC-2,
  AC-3, AC-4, AC-6, AC-7 and AC-8's tests run once, on `quire-eval`
  (FR-405-AC-8).
- AC-10: `make evaluator-regression EVALUATOR_CORPUS_DIR=<dir>` runs CG's
  bundle, and every record needs a non-null `expected` (FR-405-AC-9).

### What CG writes

CG SHALL write its bundle with `quire-eval-corpus`: each case's source,
source identity labels and library dependencies as it compiles them, the
`package_id` that `qsl_replay::compile_package` returns for them (called
from CG's `replay` module, FR-060 T12-A), and `expected` `null`, with
`generated_bodies` empty while `quire-eval` carries no implementation of the
interface. Once it carries one, CG MAY instead record a non-null `expected`
from one run of the case through `quire-eval`'s implementation, with its generated bodies registered and
named in `generated_bodies`; the harness then checks that record against the
std reference.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-402-AC-1 | `quire-eval-corpus`'s manifest names no crate of the QSL repository in any dependency table, its normal dependencies are drawn only from the stated list, and its `charge-log` feature enables `quire-exact/test-support` and nothing else. | Inspection |
| FR-402-AC-2 | A `CaseRecord` for `fr402/count-3` (FR-262-AC-1's `count`, argument `3`, default limits, no dependencies, `package_id` from `compile_package`) has a `line()` that is its `quire-canonical` text followed by `\n`, and `load` of a bundle holding that line returns the record unchanged. QSL's std admission admits it; the same record with a changed `source.identity.authority`, or a changed `source.identity.identity`, and the old `package_id` refuses `package_mismatch`; the same record with a changed `source.identity.revision` and the old `package_id` admits. | Test (TC-1112) |
| FR-402-AC-3 | A case whose `source` imports a library supplied in `dependencies`, with `compile_package`'s `package_id`, loads and admits; with the dependency entry removed, admission refuses `compile_refused` carrying `missing_import`; with two entries out of `identity` order, `load` refuses `dependency_order`. | Test (TC-1112) |
| FR-402-AC-4 | Each bundle below makes `load` refuse the whole bundle with the stated code. Bundle refusals, naming no line: no `cases.jsonl` (`missing_bundle`); a zero-byte file (`empty_bundle`). Line refusals at line 1 with no pointer: a file holding only `\n`, and AC-2's line without its `\n` (`malformed_line`); AC-2's line with members reordered (`noncanonical_line`). Member refusals at line 1 naming the pointer, each one mutation of AC-2's record: `format` `quire.evaluator-case/v2` (`unknown_format`, `/format`); an extra member `note` (`member_set`, `/note`); `object_environment` `true` (`member_set`, `/object_environment`); `case_id` `A` (`invalid_case_id`, `/case_id`); `limits` `{"depth": "1"}` and `limits` `{"work_units": "-1"}` (`invalid_limit`, `/limits/depth` and `/limits/work_units`); `object_environment` `{}` (`unsupported_member`, `/object_environment`); `generated_bodies` `["b", "a"]` (`dependency_order`, `/generated_bodies`); `package_id` `{"algorithm": "sha256", "digest": "zz"}` (`invalid_package_id`, `/package_id`); `expected` `{}` (`malformed_expected`, `/expected`). A two-line bundle repeating AC-2's line refuses `case_order` at line 2. | Test (TC-1112) |
| FR-402-AC-5 | QSL's std admission refuses each bundle below whole, before any case is evaluated, naming the bundle, `case_id` and pointer: a source that does not parse (`compile_refused`, `/source`, with the compile's catalog code); one changed digest byte of `package_id` (`package_mismatch`, `/package_id`); `function` `nope` (`unknown_function`, `/function`); `generated_bodies` `["nope"]` (`unknown_generated_body`, `/generated_bodies/0`); an argument `{"nope": 1}` (`malformed_argument`, `/arguments/0`). | Test (TC-1112) |
| FR-402-AC-6 | The std implementation's compared record for `fr402/count-3` has `result.outcome` `completed` with `value` `{"integer": "3"}`, a `charges` list with exactly four entries whose `point` is `function.call`, and `consumed` with all ten counters as decimal strings. For `fr402/count-3-short` it has `outcome` `incomplete` with `limit_kind` `work_units` and no `charges` entry for the denied charge. For `fr402/count-3-boolean` it has `result.kind` `call_failure`, `arm` `input` and the `WrongValueKind` code. For `fr262/ac1/count-100000` it has `charges` `null` and value `{"integer": "100000"}`. A run completing with a `Value::Text` is refused `unencodable_value` naming the kind `text`. | Test (TC-1112) |
| FR-402-AC-7 | A harness run over a bundle holding `fr402/count-3` with an `expected` whose `consumed.work_units` is one less than the std implementation's record's fails, reports that `case_id` with pair `expected/std` and pointer `/consumed/work_units`, and still runs and reports every other case; with the std implementation's own record as `expected` that case passes; with `expected` `null` it is compared `std/no_std` only. A comparator fed two record sets that differ only in one `charges` entry's `sizes` reports pair `std/no_std` and a pointer under `/charges`; a record set missing one `case_id` reports that case and the implementation that produced no record. | Test (TC-1112) |
| FR-402-AC-8 | The checked-in bundle loads, admits and holds exactly the `case_id`s the corpus list names; the generator, run twice for count 256, produces byte-identical lines with `case_id`s `gen/000` to `gen/255` that all load and admit; a run whose external bundle reuses the `case_id` `fr402/count-3` runs, pairs and reports it as a second case, keyed by bundle `external`; and the QSL workspace's `Cargo.lock` gains no random-number or property-testing crate for the generator. | Test (TC-1112) |
| FR-402-AC-9 | `make ci` runs `evaluator-differential` over the QSL-owned corpus; the command runs the runner's test build, which admits every case once and produces a std and a `no_std` record for every admitted case, both records carry non-null `charges` for `fr402/count-3`, and the run reports zero divergences; a comparator fed a `no_std` record set that lacks an admitted case reports that case as a divergence; FR-262-AC-2, AC-3, AC-4, AC-6, AC-7 and AC-8's tests pass against both implementations; and the harness's command line, environment and configuration offer no setting that tolerates, skips or excludes a case. | Test (TC-1112) and Inspection |
| FR-402-AC-10 | Given a bundle that CG's `make evaluator-corpus` writes, `make evaluator-differential EVALUATOR_CORPUS_DIR=<dir>` loads and admits every record and reports zero divergences, and every record whose `expected` is non-null passes its `expected/std` check. | Test (TC-1112) |

## Dependencies

- [FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md): the interface
  both implementations serve, its charge rules and its programs.
- [FR-401](FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md):
  `quire-eval`, its repository, `Evaluation`, `CallFailure` and the
  implementation-supplied checked input.
- [FR-060](FR-060-check-qsl-api-surface-boundary.md) T12-A: CG's
  `compile_package` call stays in its `replay` module; the corpus crates are
  outside the rule.
- [FR-098](FR-098-execute-a-replay-request.md) and FR-071: the dependency
  input built from entries, the `WitnessValue` wire shape and its
  conversion, and the run over an environment with unresolved references.
- [AD-001](../assurance/AD-001-shared-evaluation-core-interface.md): the
  slices that add the `no_std` implementation and switch the default.
- agent-ix/quire-contract-codegen FR-044 (IR-591): CG writes its bundle with
  `quire-eval-corpus` and records expectations through `quire-eval`. FR-044
  as written differs in the format name (`quire.generated-closure-evaluation/v1`,
  which `load` refuses as `unknown_format`), in requiring object
  environments (version 1 refuses a non-null one), and in taking expected
  evaluations from QSL's std reference rather than from a `quire-eval`
  implementation; CG resolves these.
