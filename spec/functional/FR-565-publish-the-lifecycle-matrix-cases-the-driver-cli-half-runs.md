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
}

pub fn run_case(case: &MatrixCase) -> CaseOutcome;

pub fn run_case_with(case: &MatrixCase, registry: &Registry, engines: &Engines) -> CaseOutcome;
```

`run_case` is `run_case_with` over the first slice's registry (ADR-029 SL-1:
the linked Kani descriptor) and `Engines::qsl()`. A case whose operation
takes neither ignores them.

- `Verb` is `Check`, `Package`, `Execute`, `Analyze`, `Monitor` or `Prove`,
  and `OutcomeKind` is one of FR-564's eight kinds.
- `lifecycle_matrix()` holds exactly one cell per (verb, kind) pair, 48 in
  all, in verb then kind order as FR-564's table lists them: a `Case` for each
  applicable cell and a `NotApplicable` with FR-564's reason for each other.
- A `MatrixCase` holds its verb and kind, every input byte the case needs
  with the identity it is read under (sources, domain packages, libraries,
  traces, snapshots), the operation's request parameters and limits, the
  cancellation plan (none, or cancelled before the call with `Requested` or
  `Deadline`), and the expected category and exit code. Its bytes are the
  crate's own data: a `MatrixCase` has no member that names a file path, and
  `run_case` takes none.
- `run_case` runs the case through QSL's library operation and returns the
  outcome, its FR-286 document bytes and its FR-285 exit code. For a `prove`
  (QSL half) case it returns the FR-286 item and its exit code instead of a
  whole document.

## Inputs

None for `lifecycle_matrix`; a `MatrixCase` for `run_case`.

## Outputs

The matrix cells; for each case, the library outcome, document or item
bytes and exit code.

## Behavior

- `lifecycle_matrix()` shall return one cell per (verb, kind) pair of FR-564,
  a `Case` for each applicable cell and a `NotApplicable` with its reason for
  each other.
- Each `MatrixCase` shall hold every input byte it needs.
- The `run_case` operation shall read no file.
- When `run_case` runs a case, it shall return the outcome of the case's
  operation, its FR-286 document or item bytes, and its FR-285 exit code.
- The document `run_case` returns shall have the case's expected category,
  and its exit code shall be the case's expected exit code.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-565-AC-1 | `lifecycle_matrix()` returns 48 cells in FR-564's verb and kind order, one per (verb, kind) pair: 40 `Case` cells exactly at FR-564's applicable cells and 8 `NotApplicable` cells at its n/a cells, each with its FR-564 reason. | Test (TC-1394) |
| FR-565-AC-2 | For every `Case`, `run_case` returns a document (or, for `prove`, an item) whose category and exit code equal the case's expected category and exit code, and two `run_case` calls give equal bytes. | Test (TC-1394) |
| FR-565-AC-3 | A test that destructures `MatrixCase` and each of its member types exhaustively finds no member of type `Path`, `PathBuf`, `OsString` or a string documented as a path, and `run_case` and `run_case_with` take no path argument; run with the process's working directory set to an empty temporary directory, every `Case` gives the same bytes as AC-2. | Test (TC-1394) |

## Dependencies

- ADR-029 CB-1, CB-4: the driver CLI and the library-CLI document identity.
- [FR-564](FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md): the matrix.
- [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md): the document.
- [FR-287](FR-287-reach-qsl-through-the-driver-cli.md): the driver CLI.
- QSpec FR-300-AC-3, FR-301-AC-1: the library and the CLI agree.

## Overlap

The driver repository depends on `qsl-conformance` and, for each `Case` of
the `check`, `package`, `execute`, `analyze` and `monitor` rows, runs the
`quire` verb with the case's inputs and asserts that the `--format json`
document it writes is byte-equal to `run_case`'s and that it exits with the
same code. It owns how its CLI produces the timeout and cancellation cells
(its deadline option and its interrupt handling), and produces each before
the first stage runs, as the case does, so the documents stay byte-equal. For the `prove` row it runs
`quire prove` with the Kani provider and a process plugin and asserts each
item equals the case's item. The driver half carries no copy of a case.

## References

- QSL-390 (ARCH-50), QSL-392 (ARCH-52).
