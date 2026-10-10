---
id: FR-109
title: "Run a selected state clause through the spine and report a typed disposition"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-023
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-028
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-301
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-267
    type: traces_to
---
# FR-109: Run a selected state clause through the spine and report a typed disposition

## Description

QSL SHALL provide `qsl_replay::spine::run_clause`, a layer-6 entry beside
FR-100's `qsl_replay::spine::run`. It compiles a `1-draft` unit through the
spine (S1 to S4, the same `spine::compile` CLI `compile` and FR-100 use),
selects one state clause, or one Boolean function run as a claim, by name,
admits the supplied observations (FR-106), evaluates through S6a (FR-107 or
`CheckedPackage::call`) and returns one `ClauseRunReport`: a typed
disposition, the compiled `package_id` and the usage. It is the spine replacement
for native-run/1 clause execution (ADR-011 §5, the paragraph that says it has
no spine equivalent before M-6c) and for FR-023's in-process `execute`.

FR-100 already owns the spine run's argument binding and its S6a outcome
mapping. This requirement reuses both by reference and adds only object
arguments, observations and the claim reading of a Boolean result.

AC-7's `UndefinedEvaluation{cause}` record is not implemented. The I3
extracted-source input is `ClauseRunSource::Extracted`, behind
`qsl-replay`'s `quire-extraction` feature.

AC-8 specifies the CG consumer boundary owned by Linear QSL-688 and IR-514;
it does not claim implementation of CG's external receipt verification.

## Inputs

A `ClauseRunRequest`:

- the unit's source bytes and its four FR-001 labels, or an I3 extracted
  source (ADR-011 I3, `qsl-source`, feature `quire-extraction`) with its
  original document identity;
- FR-056's package input (domain packages by canonical package identity) and
  FR-099's dependency input;
- the snapshot and invocation provisions (FR-106);
- a selection: `Clause(ClauseSelection)` (FR-106), `Frame { operation,
  invocation }` (FR-115), or `Function { name,
  arguments, snapshot }`. `name` follows FR-100's `function` rule.
  `arguments` are FR-100's `{parameter, value}` pairs, one per parameter, in
  any order, whose `value` is FR-100's canonical integer, or
  `{"reference": {"population": ..., "key": ...}}` for a parameter of a
  model object type, resolved in the current snapshot `snapshot` names;
- an optional expected `package_id`;
- `SpineLimits`, `ObservationLimits` and the evaluation meter budget
  (FR-100's `work_units`).

## Outputs

`Result<ClauseRunReport, ClauseRunRefusal>`. `ClauseRunRefusal` is only for a
request that cannot be formed (such as an empty source); every compile,
selection, admission or evaluation result is a report.

`ClauseRunReport` holds:

- `disposition`: `stage` (`compile`, `select`, `admit` or `evaluate`),
  `category` (`success`, `violation`, `undefined`, `refusal`,
  `unsupported`, `incomplete` or `internal-failure`), `truth` (for
  `success`, and for a `violation` whose clause evaluated `false`), the
  `UndefinedEvaluation{cause}` of an `undefined` disposition, whose clause
  evaluated undefined, with `cause` in FR-100's
  undefined `reason` spelling, and, for every other category, the one
  record: for `compile`, `select` and `admit` a `RefusalRecord` (code,
  cause, locus); for an `evaluate` refusal, unsupported or incomplete, FR-100's
  `outcome` member for that S6a outcome; for `evaluate` `internal-failure`, the `InternalFault`'s stage
  and invariant, as FR-100's internal-failure section gives them;
- `package_id`: the compiled package's identity, when compile completed;
- usage: the admission work and the evaluation meter charges, separately.

The report's `package_id` identifies the compiled QSL package. The report
does not authenticate generated Rust, its source map or coverage-producer
execution. [FR-267](FR-267-write-run-results-as-native-run-result-2.md)'s CG
source-identity boundary requires a separate CG-owned producer/artifact
binding receipt when CG consumes the serialized result.

## Behavior

- The entry SHALL compile first. If the compile refuses or hits a limit, then
  the entry SHALL report stage `compile` with that code (for example
  `missing_import`/`missing-selection` for a model selection whose package is
  not supplied) and SHALL admit and evaluate nothing.
- If the unit is an I3 extracted source whose declared language is not
  `ix:native`, then the entry SHALL report stage `compile`, category
  `refusal`, `unknown_language`, and SHALL compile, admit and evaluate nothing. This is the
  refusal the CLI's extracted-clause join gives the same fence.
- If an expected `package_id` is given and the recompiled one differs, then
  the entry SHALL report stage `compile`, category `refusal`,
  `stale_dependency`, naming both identities, with no admission or
  evaluation (FR-098's stale package rule).
- The entry SHALL resolve a `Clause` or `Function` selection's name in the
  compiled package's one name table of state clauses and functions. A
  `Frame` selection names an operation, not a clause or function, and
  resolves as FR-115 states. FR-104 refuses a clause whose
  name equals another clause's or a function's, so a name resolves to at
  most one declaration. This is the same one name lookup after checking that
  FR-100's `spine::run` makes (ADR-011 §5).
- If a `Clause` selection's name resolves to no state clause, or a
  `Function` selection's name resolves to no function, then the entry SHALL
  report stage `select`, `missing_declaration`/`missing-name`.
- If a `Function` selection's function does not declare a `Boolean` result,
  then the entry SHALL report stage `select`, `ill_typed`/`type-mismatch`,
  before any call. `run_clause` runs a function as a claim; FR-100's `run`
  runs any value.
- If admission (FR-106) fails, then the entry SHALL report stage `admit` with
  its category and record. S6a runs only after admission succeeds, and runs
  exactly once.
- For `Function`, admission SHALL admit the one current snapshot, bind the
  arguments by FR-100's argument rules (with FR-100's refusals, reported at
  stage `admit`) and resolve each object argument to an object of that
  snapshot, refusing an unresolved one `invalid_runtime_input`/
  `wrong-role-mapping`; evaluation SHALL be `CheckedPackage::call` with that
  snapshot's `ObjectEnvironment`.
- The entry SHALL map `Completed(true)` to `success` (exit 0) and
  `Completed(false)` to `violation` (exit 10), as QSpec FR-301 does for a
  claim, with `truth` set.
- When a clause evaluates undefined (the kernel `Outcome::Undefined(u)` or
  a family's `FamilyResult::Undefined(u)`), the entry SHALL report stage
  `evaluate`, category `undefined` (FR-285) with
  `UndefinedEvaluation{cause}`, and exit 10 (ADR-018 RU-5, QSpec FR-301).
- The entry SHALL map every other S6a outcome other than FR-100's internal
  failures, kernel or family, by FR-100's outcome mapping, to FR-100's
  `outcome` member and FR-100's exit status, category `refusal`,
  `unsupported` or `incomplete` by its ADR-013 O-16 category. It restates none of FR-100's
  rows.
- If the S6a outcome is one FR-100 handles as an internal failure (the
  kernel `Refusal::CheckedInvariant`, or `CallFailure::Fault`), then the
  entry SHALL report stage `evaluate`, category `internal-failure`, with the
  `InternalFault` FR-100's internal-failure section names, and FR-100's
  internal-failure exit status.
- The clause run's exit code SHALL be FR-285's exit code
  (`Category::exit_code`) of the disposition's category: 0 and 10 as above,
  an undefined evaluation included; FR-100's exit status for an `evaluate`
  refusal or incomplete; FR-100's internal-failure exit status for
  `evaluate` `internal-failure`; refusal (20) for a `select` result
  (`missing_declaration`, `ill_typed`); the category of the code for a
  `compile` refusal, an `admit` argument refusal or an `admit` refusal
  record (20, 21 for an unsupported code such as
  `unknown_required_feature`, 22 for an incomplete code); and incomplete
  (22) for an `admit` incomplete result.
- Each call SHALL build fresh admission and evaluation meters from the
  request's limits. The entry SHALL read no path, environment variable,
  clock or search location: every byte arrives in the request.
- `run_clause` SHALL NOT be reachable by CG: FR-060's T12-A rule already
  refuses any CG reference into `qsl_replay::spine`. Like FR-100's `run`, no
  public item of `run_clause`'s signature names a `qsl_eval` path
  (FR-100-AC-8).
- A consumer SHALL NOT substitute the report's `package_id`, including a
  match against the request's expected `package_id`, for the external
  generated-source, source-map and producer-execution authentication required
  by [FR-267](FR-267-write-run-results-as-native-run-result-2.md).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-109-AC-1 | The healthy-parent request reports stage `evaluate`, `success`, `truth: true`, exit code 0, with the compiled `package_id`; violating-parent reports `violation`, `truth: false`, exit 10. | Test |
| FR-109-AC-2 | missing-model (no package supplied) reports stage `compile`, `refusal`, `missing_import`/`missing-selection`, exit 20, with no `package_id`; an expected `package_id` of another unit reports stage `compile`, `stale_dependency`, naming both; a `Clause` selection naming `Absent`, and one naming the function `sameIdentity`, each report stage `select`, `missing_declaration`/`missing-name`. | Test |
| FR-109-AC-3 | dangling-parent reports stage `admit`, `refusal`, `dangling_reference`, exit 20; incomplete-population reports stage `admit`, `incomplete`, `incomplete_population`, exit 22; exhausted-work (budget zero) reports stage `evaluate`, `incomplete`, FR-100's `{"kind": "incomplete", "limit": "work_units"}`, exit 22; none carries `truth`. | Test |
| FR-109-AC-4 | A `Function` selection of `sameIdentity` with arguments `{b: child, a: root}` (given in that order) over the distinct-identities snapshot reports `violation`, `truth: false`; with `a` = `b` = `child`, `success`; with `b` naming `ghost`, stage `admit`, `invalid_runtime_input`/`wrong-role-mapping`; with an argument naming `c`, stage `admit`, FR-100's refusal for an unknown parameter; a function returning `Integer` reports stage `select`, `ill_typed`/`type-mismatch`, before any call. | Test |
| FR-109-AC-5 | Running one request twice gives equal reports, including usage. For each S6a outcome other than `Completed`, the report's `outcome` member and exit code equal what FR-100's mapping gives for the same outcome (checked over the outcomes FR-100-AC-9 constructs); for the kernel `CheckedInvariant` and a `CallFailure::Fault`, which FR-100 handles as an internal failure, the report is stage `evaluate`, category `internal-failure`, carrying the fault's stage and invariant, with no `outcome` member and FR-100's internal-failure exit status. | Test |
| FR-109-AC-6 | The healthy-parent request whose unit is an I3 extracted source reports `success`, exit 0, with the `package_id` its extracted body compiles to; violating-parent over the same source reports `violation`, exit 10; missing-model over the same source reports stage `compile`, `refusal`, `missing_import`/`missing-selection`, exit 20; the same unit in an `ix:formal` fence reports stage `compile`, `refusal`, `unknown_language`, exit 20, with no `package_id`. | Test |
| FR-109-AC-7 | The step 1 unit with the clause `invariant Ratio using v on Config::ConfigVersion at current { 6 / (self.versionNumber - self.versionNumber) >= 0 }` added, run over healthy-parent selecting `Ratio`, reports stage `evaluate`, category `undefined`, `UndefinedEvaluation{cause: division-by-zero}`, no `truth`, exit 10. | Test |
| FR-109-AC-8 | Given a successful clause run with the expected compiled `package_id`, CG consumption of its serialized result still refuses campaign binding when its external receipt is missing or its generated-source or source-map association is mismatched, as FR-267-AC-4 and FR-267-AC-6 require; the QSL report retains its original success disposition and package identity. | Test |

## Dependencies

- FR-100 (the spine run's argument binding, outcome mapping, internal-failure
  handling and exit statuses), FR-106, FR-107 (admission and evaluation), FR-099 and FR-027
  (the spine compile), FR-098 (the stale `package_id` rule), FR-060 (the CG
  boundary).
- QSpec FR-301 (exit codes); ADR-018 RU-5 (an undefined claim is
  refuted).
- The clause-run result document is the library's serialization of
  `run_clause`'s `ClauseRunReport` as `native-run-result/2`
  ([FR-267](FR-267-write-run-results-as-native-run-result-2.md)).
- The request file form is
  [FR-312](FR-312-read-a-clause-run-request-and-run-it.md)'s: `run` and the
  driver read it with `read_clause_run_request` and call `run_clause`. It
  lands in the change that lands FR-100's clause runner (FR-312's reader plus `run_clause`), with the deletion of native `run`
  ([FR-026](FR-026-run-standalone-native-workflow.md), ADR-031 R-1).
