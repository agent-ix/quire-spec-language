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
---
# FR-109: Run a selected state clause through the spine and report a typed disposition

## Description

QSL SHALL provide `qsl_replay::spine::run_clause`, a layer-6 entry beside
FR-100's `qsl_replay::spine::run`. It compiles a `1-draft` unit through the
spine (S1 to S4, the same `spine::compile` CLI `compile` and FR-100 use),
selects one state clause, or one Boolean function run as a claim, by name,
admits the supplied observations (FR-106), evaluates through S6a (FR-107 or
`CheckedPackage::call`) and returns one `ClauseRunReport`: a typed
disposition with the provenance of every input. It is the spine replacement
for native-run/1 clause execution (ADR-011 §5, the paragraph that says it has
no spine equivalent before M-6c) and for FR-023's in-process `execute`.

FR-100 already owns the spine run's argument binding and its S6a outcome
mapping. This requirement reuses both by reference and adds only object
arguments, observations and the claim reading of a Boolean result.

## Inputs

A `ClauseRunRequest`:

- the unit's source bytes and its four FR-001 labels, or an I3 extracted
  source (ADR-011 I3, `qsl-source`, feature `quire-extraction`) with its
  original document identity;
- FR-056's package input (domain packages by `sha256-jcs` digest) and
  FR-099's dependency input;
- the snapshot and invocation provisions (FR-106);
- a selection: `Clause(ClauseSelection)` (FR-106), or `Function { name,
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
  `category` (`success`, `violation`, `refusal`, `incomplete`, `undefined`
  or `internal-failure`), `truth` (only for `success` and `violation`) and,
  for every other category, the one record: for `compile`, `select` and
  `admit` a `RefusalRecord` (code, cause, locus); for an `evaluate`
  refusal, undefined or incomplete, FR-100's `outcome` member for that S6a
  outcome; for `evaluate` `internal-failure`, the `InternalFault`'s stage
  and invariant, as FR-100's internal-failure section gives them;
- provenance: the source identity and byte digest, the extraction's original
  identity and digest when I3 was used, the `package_id`, each model
  selection, the selection as given, and the identity and digest of every
  snapshot and invocation admission read;
- usage: the admission work and the evaluation meter charges, separately.

## Behavior

- The entry SHALL compile first. If the compile refuses or hits a limit, then
  the entry SHALL report stage `compile` with that code (for example
  `missing_import`/`missing-selection` for a model selection whose package is
  not supplied) and SHALL admit and evaluate nothing.
- If an expected `package_id` is given and the recompiled one differs, then
  the entry SHALL report stage `compile`, category `refusal`,
  `stale_dependency`, naming both identities, with no admission or
  evaluation (FR-098's stale package rule).
- The entry SHALL resolve the selected name in the compiled package's one
  name table of state clauses and functions. FR-104 refuses a clause whose
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
  claim, with `truth` set. The entry SHALL map every other S6a outcome, kernel
  or family, by FR-100's outcome mapping, to FR-100's `outcome` member and
  FR-100's exit status, category `refusal`, `undefined` or `incomplete` by
  its ADR-013 O-16 category. It restates none of FR-100's rows.
- If the S6a outcome is one FR-100 handles as an internal failure (the
  kernel `Refusal::CheckedInvariant`, or `CallFailure::Fault`), then the
  entry SHALL report stage `evaluate`, category `internal-failure`, with the
  `InternalFault` FR-100's internal-failure section names, and FR-100's
  internal-failure exit status.
- `ClauseRunReport::exit_code()` SHALL be one total match over the stage and
  category with no `_` arm: 0 and 10 as above; FR-100's exit status for an
  `evaluate` refusal, undefined or incomplete; FR-100's internal-failure
  exit status for `evaluate` `internal-failure`; and
  `qsl_foundation::diagnostic::Code::exit_code` of the record's code for a
  `compile`, `select` or `admit` result (20, 21 for an unsupported code, 22
  for an incomplete code).
- Each call SHALL build fresh admission and evaluation meters from the
  request's limits. The entry SHALL read no path, environment variable,
  clock or search location: every byte arrives in the request.
- `run_clause` SHALL NOT be reachable by CG: FR-060's T12-A rule already
  refuses any CG reference into `qsl_replay::spine`. Like FR-100's `run`, no
  public item of `run_clause`'s signature names a `qsl_eval` path
  (FR-100-AC-8).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-109-AC-1 | The healthy-parent request reports stage `evaluate`, `success`, `truth: true`, exit code 0, with the source digest, `package_id`, model selection, selection and the one snapshot's identity and digest in its provenance; violating-parent reports `violation`, `truth: false`, exit 10. | Test (TC-468) |
| FR-109-AC-2 | missing-model (no package supplied) reports stage `compile`, `refusal`, `missing_import`/`missing-selection`, exit 20, with no snapshot in its provenance; an expected `package_id` of another unit reports stage `compile`, `stale_dependency`, naming both; a `Clause` selection naming `Absent`, and one naming the function `sameIdentity`, each report stage `select`, `missing_declaration`/`missing-name`. | Test (TC-468) |
| FR-109-AC-3 | dangling-parent reports stage `admit`, `refusal`, `dangling_reference`, exit 20; incomplete-population reports stage `admit`, `incomplete`, `incomplete_population`, exit 22; exhausted-work (budget zero) reports stage `evaluate`, `incomplete`, FR-100's `{"kind": "incomplete", "limit": "work_units"}`, exit 22; none carries `truth`. | Test (TC-468) |
| FR-109-AC-4 | A `Function` selection of `sameIdentity` with arguments `{b: child, a: root}` (given in that order) over the distinct-identities snapshot reports `violation`, `truth: false`; with `a` = `b` = `child`, `success`; with `b` naming `ghost`, stage `admit`, `invalid_runtime_input`/`wrong-role-mapping`; with an argument naming `c`, stage `admit`, FR-100's refusal for an unknown parameter; a function returning `Integer` reports stage `select`, `ill_typed`/`type-mismatch`, before any call. | Test (TC-468) |
| FR-109-AC-5 | Running one request twice gives equal reports, including usage; a request whose snapshot bytes change after the selection digest was taken reports stage `admit`, `stale_dependency`/`byte-digest-mismatch`. For each S6a outcome other than `Completed`, the report's `outcome` member and exit code equal what FR-100's mapping gives for the same outcome (checked over the outcomes FR-100-AC-9 constructs); for the kernel `CheckedInvariant` and a `CallFailure::Fault`, which FR-100 handles as an internal failure, the report is stage `evaluate`, category `internal-failure`, carrying the fault's stage and invariant, with no `outcome` member and FR-100's internal-failure exit status. | Test (TC-468) |

## Dependencies

- FR-100 (the spine run's argument binding, outcome mapping, internal-failure
  handling and exit statuses), FR-106, FR-107 (admission and evaluation), FR-099 and FR-027
  (the spine compile), FR-098 (the stale `package_id` rule), FR-060 (the CG
  boundary).
- QSpec FR-301 (exit codes).
- The CLI request form for a clause run is not specified here: FR-100's
  native-run/1 `1-draft` route refuses `selection`, `snapshots` and
  `invocations` today, and the CLI form is a follow-up (the QSL-273 PR's
  open questions).
