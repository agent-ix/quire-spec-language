---
id: FR-565
title: "Publish the lifecycle matrix cases and their library documents for the driver CLI half"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-564
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-296
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-301"
    type: depends_on
---
# FR-565: Publish the lifecycle matrix cases and their library documents for the driver CLI half

## Description

The same request through the library and through the CLI gives the same
document (ADR-029 CB-4, QSpec FR-300-AC-3). The CLI is the driver's
(FR-287), so the CLI half of the FR-564 matrix runs in the driver
repository. For both halves to run the same cases without a copy, QSL SHALL
publish the cases as a library in `qsl-conformance`, and the driver SHALL
depend on that crate.

```rust
pub fn lifecycle_matrix() -> &'static [MatrixCell];

pub enum MatrixCell {
    Case(MatrixCase),
    NotApplicable { verb: Verb, kind: OutcomeKind, reason: &'static str },
    PendingEngine { verb: Verb, kind: OutcomeKind, reason: &'static str },
}

pub fn run_case(case: &MatrixCase) -> Result<CaseOutcome, CaseError>;

pub fn run_case_with(
    case: &MatrixCase,
    registry: &Registry,
    engines: &Engines,
) -> Result<CaseOutcome, CaseError>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CaseStage { Read, Check, Package, Execute, Analyze, Monitor, Prove, Document }

pub struct CaseError { pub stage: CaseStage, /* detail */ }
```

`run_case` is `run_case_with` over an empty registry and `Engines::qsl()`.
QSL cannot link CG, so it holds no Kani descriptor and no copy of the Kani
manifest: the driver, which links the Kani backend through CG, passes its own
registry through `run_case_with`. A case whose operation takes neither
argument ignores them.

`CaseError` is the failure of running a case as such, not an outcome of the
case's operation: its `stage` is a `Copy` enum, `CaseStage`, naming the stage
that failed (reading the case's inputs, the operation's own stage, or building
the document), so the caller matches on it without parsing text. A case whose
operation settles refusal, unsupported, incomplete or any other FR-564 kind
returns `Ok`.

`CaseOutcome` holds the typed library outcome, its typed FR-286
`OutcomeDocument` (or, for a `prove` (QSL half) case, the typed FR-286
item), and its FR-285 exit code. It holds no encoded bytes. The comparison surface between the library
and the CLI is the typed document, not its bytes: the driver encodes the
`OutcomeDocument` `run_case` returns and the document its CLI wrote with the
same canonical JSON encoder, `ix_cli_kit`'s, each followed by one newline,
and compares those encodings. QSL's own `OutcomeDocument::to_bytes`
(`serde_json::to_vec`) is not the comparison surface.

- `Verb` is `Check`, `Package`, `Execute`, `Analyze`, `Monitor` or `Prove`,
  and `OutcomeKind` is one of FR-564's eight kinds.
- `lifecycle_matrix()` holds exactly one cell per (verb, kind) pair, 48 in
  all, in verb then kind order as FR-564's table lists them: a `Case` for each
  applicable cell, a `NotApplicable`, carrying a reason, for each n/a cell,
  and a `PendingEngine`, carrying a reason like a `NotApplicable`, for each
  † cell (FR-564) whose byte-driven route to its engine does not yet exist; a
  † cell whose route exists is a `Case`. A `PendingEngine`
  cell carries no case: no document is produced for it and `run_case` cannot
  be called on it.
- A `MatrixCase` holds its verb and kind, every input byte the case needs
  with the identity it is read under (sources, domain packages, libraries,
  traces, snapshots), the operation's request parameters and limits, the
  cancellation plan (none, or cancelled before the call with `Requested` or
  `Deadline`), and the expected category and exit code. Its bytes are the
  crate's own data, and `run_case` reads no file.
- **Source identity.** The `quire` CLI reads each source from the path its
  arguments name and builds `SourceIdentity::new("quire", path)` from that
  path. Every `MatrixCase` therefore states, for each input it carries, the
  file name the driver writes it under, relative to the case's working
  directory, and states that working directory: a new empty directory the
  driver creates for the case, from which it runs `quire` with each file
  argument spelled as its file name. `run_case` reads each source under
  `SourceIdentity::new("quire", <file name>)`, so the identities in the two
  documents agree.
- `run_case` runs the case through QSL's library operation and returns
  `Ok(CaseOutcome)`, or `Err(CaseError)` naming the failed stage when the
  case itself could not be run.

## Inputs

None for `lifecycle_matrix`; a `MatrixCase` for `run_case`.

## Outputs

The matrix cells; for each case, `Ok(CaseOutcome)`: the library outcome, the
typed document or item, and the exit code, or `Err(CaseError)` with its stage.

## Behavior

- `lifecycle_matrix()` shall return one cell per (verb, kind) pair of FR-564,
  a `Case` for each applicable cell, a `NotApplicable` with its reason for
  each n/a cell and a `PendingEngine` with its reason for each pending †
  cell.
- Each `MatrixCase` shall hold every input byte it needs.
- The `run_case` operation shall read no file.
- When `run_case` runs a case, it shall return `Ok` with the outcome of the
  case's operation, its typed FR-286 document or item, and its FR-285 exit
  code.
- When a case cannot be run, `run_case` shall return `Err(CaseError)` whose
  `stage` names the failed stage.
- Each `MatrixCase` shall state the file name of each input and the working
  directory the driver runs `quire` in.
- The `run_case` operation shall read each source under
  `SourceIdentity::new("quire", <file name>)`.
- The document `run_case` returns shall have the case's expected category,
  and its exit code shall be the case's expected exit code.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-565-AC-1 | `lifecycle_matrix()` returns one cell per (verb, kind) pair in FR-564's verb and kind order: `Case` cells exactly at FR-564's applicable cells and at each † cell whose route exists, `NotApplicable` cells exactly at its n/a cells, and `PendingEngine` cells exactly at each † cell whose route does not exist, each `NotApplicable` and `PendingEngine` with a non-empty reason. | Test (TC-1394) |
| FR-565-AC-2 | For every `Case`, `run_case` returns `Ok` with a typed document (or, for `prove`, an item) whose category and exit code equal the case's expected category and exit code, two `run_case` calls give equal `OutcomeDocument` values, and every source identity in the document is `SourceIdentity::new("quire", <the case's file name>)`. A case with an input the test removes returns `Err(CaseError)` whose `stage` is `CaseStage::Read`, the `Copy` stage value, and no `CaseOutcome`. | Test (TC-1394) |
| FR-565-AC-3 | Run with the process's working directory set to an empty temporary directory, every `Case` gives the same `OutcomeDocument` as AC-2. | Test (TC-1394) |

## Dependencies

- ADR-029 CB-1, CB-4: the driver CLI and the library-CLI document identity.
- [FR-564](FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md): the matrix.
- [FR-294](FR-294-run-every-execution-backend-behind-one-seam.md), [FR-296](FR-296-select-the-execution-backend-in-the-request.md): the backend seam and choice the `execute` cases use.
- [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md): the document.
- [FR-287](FR-287-reach-qsl-through-the-driver-cli.md): the driver CLI.
- QSpec FR-300-AC-3, FR-301-AC-1: the library and the CLI agree.

## Overlap

The driver repository depends on `qsl-conformance` and, for each `Case` of
the `check`, `package`, `execute`, `analyze` and `monitor` rows, writes the
case's inputs at their file names in the case's new working directory, runs
the `quire` verb there, and asserts that the `ix_cli_kit` canonical JSON
encoding (plus one newline) of the `--format json` document it writes equals
that of `run_case_with`'s `OutcomeDocument` over its own Kani registry, and
that it exits with the same code.

Two kinds of cell cannot be produced through the shipped binary, and the
driver produces them through a test-only adapter behind a Cargo feature that
the shipped `quire` binary does not enable (the precedent is its
`tc018-test-adapter`):

- **Timeout and cancellation cells.** The adapter adds a pre-cancel hook
  that cancels the operation's `Cancel` handle with `Requested` or
  `Deadline` before the first stage runs, as the case states.
- **The `execute` tool-failure cell.** Under the same feature, the adapter
  links `qsl-conformance`'s fault-injecting test backend into `quire` as a
  supplied backend, so the cell's `CallFailure::Fault` reaches the CLI.

For the `prove` row it runs `quire prove` with the Kani provider and a
process plugin and asserts each item equals the case's item. The driver half
carries no copy of a case.

## References

- QSL-390 (ARCH-50), QSL-392 (ARCH-52).
