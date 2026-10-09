---
id: TC-469
title: "The ConfigVersion spine corpus gives native-equal typed dispositions"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-108
    type: verifies
---
# TC-469: The ConfigVersion spine corpus gives native-equal typed dispositions

## Description

Verify the 17-case ConfigVersion corpus through `run_clause` against the
independent expected dispositions, against native `quire-spec run`, at the
QSpec FR-180-AC-4 boundaries, for input-fixed identity, and through I3
extraction.

Scope: FR-108-AC-1 to FR-108-AC-6.

## Test Procedure

The tests live in `tests/it/config_version_spine.rs`, with
`mod config_version_spine;` in `tests/it/main.rs` (AGENTS.md). The fixtures
generator (`examples/config-version/fixtures.rs`) writes, per case, the native
files it writes today and the spine files: the FR-108 unit, the domain package
`model.semantic-ir.json`, the FR-106 documents and the `ClauseRunRequest`
inputs. `cases.rs` gains `BoundaryZero` (root 0, child 1), `BoundaryMax`
(root 999, child 1000), `BelowRange` (root -1) and `AboveRange` (child 1001),
all under `ParentOrder` with self `child`.

1. For each of the 17 cases, run `run_clause` and compare the report with an
   exhaustive `match` over `Case` giving FR-108's Corpus table.
2. For each case, run native `quire-spec run request.json` and `run_clause`;
   map native (`status`, `stage`, `truth`, `code` or diagnostic codes, exit)
   through FR-108's map (native `resource_exhausted` ↔ spine
   `limit: work_units`) and compare.
3. For below-range and above-range, read the refusal's locus in both
   outputs.
4. Generate the corpus into two temporary directories and compare every
   file, in both directions (identical file sets, not just one direction's
   files present in the other); run every one of the 17 cases twice and
   compare the full report; then,
   for every case that reaches compile, run again with the expected
   `package_id` set to the `package_id` spine `compile` emits for the unit,
   and compare.
5. With `--features quire-extraction`, embed each case's unit in a
   `rules.md` fence, extract it through `qsl-source`, and run the extracted
   source. Independently compile the extracted body's bytes under its body
   identity with a direct `spine::compile` call.
6. Independently re-derive the unit's `package_id` via a direct `spine::
   compile` call, compare it with every direct run's report, and read the emitted
   package bytes through QSpec's I04 `read`.

Extend `tests/it/config_version.rs`'s `expected()` with the four new cases
(`Completed(true)` twice, `Validation { code: "invalid_runtime_input",
incomplete: false }` twice). Tag the tests `#[trace("TC-469",
"FR-108-AC-n")]`; the native parity test also carries `"FR-032-AC-1"`.

## Expected Results

- Step 1: each report equals its row, including the exit code.
- Step 2: no case differs in stage (under the map), category, truth, code or
  exit code.
- Step 3: both outputs name the object (`root` for below-range, `child` for
  above-range) and the field `versionNumber`.
- Step 4: identical files; identical reports, each holding the
  disposition, the `package_id` and the usage; the `package_id`-selected
  runs give the same reports.
- Step 5: each extracted run's stage, category, truth and exit code equal
  the direct run's, and its `package_id` equals the one a direct
  `spine::compile` of the extracted body's bytes under its body identity
  emits.
- Step 6: every direct run's report carries that `package_id`, and the bytes admit
  through I04 `read`.

## Status

Implemented. All six steps pass, including step 6's I04 `read`
half (FR-108-AC-6). Step 2 and the
native test's four new cases run until M-6c retires the `0-draft` native
path; that PR deletes step 2 with it (ADR-012 §15.8).
