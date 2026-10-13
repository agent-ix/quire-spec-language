---
id: SR-8732
title: "QSL-745 spec review of PR #811"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@34a3c0a6e8ae0dbfd9cc8a68cd22c78c842a8df8; base impl/business-model-meanings; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md; spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md; spec/test-cases/TC-758-every-bound-is-a-caller-limit-that-names-itself-when-reached.md; context: FR-096, FR-100, FR-260, ADR-014 section 1"
review_set: subset
---
## Summary

Ticket: QSL-745. This is a base spec review of the PR's spec hunks: FR-277 (Description, Outputs, Behavior, AC-1), FR-255 Behavior 1 and AC-1, TC-452 step 4 and TC-758 step 2. They were read against the specs they must agree with: FR-096's key table, FR-100's outcome mapping, FR-260-AC-4 and ADR-014 section 1.

FR-255 now leaves only follow-up (c) (the QSpec catalog). AC-1 says "the QSL-745 follow-up", singular. A grep finds no stale reference to the dropped follow-ups (a) and (b).

## Verdict

FAIL. The PR gives three model causes catalog fields and records. It does not amend FR-096's key table or FR-100's mapping and AC-9, both of which still say these causes build no record.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The PR leaves two specs that contradict FR-255 Behavior 1 as edited, TC-452 step 4 and the code. FR-096's key table has no `ancestor-steps`, `family-steps` or `intake-limit-exceeded` row, and FR-096 states that the other `ModelQueryRefusal` causes build no record. FR-100 makes "builds a record" equivalent to "has an FR-096 key-table row". FR-100-AC-9 still gives `AncestorSteps`, `resource_exhausted`, exit 22 as the example of a family cause with no record and no `fields`. Add the three rows (`setting`, `bound`, `actual`) to FR-096's key table. Re-point FR-100-AC-9's no-record example to `TypeMismatch`, which `tc_452_step_4` already covers. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:38, 228-233; spec/functional/FR-100-run-a-named-function-through-the-spine.md:298, 309, 559 |
| FND-002 | medium | FR-277's carve-out names only `select`. Its Description and Behavior still say every other lifecycle limit path returns `LimitExceeded`. FR-255 Behavior 1, which is in force, makes `model.*` walks re-run in observation admission and `admission.*` caller budgets `resource_exhausted` wherever they are read, and observation.rs refuses that way under `replay`. Scope the `LimitExceeded` sentence to stage and reader limits types, or name every caller-budget path. | spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md:26-32, 65-68 |
| FND-003 | medium | TC-758 step 2's exception covers `format.output_bytes` and the `select` model-normalization fields, but not `intake.input_bytes`. The amended FR-277-AC-1 makes `intake.input_bytes` refuse `resource_exhausted`/`intake-limit-exceeded`, so as written the TC still expects `LimitExceeded` for that field. | spec/test-cases/TC-758-every-bound-is-a-caller-limit-that-names-itself-when-reached.md:31 |
| FND-004 | low | FR-277 cites "ADR-014 section 1" for `intake.input_bytes` being a caller budget. ADR-014 section 1 lists only accounting-contract types under B-2, and lists "the reader limits" under B-3. The intake classification comes from FR-255's table and FR-260. FR-277's relationships add neither ADR-014 nor FR-255. Cite FR-255 and FR-260 for intake, and add the edges. | spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md:29-32; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:118-122 |
| FND-005 | low | FR-255 Behavior 1's new sentence says `select` and the compile path give these limits "the same `resource_exhausted` code, cause and fields", with an unclear referent. The preceding sentence covers only observation admission's re-run of `model.*` walks, not `intake.input_bytes`. State the code, cause and field set for each case directly. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:141-144 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | FR-082 says the walk's `charge_point` is "`model.ancestor-edge`, or `admission.ancestor-edge` in population admission". Observation admission's re-run of `model.*` walks also runs in population admission. There the code, keyed on the setting, gives `model.ancestor-edge` with stage `model-normalization`, and `a_deep_hierarchy_is_judged_by_the_callers_ancestor_steps_not_a_fixed_ceiling` asserts that. Key the sentence on the limit: under `admission.ancestor_steps` (`PopulationAdmissionLimitsV1`, stage `population-admission`) the point is `admission.ancestor-edge`, otherwise `model.ancestor-edge`. | spec/functional/FR-082-resolve-conformance-subsetting-and-redefinition.md:152-156; qsl-semantics/src/model/refusal.rs:1166-1184 |

## Dispositions

Round 1, reviewed at a7be05cb81cb87eab1b83eee676595771d16b494.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | d060eb068: FR-096's key table adds a `ModelQueryRefusal` row for the walk-ceiling denied charge and one for `intake-limit-exceeded`, and its "no record" sentence excludes them. FR-100-AC-9's no-record example is now `TypeMismatch`, `ill_typed`, exit 20 (`refused_family_with_no_record_renders_code_and_cause_only`) |
| FND-002 | fixed | d060eb068: FR-277's Description, Outputs and Behavior classify by carrying type. A stage's or reader's own limits type gives `LimitExceeded`. A caller-budget ceiling gives `resource_exhausted` wherever it is read, including observation admission's re-run walks and `admission.*` |
| FND-003 | fixed | d060eb068: TC-758 step 2 adds `intake.input_bytes`, with `resource_exhausted`/`intake-limit-exceeded` and `kind`, `bound`, `actual`, `setting` |
| FND-004 | fixed | d060eb068: FR-277 cites FR-255 Behavior 1 and FR-260-AC-4 for intake, and adds FR-255, FR-260 and ADR-014 `references` edges |
| FND-005 | fixed | d060eb068: FR-255 Behavior 1 states the code, cause and field set for each case: the intake size ceiling, and the denied charge with its walk-ceiling values |
