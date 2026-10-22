---
id: SR-13792
title: "EARS analysis of STD-159 task-set refusal codes (PR #1184)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@00e43d31d4bbdcbb2b73d8191a5862fd1e986d82; spec/functional/FR-246-check-task-sets-and-schedulability-claims.md, spec/test-cases/TC-701-task-sets-and-schedulability-claims-check.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-246
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-701
    type: reviews
---
# SR-13792: EARS analysis of STD-159 task-set refusal codes (PR #1184)

## Summary

Ticket: STD-159 (QSL half). Method `spec-review/ears-conformance` over `git diff origin/main...00e43d31d`
(1 file, +9/-9): FR-246's invalid-task-set refusal bullets, its priority-clash bullet,
and FR-246-AC-2 now use QSpec `invalid_timed_model`/`invalid-task-set`, naming the
task, in place of `invalid_model_binding`/`malformed-declaration`. The paired QSpec PR
agent-ix/quire-specification#326 (b8cbeabdd) was read for consistency only. Sibling
artifacts of this review: SR-13790, SR-13791, SR-13793.

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
| FND-001 | low | The rewritten priority bullet makes the claim the subject of SHALL ("A `fixed-priority` or `amc` claim ... SHALL refuse"; "An `edf` claim SHALL ignore priorities"), but the claim is the input and the checker is the actor. The bullet two lines above, which this PR also rewrote, uses "The checker SHALL refuse". Use the event pattern with the system as subject, for example "When a `fixed-priority` or `amc` claim is over a task set where two tasks share a priority or a task has none, the checker SHALL refuse ...; for an `edf` claim the checker SHALL ignore priorities." quire's grammar lint does not flag this line. | spec/functional/FR-246-check-task-sets-and-schedulability-claims.md:77-80 |

## Verdict

Mergeable; one low. The main refusal bullet (lines 70-74) is a ubiquitous EARS statement with an explicit system subject, one SHALL, and an enumerated, testable list of conditions. `make check-spec-validation` reports no EARS or AC warning on FR-246 or TC-701.
