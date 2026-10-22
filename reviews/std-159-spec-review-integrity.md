---
id: SR-13791
title: "integrity analysis of STD-159 task-set refusal codes (PR #1184)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@00e43d31d4bbdcbb2b73d8191a5862fd1e986d82; spec/functional/FR-246-check-task-sets-and-schedulability-claims.md, spec/test-cases/TC-701-task-sets-and-schedulability-claims-check.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-246
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-701
    type: reviews
---
# SR-13791: integrity analysis of STD-159 task-set refusal codes (PR #1184)

## Summary

Ticket: STD-159 (QSL half). Method `spec-review/integrity` over `git diff origin/main...00e43d31d`
(1 file, +9/-9): FR-246's invalid-task-set refusal bullets, its priority-clash bullet,
and FR-246-AC-2 now use QSpec `invalid_timed_model`/`invalid-task-set`, naming the
task, in place of `invalid_model_binding`/`malformed-declaration`. The paired QSpec PR
agent-ix/quire-specification#326 (b8cbeabdd) was read for consistency only. Sibling
artifacts of this review: SR-13790, SR-13792, SR-13793.

Read: QSpec FR-419 at #326 head (§Task sets, §Fixed priority, AC-2, AC-5, AC-7),
QSpec FR-272 and native-diagnostics (the `invalid_timed_model` cause list), `qsl-replay/src/certificate/closed_form/task_set.rs`
(`TaskSetRefusal`, `code()`, `cause()`, `check_priorities` call sites),
`qsl-analyze/tests/it/schedulability.rs`, ADR-026 RT-1 to RT-3 and RT-7, and every
QSL spec citing FR-246. Stale-statement sweep: a grep over `spec/` outside `reviews/`
for `malformed-declaration`/`invalid_model_binding` finds no task-set statement.

Gates (spec-only, per brief): `make check-spec-validation` exit 0 with no warning on FR-246 or TC-701; `tools/check-index-completeness.sh` exit 0; `quire validate --scope .` on FR-246 and TC-701 exit 0. The PR touches only `spec/functional/FR-246-...md`: no `docs/` file and no test-parsed vector table, so `make ci` is not needed. QSpec gates belong to the paired PR #326, which another reviewer covers; it was read for consistency only. Tools: quire 0.36.2 (engine 0.50.2), quoin 0.28.3.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-246 gives no rule for four task-set refusals the implementation already makes: an empty task set (`Empty`), a negative jitter or blocking term (`Negative`), an `edf` claim over a task with jitter or blocking (`EdfIgnoresTerm`), and an `amc` claim over a task whose deadline exceeds its period (`AmcDeadlineBeyondPeriod`). All four use `invalid_model_binding`/`malformed-declaration` today. The PR's remaining-work list moves `Negative` to `invalid-task-set`, which no spec text supports, and leaves the other three on `malformed-declaration`. After the code work, one invalid task set would therefore refuse under two catalog codes, and only one of them would be specified. State each of the four conditions in FR-246, with its code. | spec/functional/FR-246-check-task-sets-and-schedulability-claims.md:70-80; qsl-replay/src/certificate/closed_form/task_set.rs:148-262 |
| FND-002 | low | FR-246-AC-2 says the period, budget and priority-clash vectors "each refuse ... naming the task". A priority clash involves two tasks. The behaviour bullet says "naming the tasks", and `PriorityClash` carries `first` and `second`. Say "naming the task (both tasks for the clash)" so that a test asserting one name cannot pass for the clash. | spec/functional/FR-246-check-task-sets-and-schedulability-claims.md:79; spec/functional/FR-246-check-task-sets-and-schedulability-claims.md:94; qsl-replay/src/certificate/closed_form/task_set.rs:195-202 |

## Verdict

Mergeable after the medium is fixed. Ids are unchanged and unique: no new ids are added, and AC-1 and AC-2 keep their numbers. The statement and AC-2 now agree on the code of every vector, and the old AC-2 ("Refusals at the span" with no codes) is now specific. The wording describes behaviour only, with no changelog, status or review language. The duplicate, response and dimension bullets are unchanged and consistent with AC-2 and the code.

## Dispositions

Disposition pass 1, against agent-ix/quire-spec-language@692abe7b425d15a1a7a696d2a6aa2d343769a2e0 (fix commit 692abe7b4).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-246 now states all four conditions under `invalid_timed_model`/`invalid-task-set`, so every `TaskSetRefusal` variant (task_set.rs:148-229) has a specified code. After: "... a task set that declares no task; ... a jitter or blocking term that is negative; ..." and "When an `edf` claim is over a task with a jitter or blocking term other than zero, or an `amc` claim is over a task whose deadline exceeds its period, the checker SHALL refuse `invalid_timed_model`/`invalid-task-set`, naming the task, because the analysis does not read the term or does not cover that deadline." |
| FND-002 | fixed | AC-2 and TC-701 step 2 now require both names for the clash. AC-2 after: "Each of these refuses `invalid_timed_model`/`invalid-task-set` naming the task, and both tasks for the clash: ..." TC-701 after: "each refuses at its span with its stated code, naming the task (both tasks for the clash)". |

Disposition pass 2, against agent-ix/quire-spec-language@8432b364990bd1aa1c245836f10cb57ea683e468 (fix commit 8432b3649).

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | fixed | The refusal bullet, FR-246-AC-2 and TC-701's expected result now name the task set for the empty one. Bullet after: "The checker SHALL refuse, each with `invalid_timed_model`/`invalid-task-set` at the span, naming the task (the task set when it declares none): a task set that declares no task; ..." AC-2 after: "Each of these refuses `invalid_timed_model`/`invalid-task-set` naming the task (the task set for the empty one), and both tasks for a clash: a task set with no task; ..." TC-701 after: "naming the task (the task set for the empty one, both tasks for a clash)". `TaskSet::new` takes the set's name, so the code work can name it. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The refusal bullet and FR-246-AC-2 both say each listed condition refuses "naming the task", and both lists include a task set that declares no task. An empty set has no task to name, and `TaskSetRefusal::Empty` carries no task. A test cannot satisfy the AC as written for that vector. Say that the empty-set refusal names the task set, for example "naming the task (the task set when it declares none)". | spec/functional/FR-246-check-task-sets-and-schedulability-claims.md:70-72; spec/functional/FR-246-check-task-sets-and-schedulability-claims.md:105; spec/test-cases/TC-701-task-sets-and-schedulability-claims-check.md:29; qsl-replay/src/certificate/closed_form/task_set.rs:149-151 |
