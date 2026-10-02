---
id: FR-312
title: "Read a clause-run request and run it through run_clause"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-014
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-267
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-031
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-301
    type: depends_on
---
# FR-312: Read a clause-run request and run it through run_clause

## Description

QSL SHALL provide `qsl_replay::spine::read_clause_run_request`, which reads a
clause-run request file and the files it names into FR-109's
`ClauseRunRequest`. The request shape is QSL's. The `run` command
([FR-100](FR-100-run-a-named-function-through-the-spine.md)) and the
orchestrating driver (agent-ix/quire-driver, ADR-011 T-13) expose it: each
passes the request file to this reader, calls
`qsl_replay::spine::run_clause` (FR-109) with the result, and writes the
report as FR-267's `native-run-result/2` clause-run document. Neither forms
a `ClauseRunRequest` any other way.

FR-100's clause runner is this requirement's reader plus `run_clause`
(FR-109); every artifact names the change that lands it as "the change that
lands FR-100's clause runner". That change is ADR-011 M-6c's deletion of
native `run` (FR-026 to FR-032) and `native-run-result/1` (ADR-031 R-1).

## Inputs

A native-run/1 request file for a `1-draft` program (FR-100's closed
envelope, source selections and intake limits) that carries `clause` in
place of `call`. `clause` is a closed object:

- `selection`: exactly one of
  - `{"clause": {"name": N, "observation": O}}`, FR-106's
    `ClauseSelection`, where `O` names the snapshot or invocation the clause
    reads by its key in `observations`;
  - `{"frame": {"operation": Op, "invocation": K}}`, FR-115's frame
    selection;
  - `{"function": {"name": N, "arguments": A, "snapshot": K}}`, FR-109's
    `Function` selection, with `A` as FR-109 states;
- `observations`: an object mapping each key to the file of a
  `quire.state.snapshot/v1` or `quire.state.invocation/v1` document
  (FR-106);
- `package_id`: optional, the expected `package_id`, lowercase hex;
- `work_units`: optional, FR-100's evaluation budget;
- `observation_limits`: optional, FR-106's `ObservationLimits` overrides.

The request's `models` and `libraries` are FR-100's and give FR-109's
package and dependency inputs.

## Outputs

- `read_clause_run_request` returns `Result<ClauseRunRequest,
  ClauseRunRequestRefusal>`.
- The `run` command and the driver write the report as one
  `native-run-result/2` clause-run document (FR-267) on stdout and exit with
  `ClauseRunReport::exit_code()` (FR-109, QSpec FR-301).

## Behavior

- The reader SHALL decode `clause` under FR-100's closed-envelope rules:
  unknown and duplicate members and positional record arrays refuse
  `invalid-request` at stage `request`, naming the member.
- If a `1-draft` request carries both `call` and `clause`, or neither, then
  the reader SHALL refuse `invalid-request` at stage `request`.
- If `selection` holds no variant, more than one, or a variant whose
  members are not the closed set above, then the reader SHALL refuse
  `invalid-request` at stage `request`, naming the member.
- If a selection names an observation key `observations` does not hold,
  then the reader SHALL refuse `invalid-request` at stage `request`, naming
  the key.
- The reader SHALL read each observation file once, under FR-100's
  `dependent_bytes` limit and relative to the request file's directory.
- The reader SHALL pass every byte it read into the `ClauseRunRequest`;
  `run_clause` reads no path (FR-109).
- When the reader admits a request, the run command SHALL run it through
  `run_clause` exactly once.
- The run command SHALL write every compile, selection, admission or
  evaluation result as the report FR-109 gives, never as a command-error
  envelope.
- If the reader refuses a request, then the run command SHALL write FR-267's
  command-error envelope to stderr, write nothing to stdout, and exit with
  the refusal code's exit status.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-312-AC-1 | Through `run`, FR-108's healthy-parent case as a clause-run request (`ParentOrder` over its current snapshot) writes one `native-run-result/2` document with `success`, `truth: true` and `basis`, exit 0; violating-parent writes `violation`, exit 10. The same request file passed to the driver's run entry gives byte-identical stdout and the same exit. | Test (TC-891) |
| FR-312-AC-2 | A request carrying both `call` and `clause`, one carrying neither, one whose `selection` holds both `clause` and `function`, one with an unknown member in `clause`, and one whose selection names an observation key `observations` lacks each refuse `invalid-request` at stage `request`, exit 20, empty stdout, naming the member or key. | Test (TC-891) |
| FR-312-AC-3 | Observation files whose total size exceeds `dependent_bytes` refuse at the intake limit, and the same request with the limit raised to their size runs. | Test (TC-891) |
| FR-312-AC-4 | A `function` selection of `sameIdentity` with object arguments, and a `frame` selection, each give the report FR-109 and FR-115 give for the same `ClauseRunRequest` built in process; a request whose `package_id` names another unit gives FR-109's stage `compile` `stale_dependency` report, exit 20, as a `/2` document, not a command-error envelope. | Test (TC-891) |

## Dependencies

- [FR-109](FR-109-run-a-state-clause-through-the-spine.md): `ClauseRunRequest`,
  `run_clause` and `ClauseRunReport`.
- [FR-100](FR-100-run-a-named-function-through-the-spine.md): the request
  envelope, source selections, `models`, `libraries`, intake limits and the
  `run` command.
- [FR-106](FR-106-admit-snapshots-and-invocations.md): observation documents
  and `ObservationLimits`; FR-115 for the frame selection.
- [FR-267](FR-267-write-run-results-as-native-run-result-2.md): the `/2`
  clause-run document and the command-error envelope.
- QSpec FR-301 (exit codes).

## Status

Specified; not yet implemented -- TC-891 planned. Lands with the deletion of
native `run` (ADR-031 R-1). That change also deletes the per-file `sha256:`
digests the native-v1 request selections carry and their checks (FR-026,
FR-027, FR-100): a request names its files, and the command reads their
bytes as they are.

## References

- Owning ticket: Linear QSL-389 (plan slice E19-CLI).
