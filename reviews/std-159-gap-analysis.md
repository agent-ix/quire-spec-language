---
id: SR-13793
title: "gap analysis of STD-159 changed criteria against test cases (PR #1184)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@00e43d31d4bbdcbb2b73d8191a5862fd1e986d82; spec/functional/FR-246-check-task-sets-and-schedulability-claims.md, spec/test-cases/TC-701-task-sets-and-schedulability-claims-check.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-246
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-701
    type: reviews
---
# SR-13793: gap analysis of STD-159 changed criteria against test cases (PR #1184)

## Summary

Ticket: STD-159 (QSL half). Method `gap-analysis` over `git diff origin/main...00e43d31d`
(1 file, +9/-9): FR-246's invalid-task-set refusal bullets, its priority-clash bullet,
and FR-246-AC-2 now use QSpec `invalid_timed_model`/`invalid-task-set`, naming the
task, in place of `invalid_model_binding`/`malformed-declaration`. The paired QSpec PR
agent-ix/quire-specification#326 (b8cbeabdd) was read for consistency only. Sibling
artifacts of this review: SR-13790, SR-13791, SR-13792.

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
| FND-001 | low | The rewritten bullets move seven conditions to `invalid-task-set`, but FR-246-AC-2 and so TC-701 exercise three of them: a zero period, `C(LO) > C(HI)`, and a priority clash. A non-positive deadline, a non-positive WCET, a `LO` task with dual budgets and a missing priority under `fixed-priority` have no vector. Two of these (deadline, LO dual budgets) are this PR's lead default and are stated nowhere else, and the code work will change their code. Add one vector each to AC-2, or at least the deadline, LO-dual and missing-priority vectors. | spec/functional/FR-246-check-task-sets-and-schedulability-claims.md:70-80; spec/functional/FR-246-check-task-sets-and-schedulability-claims.md:94; spec/test-cases/TC-701-task-sets-and-schedulability-claims-check.md:22-29 |

## Verdict

Mergeable; one low. Plan completion: not assessed. The only changed criterion, FR-246-AC-2, has a TC row: TC-701 verifies FR-246, its scope is FR-246-AC-1 to AC-2, and step 2 checks each AC-2 refusal "with the stated code", so TC-701 needs no edit for the new codes. The brief says TC-701 was updated, but the PR does not touch it, and none is needed. Spec-only analysis: by the brief's rule, no `quire matrix` or cargo run. Context from reading the code: the existing admission test `a_task_set_or_claim_that_breaks_a_rule_is_refused_with_its_code` (qsl-analyze/tests/it/schedulability.rs:169) asserts `Code::InvalidModelBinding`/`malformed-declaration` and carries no `#[trace]` tag, so FR-246-AC-2 has no bound test today. The PR's code follow-up must change those assertions and should tag the test `TC-701`/`FR-246-AC-2`. A refusal probe was not run: the new codes are not implemented yet, so a probe would only confirm the old behaviour that the test already asserts, and the brief rules out QSL cargo runs here.
