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

Verify that each FR-106 admission condition fails alone with its code, cause
and input path, that completeness precedes closure, that the frame compares
scalar as well as reference fields, and that a document with several defects
reports the first by FR-106's order.

Scope: FR-106-AC-3, FR-106-AC-4, FR-106-AC-5, FR-106-AC-7.

## Test Procedure

Start from TC-464's healthy-parent snapshot and selection (for the `Current`
rows) or its changed-version invocation (for the `Invocation` rows). Apply one
mutation per row and admit. The provision digest is recomputed for every
mutated document except where the row says otherwise.

| # | Check | Mutation | Expected |
| --- | --- | --- | --- |
| 1 | 1.1 | selected snapshot missing from the provision | `Incomplete`, `unavailable_observation`/`missing-required-artifact` |
| 2 | 1.2 | snapshot of 1 MiB + 1 byte | `stage_limit_exceeded`, `input-bytes-exceeded` |
| 3 | 1.2 | a field value nested 65 `present` levels deep | `stage_limit_exceeded`, `nesting-depth-exceeded` |
| 4 | 1.3 | the snapshot's bytes edited (`child.versionNumber` `"3"`), kept under the original digest | `stale_dependency`/`byte-digest-mismatch` |
| 5 | 1.4 | `format` `native-state-input/1` | `unknown_wire`/`unsupported-wire` |
| 6 | 1.5 | `populations` removed | `invalid_runtime_input`/`missing-member` |
| 7 | 1.6 | an extra top-level member `note` | `invalid_runtime_input`/`unknown-member` |
| 8 | 1.7 | blank `authority` in document and selection | `invalid_runtime_input`/`invalid-value` at `authority` |
| 9 | 1.8 | the selection's `revision` `2` | `stale_dependency`/`revision-mismatch`, naming both |
| 10 | 2 | `Current` selecting `VersionUnchanged` | `wrong_snapshot`/`wrong-observation` |
| 11 | 3 | the current snapshot's `observation` set to `pre` (and `anchor` removed) | `wrong_snapshot`/`wrong-observation` |
| 12 | 3 | the selection's anchor `{handler, other}` | `wrong_snapshot`/`wrong-anchor`, naming both |
| 13 | 4 | `model.digest` of another package | `invalid_model_binding`/`wrong-model-selection` |
| 14 | 5 | invocation `operation` `other` | `wrong_snapshot`/`wrong-invocation` |
| 15 | 6.1 | population `ix://example/config-version/other` | `invalid_runtime_input`/`wrong-role-mapping` |
| 16 | 6.3 | a second object keyed `root` | `invalid_runtime_input`/`conflicting-identity` at the second |
| 17 | 6.2 | a package variant whose `ConfigVersion` has field `tags` typed a set of `ConfigVersion` | `unknown_required_feature`/`unsupported-feature` at `tags` |
| 18 | 6.5 | `versionNumber` `{"boolean": true}` | `invalid_runtime_input`/`wrong-value-kind` |
| 19 | 6.5 | `root.versionNumber` `"01"` | `invalid_runtime_input`/`invalid-value` at `root`, `versionNumber` |
| 20 | 6.5 | `root.versionNumber` `"-1"`; then `child.versionNumber` `"1001"` | `invalid_runtime_input`/`invalid-value`, naming `root` then `child` and `versionNumber` |
| 21 | 9 | `self` `{config_history, ghost}` | `invalid_runtime_input`/`wrong-role-mapping` |
| 22 | 10 | invocation `result` `null` | `invalid_runtime_input`/`missing-member` |
| 23 | 7 | `complete: false` and `child.parent` naming `missing` | `Incomplete`, `incomplete_population`/`incomplete-scope`, no dangling record |
| 24 | 8 | row 23 with `complete: true` | `dangling_reference`/`absent-target-in-complete-population`, naming `missing` and `config_history` |
| 25 | 7 | healthy-parent plus a second population `ix://example/config-version/archive` over `ConfigVersion`, `complete: false`, that no reference names | admitted |
| 26 | 11.3 | post `child.parent` absent | `frame_violation`/`unauthorized-change`, naming `child` and `parent` |
| 27 | 11.3 | a package whose `attemptUpdate` frame modifies only `parent`, with the changed-version invocation (post `child.versionNumber` 3) | `frame_violation`/`unauthorized-change`, naming `child` and `versionNumber` |
| 28 | 11.4 | invocation `created: [{config_history, child}]` | `population_delta_mismatch`/`delta-disagreement` |
| 29 | 1.3 over 1.6 | row 4's edit plus row 7's extra member, under the original digest | `stale_dependency`/`byte-digest-mismatch` |
| 30 | 6.5 walk order | `root.versionNumber` `"-1"` and `child.versionNumber` `"1001"` together | `invalid_runtime_input`/`invalid-value` at `root` |
| 31 | 11.2 over 11.3 | post deletes `root` and sets `child.parent` absent | `frame_violation`/`unauthorized-change` naming the deletion of `root` |
| 32 | 6.1 | a package variant that adds object type `Note`, a member type of no population, and an object `n1` of type `Note` in `config_history` | `invalid_runtime_input`/`wrong-role-mapping` at `n1` |
| 33 | 6.4 | `root` without its `parent` field | `invalid_runtime_input`/`missing-member` at `root`, `parent` |
| 34 | 6.4 | `root` with an extra field `label` `{"boolean": true}` | `invalid_runtime_input`/`unknown-member` at `root`, `label` |
| 35 | 10 | probe invocation with `parameters` `{}` | `invalid_runtime_input`/`missing-member` at `target` |
| 36 | 10 | probe invocation with an extra parameter `other` naming `{config_history, root}` | `invalid_runtime_input`/`unknown-member` at `other` |
| 37 | 10 | probe invocation with `target` `{"integer": "1"}` | `invalid_runtime_input`/`wrong-value-kind` at `target` |
| 38 | 10 | probe invocation with `result` `{"boolean": true}` | `invalid_runtime_input`/`unknown-member` at `result` |
| 39 | 11.1 over 11.2 | a package variant where `Sub` specializes `ConfigVersion` and is a member type of `config_history`; post changes `child`'s type to `Sub`, sets `child.parent` absent and deletes `root` | `frame_violation`/`unauthorized-change` naming `child`'s type change |
| 40 | 11, population order | pre and post list `archive` (complete, object `a1` with `parent` absent) before `config_history`; post sets `a1.parent` to `{archive, a1}` and `child.parent` absent | `frame_violation`/`unauthorized-change` naming `a1` and `parent` in `archive` |

Rows 25 and 40 need the fixture package to declare a second population
`archive` over `ConfigVersion` with a maximum of 10. With no maximum, every
clause over `ConfigVersion` would refuse at S3, because its extent could not
name exactly one population (FR-104-AC-5, TC-461 step 5). Rows 35 to 38 use
TC-466 step 3's `probe` package variant: `probe(target: ConfigVersion)` on
`ConfigVersion`, no result, empty frame, with `ReachesTarget` selected. Their
base invocation has `operation` `probe`, `self` `{config_history, child}`,
`parameters` `{"target": {"reference": {"population":
config_history, "key": "root"}}}`, `result` `null`, and the healthy-parent
snapshot as both pre and post. Tag the tests
`#[trace("TC-465", "FR-106-AC-n")]`.

## Expected Results

Each row gives exactly its expected record and nothing else. Rows 1 and 23
are `Incomplete`, row 25 admits, and every other row is `Refused`. No failing
row yields an `AdmittedObservations`.

## Status

Planned (QSL-273).
