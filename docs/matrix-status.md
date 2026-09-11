# Matrix status checks

The compiler's seven matrices correctly use `Coverage Status` for functional
coverage and `Status` for test summaries. Do not rename either header to work
around the installed catalog. The owning correction is
[process PR85](https://github.com/agent-ix/spec-artifacts-process/pull/85), with
[ISO PR37](https://github.com/agent-ix/spec-artifacts-iso/pull/37). Both remain
review candidates; the compiler does not fork or install a shared catalog.

The following exact stack was checked against compiler `7619105`. Its CLI and
engine report clean source provenance. The CLI capability has landed through
[CLI PR77](https://github.com/agent-ix/quire-cli/pull/77).

| Component | Selected source revision |
| --- | --- |
| CLI | ff638b9802178aa62c757aab914cf0288c1cbe67 |
| Engine | d3bc2baff191c9521f1064480a56c8dc0bd1c7fa |
| Process module | e6ea5151b59a55d7ce0d43f1581cbe276f750e04 |
| ISO module | a60ee12d735976081849f60a38d603fb5494b015 |

Select a binary and module checkouts matching those revisions; inspect the CLI's
`provenance` output and each checkout's Git revision and clean status. Set
`QUIRE_STATUS_BIN` to that binary, `QUIRE_PROCESS_ROOT` to its selected
`spec_artifacts_process` directory and `QUIRE_ISO_ROOT` to the selected
`spec_artifacts_iso` directory. From this compiler checkout:

```sh
"$QUIRE_STATUS_BIN" provenance --pretty
"$QUIRE_STATUS_BIN" coverage --scope . \
  --module "$QUIRE_PROCESS_ROOT" --module "$QUIRE_ISO_ROOT" --json
"$QUIRE_STATUS_BIN" validate --scope . \
  --module "$QUIRE_PROCESS_ROOT" --module "$QUIRE_ISO_ROOT" 'spec/**/*.md' --summary
```

Explicit module roots replace ambient discovery. The installed CLI `4f6ed024`
with engine `ca7362d4` rejects the new `status_column` field; its ordinary ambient
run still reports seven missing status columns. A successful check with the
candidate stack does not upgrade that installed setup or promote a Quoin lock.

## Observed results and limits

The exact stack reports zero `status-column-matches-nothing` diagnostics and
zero status lies on the real matrices. The trace rollup is 325/329; its
`no_symbol_rows` explicitly exempts manual TC-010 and inspection FR-017-AC-2
from source-symbol status checks. The other two unbacked targets are StR-001's
demonstration criteria; this check does not establish their demonstration
evidence. The rollup is not a count of four missing automated tests.

| Matrix | Bound test cases |
| --- | --- |
| model-linking | 35/35 |
| native-lowering | 5/5 |
| native-packages | 14/14 |
| native-readiness | 9/9 |
| native-runtime | 23/23 |
| native-workflow | 16/16 |
| root tests | 9/10; TC-010 is Manual |

These counts establish trace binding. Status classification can detect a
completion claim with unbacked references; it does not execute tests or prove
their adequacy. Existing run records provide separate execution evidence.
FR-009-AC-5 / TC-094 remains `🚧 Generated activation deferred` even though its
test symbol exists. Other missing-section and vocabulary diagnostics remain
visible and are not dismissed by the absence of status-column diagnostics.

## Negative controls

Manual CLI controls used an isolated copy of this commit's spec and Rust source
trees. No live matrix, production source or producer module was altered.

1. Add a Unit test-summary row TC-999998 with no tagged source symbol, then
   replace the first functional row's Test Cases cell in each of the seven
   matrices with TC-999998, retaining its complete status. The report identifies
   seven `functional-coverage` status lies, one in each actual matrix. Report
   mode exits 0; strict mode exits 1 with byte-identical JSON. No status-column
   diagnostic appears.
2. Add a declared but unimplemented FR-999-AC-1 control and point TC-999998's
   Traces To cell at it. The report retains the seven functional lies and adds
   the test-summary `traces-to` lie under its ordinary `Status` header.

[SR-275](../reviews/26-09-09-matrix-status-review.md) records the bounded review.
Compiler [#28](https://github.com/agent-ix/quire-spec-language/issues/28) remains
open for producer merge and installed-stack adoption; unrelated engineering
continues while those steps are pending.
