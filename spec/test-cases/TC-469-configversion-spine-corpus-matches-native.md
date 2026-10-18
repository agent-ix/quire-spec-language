---
id: TC-469
title: "The ConfigVersion spine corpus retains typed dispositions"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-108
    type: verifies
---
# TC-469: The ConfigVersion spine corpus retains typed dispositions

## Description

Verify the 17-case ConfigVersion corpus through `run_clause` against the
independent expected dispositions, through FR-267 native-run-result/2
serialization, at the QSpec FR-180-AC-4 boundaries, for input-fixed identity,
and through I3 extraction.

Scope: FR-108-AC-1 to FR-108-AC-6.

## Test Procedure

The tests live in `tests/it/config_version_spine.rs`, with
`mod config_version_spine;` in `tests/it/main.rs` (AGENTS.md). The spine
corpus generator (`examples/config-version/spine_corpus.rs`, reached through
the `config_version_spine` support module) writes, per case, the spine files;
the native fixtures generator (`examples/config-version/fixtures.rs`) writes
the native files and delegates the spine files to the same module: the FR-108 unit, the domain package
`model.semantic-ir.json`, the FR-106 documents and the `ClauseRunRequest`
inputs. `cases.rs` gains `BoundaryZero` (root 0, child 1), `BoundaryMax`
(root 999, child 1000), `BelowRange` (root -1) and `AboveRange` (child 1001),
all under `ParentOrder` with self `child`.

1. For each of the 17 cases, run `run_clause` and compare the report with an
   exhaustive `match` over `Case` giving FR-108's Corpus table.
2. For each case, serialize the `run_clause` report as native-run-result/2,
   read it through FR-267's strict reader, and compare stage, category, truth
   and code against the independently checked typed report.
3. For below-range and above-range, read the refusal's object and field
   from the native-run-result/2 record.
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

The spine's independent expected table includes the four boundary cases:
success/true twice, then admission refusal with `invalid_runtime_input`
twice. Tag the tests `#[trace("TC-469", "FR-108-AC-n")]` and the
ConfigVersion behavior they directly verify with `FR-032-AC-n`.

## Expected Results

- Step 1: each report equals its row, including the exit code.
- Step 2: no case differs in stage, category, truth or code after the /2
  round trip.
- Step 3: each /2 record names the object (`root` for below-range, `child`
  for above-range) and the field `versionNumber`.
- Step 4: identical files; identical reports, each holding the
  disposition, the `package_id` and the usage; the `package_id`-selected
  runs give the same reports.
- Step 5: each extracted run's stage, category, truth and exit code equal
  the direct run's, and its `package_id` equals the one a direct
  `spine::compile` of the extracted body's bytes under its body identity
  emits.
- Step 6: every direct run's report carries that `package_id`, and the bytes admit
  through I04 `read`.
