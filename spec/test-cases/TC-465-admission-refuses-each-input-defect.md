---
id: TC-465
title: "Admission refuses or reports incomplete for each input defect, in check order"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: verifies
---
# TC-465: Admission refuses or reports incomplete for each input defect, in check order

## Description

Verify that each FR-106 admission check fails alone with its code, cause and
input path, that completeness precedes closure, and that the frame compares
scalar as well as reference fields.

Scope: FR-106-AC-3, FR-106-AC-4, FR-106-AC-5.

## Test Procedure

Start from TC-464's healthy-parent snapshot (for the `Current` rows) or its
changed-version invocation (for the `Invocation` rows). Apply one mutation per
row and admit.

| # | Mutation | Expected |
| --- | --- | --- |
| 1 | selected snapshot missing from the provision | `Incomplete`, `unavailable_observation`/`missing-required-artifact` |
| 2 | snapshot of 1 MiB + 1 byte | `stage_limit_exceeded`, `input-bytes-exceeded` |
| 3 | `format` `native-state-input/1` | `unknown_wire` |
| 4 | an extra top-level member `note` | `invalid_runtime_input`/`unknown-member` |
| 5 | blank `authority` | `invalid_source_identity` |
| 6 | the snapshot's bytes edited (`child.versionNumber` `"3"`) but kept in the provision under the selected, original digest | `stale_dependency`/`byte-digest-mismatch` |
| 7 | `Current` selecting `VersionUnchanged` | `wrong_snapshot`/`wrong-observation` |
| 8 | the current snapshot's `observation` set to `pre` | `wrong_snapshot`/`wrong-observation` |
| 9 | `model.digest` of another package | `invalid_model_binding`/`wrong-model-selection` |
| 10 | invocation `operation` `other` | `wrong_snapshot`/`wrong-invocation` |
| 11 | a second object keyed `root` | `invalid_runtime_input`/`conflicting-identity` |
| 12 | `versionNumber` `{"boolean": true}` | `invalid_runtime_input`/`wrong-value-kind` |
| 13 | `root.versionNumber` `"-1"`; then `child.versionNumber` `"1001"` | `invalid_runtime_input`/`invalid-value`, naming `root` then `child` and `versionNumber` |
| 14 | `self` `{config_history, ghost}` | `invalid_runtime_input`/`wrong-role-mapping` |
| 15 | invocation `result` `null` | `invalid_runtime_input`/`missing-member` |
| 16 | `complete: false` and `child.parent` naming `missing` | `Incomplete`, `incomplete_population`/`incomplete-scope`, no dangling record |
| 17 | row 16 with `complete: true` | `dangling_reference`/`absent-target-in-complete-population`, naming `missing` and `config_history` |
| 18 | post `child.parent` absent | `frame_violation`/`unauthorized-change`, naming `child` and `parent` |
| 19 | a package whose `attemptUpdate` frame modifies only `parent`, with the changed-version invocation (post `child.versionNumber` 3) | `frame_violation`/`unauthorized-change`, naming `child` and `versionNumber` |
| 20 | invocation `created: [{config_history, child}]` | `population_delta_mismatch`/`delta-disagreement` |

Tag the tests `#[trace("TC-465", "FR-106-AC-n")]`.

## Expected Results

Each row gives exactly its expected record and nothing else. Every row but 1
and 16 is `Refused`. No row yields an `AdmittedObservations`.

## Status

Planned (QSL-273).
