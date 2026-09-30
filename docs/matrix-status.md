# Matrix status checks

The compiler's matrices use one `Status` column for functional coverage, quality
coverage and test summaries. Upstream
[`spec-artifacts-process#87`](https://github.com/agent-ix/spec-artifacts-process/pull/87)
collapsed the former `Coverage Status` and `Status` names into that single
column; do not rename a header back toward `Coverage Status`. The historical
record below was measured before that collapse, when functional tables still
used `Coverage Status`; the compiler does not fork or install a shared catalog.

The CLI capability has landed through
[CLI PR77](https://github.com/agent-ix/quire-cli/pull/77). Set
`QUIRE_STATUS_BIN` to the quire binary, `QUIRE_PROCESS_ROOT` to the
`spec_artifacts_process` directory and `QUIRE_ISO_ROOT` to the
`spec_artifacts_iso` directory. From this compiler checkout:

```sh
"$QUIRE_STATUS_BIN" coverage --scope . \
  --module "$QUIRE_PROCESS_ROOT" --module "$QUIRE_ISO_ROOT" --json
"$QUIRE_STATUS_BIN" validate --scope . \
  --module "$QUIRE_PROCESS_ROOT" --module "$QUIRE_ISO_ROOT" 'spec/**/*.md' --summary
```

Explicit module roots replace ambient discovery.

## Observed results and limits

The check reports zero `status-column-matches-nothing` diagnostics and
zero status lies on the real matrices. The trace rollup is 325/329; its
`no_symbol_rows` explicitly exempts manual TC-010 and inspection FR-017-AC-2
from source-symbol status checks. The other two unbacked targets are StR-001's
demonstration criteria; this check does not establish their demonstration
evidence. The rollup is not a count of four missing automated tests.

| Matrix | Bound test cases |
| --- | --- |
| model-linking | 41/42; TC-115 is unwritten |
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

## Status repair re-run (#28)

The same commands were re-run after the
`spec/model-linking/tests.md` status repair. `coverage` exits 0
with `status_lies: []`, no `status-column-matches-nothing` diagnostic and
`coverage.backed` 374/383 matrix rows over 264 criteria; `validate --scope .
'spec/**/*.md'` exits 0.

An empty `status_lies` list means nothing only when the run reports no
`status-column-matches-nothing` diagnostic; with that diagnostic present the
check was skipped.

Four limits of that clean result are worth stating, because none is caught by the
absence of status diagnostics:

- **A row is classified by its `Test Cases` cell, not per acceptance criterion.**
  A row naming a backed test case reads complete whatever criterion sits in its
  `Acceptance Criteria` cell. Per-criterion honesty therefore rests on the minted
  acceptance-criterion targets and their group counts, never on the row status.
- **The group counts, not the row counts, expose `FR-042-AC-10`.** Its matrix row
  names `TC-121`, and `TC-121` binds through nine test modules, so the row is not
  listed in `unbacked_rows`; only the FR-042 group count, 9/10, shows the
  criterion itself has no tagged evidence. A completion status on that row would
  not have been reported as a status lie. It stays `🚧 Planned` and waits on the
  `quire-protocol` IT-001 handoff.
- **One `tag-on-non-binding-symbol` diagnostic remains.** Trace id `FR-041` is
  written on the production function `check_selected_profiles` at
  `src/linking/native.rs:19`, which binds nothing. Every FR-041 acceptance row is
  independently backed by `tests/native_model_profiles.rs`, so no status depends
  on that tag, but the tag itself should become an `Implements:` marker.
- **Seven `section-matches-nothing` diagnostics remain, and they are not a
  status defect.** The `nfr-acceptance-criterion` declaration selects every NFR
  by archetype and reports that none has an `Acceptance Criteria` section. None
  does: the seven NFRs state their obligations as `Measurement and Evaluation`
  metric rows plus a prose `Verification` section, and when measured no
  `NFR-nnn-AC-n` id existed anywhere in this repository. That last
  clause is time-bounded and is already being overtaken: `NFR-008` on
  `agent-a/l5-native-temporal` declares `NFR-008-AC-1..AC-5` under a real
  `## Acceptance Criteria` heading with the required `ID | Criteria |
  Verification` shape, tagged by `tests/composed_temporal_limits.rs`. That does
  not weaken the conclusion — the seven documents measured here still
  legitimately have no acceptance table — but do not re-cite "none anywhere"
  without re-measuring. The NFR archetype makes
  `Acceptance Criteria` optional while a trace-target declaration has no way to
  say a section is optional, which is the `agent-ix/quire-rs#327` class of false
  alarm the process module already documents for `## Constraints`. Renaming
  `## Verification` to `## Acceptance Criteria` is not the fix: measured on
  `NFR-001`, it makes `quire validate` exit 1 with `required 'verification'
  (section_body(Verification)) is missing` plus an `acceptance_criteria` assert
  demanding an `| ID | Criteria | Verification |` table, while `coverage` mints
  exactly nothing more (374/383 before and after) and only trades the diagnostic
  for `section-holds-no-table`. Authoring such a table would be inventing
  acceptance criteria that no requirement states and no test carries; it is not
  done here.

Two further traps, both found by paying for them once:

- **Census both trees, not just `tests/`.** Tag coverage in this repository is
  not confined to `tests/*.rs`: `TC-120` binds through three modules, of which
  only `tests/native_model_profiles.rs` is a test file — the other two are
  `#[cfg(test)]` unit tests at `src/checking/types.rs:462` and
  `src/linking.rs:1006`. `TC-114` binds through eight modules including
  `src/linking/composed/arena.rs`. Two independent audits of these same rows
  each under-counted, in different directions, by scanning `tests/` alone. Any
  tag census must scan `src/` as well or it will report a module count that is
  simply wrong.

[SR-275](../reviews/26-09-09-matrix-status-review.md) records the bounded review.
The producer merge and installed-stack adoption landed through
`spec-artifacts-process#87`. Remaining work: compiler
[#28](https://github.com/agent-ix/quire-spec-language/issues/28) for the negative
control on a false Tested claim and the readiness-claim corrections it lists;
unrelated engineering continues while those steps are pending.
