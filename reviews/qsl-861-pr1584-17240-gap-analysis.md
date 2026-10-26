---
id: SR-17240
title: "QSL-861 PR #1584 gap analysis: FR-246 against TC-701 and the task-set front end"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3fd0af8e230442634534179a47b2d20865a8d1b5; FR-246 (Behavior, AC-1 to AC-3), FR-247 (EN-7 manifest routing only), TC-701 (by id); the diff's traced tests in qsl-cst, qsl-forms, qsl-semantics and qsl-analyze"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-246
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-701
    type: references
---

## Summary

Ticket: QSL-861. PR quire-spec-language#1584 at head 3fd0af8e2. Planless gap analysis scoped to the diff. Plan completion: not assessed. Criteria were traced to tagged tests by reading; the full suite was run on the head merged with main 392db214a (green).

| Criterion | Test | Assertion | Verdict |
| --- | --- | --- | --- |
| FR-246-AC-1 | `ctl_checks_and_its_claims_carry_the_schedulability_kind`, `en_7_registers_and_a_schedulability_claim_routes_to_it` | check and routing halves only; identity (third WCET 6 gives another identity) not asserted | correctly untagged: deferred to QSL-1109, and the comment says so |
| FR-246-AC-2 | `each_defect_refuses_with_its_code_naming_its_task` | all 17 invalid cases with code, cause, names and span text; duplicate name, `zz`, `3 m`; the clash and no-priority-under-`edf` check | bound, real; the tag is correct, the whole AC is asserted |
| FR-246-AC-3 | `the_unit_is_the_first_task_s_and_every_value_converts_into_it`, `every_defect_is_reported_in_the_stated_order` | unit `us`, conversions, the four-refusal order, duplicate then `z`, `Empty`; fixpoints and "keeps the bound 12 ms" not asserted | correctly untagged: fixpoints are FR-248 and the bound check is unobservable (SR-17239 FND-002) |
| S1/S2 productions | taskset.rs (cst), task_set_forms.rs, complete_grammar corpus row | productions, spans, signed values, admitted-but-refused forms | bound, real |

No stub, no tautology and no assertion-free test found. Tests carry `#[trace("TC-701")]` and only AC-2 carries an AC id. One junk line (`let _ = (TaskSetDefect::NoTask, TaskTimeMember::Period);`) exists to use imports.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The facet gate is untested. S3 refuses `taskset`, `schedulable` and `response` with `unsupported_construct`/`TimedFacetRequired` (new `TimedConstruct` variants) unless `quire.temporal.complete/v1` is selected, but no test omits the profile; the neighbouring timed constructs have one (timed_declarations.rs:574). Add a case per construct. | qsl-semantics/src/check/mod.rs; qsl-semantics/tests/it/task_sets.rs | correct-requirement-no-evidence |
| FND-002 | medium | The two refusals the PR adds beyond the spec, `MissingTaskSet` and `DuplicateTaskSet`, and the `PriorityOutOfRange` defect, have no test (probed with a temporary test, reverted: a claim on an unknown set gives `missing_declaration`/`missing-name` at the set name; a second set of one name gives `conflicting_declaration`/`duplicate-declaration` at its name and its claims resolve to the first; a priority past u64 refuses once). Add them. | qsl-semantics/src/check/task_set.rs | correct-requirement-no-evidence |
| FND-003 | low | Contextual words are shown for `taskset` only (qsl-cst/tests/it/taskset.rs). The words with a leading position (`response`, `schedulable`) and the member words (`task`, `lo`, `hi`, `deadline`, `wcet_hi`, `periodic`) as parameter, function and task names parse today (probed, reverted) but nothing pins it, unlike #1578's per-word test. | qsl-cst/tests/it/taskset.rs | correct-requirement-no-evidence |
| FND-004 | low | Exact arithmetic is asserted for integers only. A fractional conversion (`1500 us` with `deadline rational(3,2) ms` giving unit `us` and 1500, or a `ms` unit with a 3/2 value), a `decimal` magnitude, a magnitude past the rational-bits ceiling (`ResourceExhausted`), `rational(1,0)` and an out-of-range priority are untested; they behave correctly under a temporary probe. | qsl-semantics/tests/it/task_sets.rs | correct-requirement-no-evidence |
| FND-005 | low | The identity half is not claimed anywhere (FR-454, FR-248-AC-4, FR-246-AC-1 identity clause, AC-3 fixpoints); the deferral is recorded in the AC-1 test comment and QSL-1109. | | correct-requirement-no-evidence |

## Verdict

No code lacks an owning requirement and the AC tags are correct. FND-001 and FND-002 are evidence gaps on behaviour this PR adds; fix with the PR. FND-003 and FND-004 are low.
