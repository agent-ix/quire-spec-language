---
id: SR-13790
title: "base review of STD-159 task-set refusal codes (PR #1184)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@00e43d31d4bbdcbb2b73d8191a5862fd1e986d82; spec/functional/FR-246-check-task-sets-and-schedulability-claims.md, spec/test-cases/TC-701-task-sets-and-schedulability-claims-check.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-246
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-701
    type: reviews
---
# SR-13790: base review of STD-159 task-set refusal codes (PR #1184)

## Summary

Ticket: STD-159 (QSL half). Method `spec-review/base` over `git diff origin/main...00e43d31d`
(1 file, +9/-9): FR-246's invalid-task-set refusal bullets, its priority-clash bullet,
and FR-246-AC-2 now use QSpec `invalid_timed_model`/`invalid-task-set`, naming the
task, in place of `invalid_model_binding`/`malformed-declaration`. The paired QSpec PR
agent-ix/quire-specification#326 (b8cbeabdd) was read for consistency only. Sibling
artifacts of this review: SR-13791, SR-13792, SR-13793.

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
| FND-001 | medium | FR-246 now cites QSpec FR-419 for the priority rule, but the two disagree on an `edf` claim over two tasks of equal priority. FR-419 §Task sets makes equal priority a property of the task set ("A task set with ... two tasks of equal priority refuses with `invalid_timed_model`/`invalid-task-set`"), independent of the claim. FR-246 refuses the clash only under a `fixed-priority` or `amc` claim, says an `edf` claim ignores priorities, and FR-246-AC-2 requires "the same priority clash under `edf` checks". The code agrees with FR-246 (`check_priorities` runs only for fixed-priority, AMC and response claims). FR-419-AC-5 admits `edf` only for a missing priority, not for a clash. Align FR-419's prose with FR-246 in the paired QSpec PR (#326), or have FR-246 cite FR-419 only for the code and not for the rule. | spec/functional/FR-246-check-task-sets-and-schedulability-claims.md:77-80; spec/functional/FR-246-check-task-sets-and-schedulability-claims.md:94; quire-specification/spec/functional/analysis/FR-419-analyze-task-set-schedulability.md:50-56; qsl-replay/src/certificate/closed_form/task_set.rs:376-392 |
| FND-002 | low | The bullet cites QSpec FR-419 as the source of the code for five conditions, but FR-419 lists only three of them (non-positive `T` or `C`, `C(LO) > C(HI)`). A non-positive deadline and a `LO` task with dual budgets are this PR's lead default and appear in no QSpec text, and the paired QSpec PR #326 does not add them. As written, the citation implies that QSpec states all five. Either add the two conditions to FR-419's list in #326, or say in FR-246 that QSL refuses them under the same code. | spec/functional/FR-246-check-task-sets-and-schedulability-claims.md:70-74; quire-specification/spec/functional/analysis/FR-419-analyze-task-set-schedulability.md:50-53 |

## Verdict

Mergeable after the medium is fixed or deferred to the paired QSpec PR. The code change matches what the ticket allows: STD-159 asks which text is normative, and the PR body records QSpec's codes as a lead default, with the alternative stated. The new code and cause exist in QSpec's catalog (FR-272 and native-diagnostics list `invalid-task-set` under `invalid_timed_model`), and `Code::InvalidTimedModel` exists in `qsl-foundation`, so the follow-up code work can be built. Duplicate names, unknown response tasks and dimension errors keep their codes, which matches FR-419 and the code. No other QSL spec outside `reviews/` cites `malformed-declaration` or `invalid_model_binding` for a task set: FR-247, FR-248, FR-249, FR-252, US-026, ADR-026 RT-1 to RT-3 and RT-7 state no task-set code. The remaining `malformed-declaration` hits (FR-230/TC-685 tick period, FR-253 delay distributions) are about other declarations. FR-247, FR-248 and RT-7 are unchanged, as the brief says. Their miss evidence (task, job `q`, iterates) matches QSpec #326's new FR-419 fixed-priority text and its AC-2 and AC-7.
