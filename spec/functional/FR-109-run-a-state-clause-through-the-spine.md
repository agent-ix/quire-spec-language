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

QSL SHALL provide `qsl_replay::spine::run_clause`, the layer-6 entry that
compiles a `1-draft` unit through the spine (S1 to S4, the same
`spine::compile` CLI `compile` uses), selects one state clause, or one
function, by `QualifiedName`, admits the supplied observations (FR-106),
evaluates through S6a (FR-107) and returns one `ClauseRunReport`: a typed
disposition with the provenance of every input. It is the spine replacement
for native-run/1 clause execution (ADR-011 §5, the paragraph that says it has
no spine equivalent before M-6c) and for FR-023's in-process `execute`.

Today the spine has no run step. `qsl_replay::replay` recompiles and calls a
function with `ObjectEnvironment::default()`
(`qsl-replay/src/execute.rs:230-250`), so it never supplies objects,
populations or observations.

## Inputs

A `ClauseRunRequest`:

- the unit's source bytes and its four FR-001 labels, or an I3 extracted
  source (ADR-011 I3, `qsl-source`, feature `quire-extraction`) with its
  original document identity;
- FR-056's package input (domain packages by `sha256-jcs` digest) and
  FR-099's dependency input;
- the snapshot and invocation provisions (FR-106);
- a selection: `Clause(ClauseSelection)` (FR-106), or `Function { name,
  arguments, snapshot }`, whose arguments are Boolean, integer or
  `{population, key}` object references resolved in a current snapshot;
- an optional expected `package_id`;
- `SpineLimits`, `ObservationLimits` and the evaluation meter budget.

## Outputs

`Result<ClauseRunReport, ClauseRunRefusal>`. `ClauseRunRefusal` is only for a
request that cannot be formed (such as an empty source); every compile,
admission or evaluation result is a report.

`ClauseRunReport` holds:

- `disposition`: `stage` (`compile`, `admit` or `evaluate`), `category`
  (`success`, `violation`, `refusal`, `incomplete` or `undefined`), `truth`
  (only for `success` and `violation`) and, for every other category, the
  one `RefusalRecord` (code, cause, locus);
- provenance: the source identity and byte digest, the extraction's original
  identity and digest when I3 was used, the `package_id`, each model
  selection, the selection as given, and the identity and digest of every
  snapshot and invocation admission read;
- usage: the admission work and the evaluation meter charges, separately.

## Behavior

- The entry SHALL compile first. A compile refusal or limit SHALL be the
  report, stage `compile`, with its code (for example `missing_import`/
  `missing-selection` for a model selection whose package is not supplied),
  and nothing is admitted or evaluated.
- When an expected `package_id` is given and the recompiled one differs, the
  report SHALL be stage `compile`, category `refusal`, `stale_dependency`,
  naming both identities, with no admission or evaluation (FR-098's stale
  package rule).
- The selected name SHALL resolve in the compiled package's own clause and
  function tables. A name that resolves to neither SHALL be stage `admit`,
  `missing_declaration`/`missing-name`.
- An admission failure (FR-106) SHALL be the report, stage `admit`, with its
  category and record. S6a runs only after admission succeeds, and runs
  exactly once.
- For `Function`, admission SHALL admit the one current snapshot and resolve
  each object argument in it, and evaluation SHALL be
  `CheckedPackage::call` with that snapshot's `ObjectEnvironment`. The
  function's declared result SHALL be `Boolean`, or the report is stage
  `admit`, `ill_typed`/`type-mismatch`, before any call.
- The S6a `Evaluation` SHALL map to the report by ADR-012 §15.6: `Completed(b)`
  to `success` or `violation` with `truth: b`; `Incomplete` to `incomplete`;
  `Undefined` and `FamilyResult::Undefined` to `undefined`; `Refused` and
  `FamilyResult::Refused` to `refusal`.
- `ClauseRunReport::exit_code()` SHALL be one total match over the category
  with no `_` arm, by QSpec FR-301: `success` 0, `violation` 10, `undefined`
  20, and for `refusal` and `incomplete` the record's code through
  `qsl_foundation::diagnostic::Code::exit_code`, the map native `run` uses
  (21 for `unsupported_construct`, `unsupported_projection` and
  `unknown_required_feature`, 22 for an incomplete code such as
  `resource_exhausted` or `incomplete_population`, 20 otherwise).
- Each call SHALL build fresh admission and evaluation meters from the
  request's limits. The entry SHALL read no path, environment variable,
  clock or search location: every byte arrives in the request.
- `run_clause` SHALL NOT be reachable by CG: FR-060's T12-A rule already
  refuses any CG reference into `qsl_replay::spine`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-109-AC-1 | The healthy-parent request reports stage `evaluate`, `success`, `truth: true`, exit code 0, with the source digest, `package_id`, model selection, selection and the one snapshot's identity and digest in its provenance; violating-parent reports `violation`, `truth: false`, exit 10. | Test (TC-468) |
| FR-109-AC-2 | missing-model (no package supplied) reports stage `compile`, `refusal`, `missing_import`/`missing-selection`, exit 20, with no snapshot in its provenance; an expected `package_id` of another unit reports stage `compile`, `stale_dependency`, naming both; a selection naming `Absent` reports stage `admit`, `missing_declaration`/`missing-name`. | Test (TC-468) |
| FR-109-AC-3 | dangling-parent reports stage `admit`, `refusal`, `dangling_reference`, exit 20; incomplete-population reports stage `admit`, `incomplete`, `incomplete_population`, exit 22; exhausted-work (budget zero) reports stage `evaluate`, `incomplete`, `resource_exhausted`, exit 22, and neither carries `truth`. | Test (TC-468) |
| FR-109-AC-4 | A `Function` selection of `sameIdentity(a, b)` with `a` = `child` and `b` = `root` over the distinct-identities snapshot reports `violation`, `truth: false`; with `a` = `b` = `child`, `success`; a function returning `Integer` reports stage `admit`, `ill_typed`/`type-mismatch`, before any call. | Test (TC-468) |
| FR-109-AC-5 | Running one request twice gives equal reports, including usage; a request whose snapshot bytes change after the selection digest was taken reports stage `admit`, `stale_dependency`/`byte-digest-mismatch`. `exit_code()` returns 0, 10 and 20 for `success`, `violation` and `undefined`, and `Code::exit_code` of the record for `refusal` and `incomplete` (20 for `dangling_reference`, 22 for `resource_exhausted`, 21 for `unsupported_construct`). | Test (TC-468) |

## Dependencies

- FR-106, FR-107 (admission and evaluation), FR-099 and FR-027 (the spine
  compile), FR-098 (the stale `package_id` rule), FR-060 (the CG boundary).
- QSpec FR-301 (exit codes).
- QSL-271 (A05-1) puts the CLI `run` entry in `qsl_replay::spine`; this
  entry sits beside it. The CLI request form for a clause run is not
  specified here (see the QSL-273 PR's open questions).
