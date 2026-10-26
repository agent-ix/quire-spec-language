---
id: SR-1483004
title: "Gap analysis of PR #1590: ra race summary, race-freedom item and race replay"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@a307ab9ef; git diff origin/main...a307ab9ef (PR #1590): FR-224 (ra halves of AC-1, AC-2, AC-4), FR-227-AC-3, TC-669, TC-672 and the production code the diff adds; ticket QSL-932"
review_set: subset
date: '2026-10-26'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-224
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-227
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-669
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-672
    type: reviews
---

## Summary

Ticket: QSL-932. Plan completion: not assessed. `quoin matrix --json` did not finish within 300 s, so the AC-to-test check was done by hand over the diff. Every criterion in scope has a tagged test that asserts it:

- FR-224-AC-1, ra half: `a_release_acquire_flag_proves_race_freedom_under_ra`.
- FR-224-AC-2, ra half: `a_relaxed_flag_refutes_race_freedom_under_ra_and_the_race_replays`.
- FR-224-AC-4: `a_race_changes_no_authored_claims_verdict_under_ra`.
- FR-227-AC-3: the relaxed-flag replay and `a_forged_race_settles_inconclusive_under_ra`.

By the ticket's scope, the following stay open for the follow-up work: FR-224-AC-2's tso half, AC-3, AC-5, AC-6 and AC-7. The gaps below are production code this PR ships with no test behind it.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-224-AC-1 | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md | examined |
| FR-224-AC-2 | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md | examined |
| FR-224-AC-4 | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md | examined |
| FR-227-AC-3 | spec/functional/FR-227-carry-and-replay-a-weak-memory-counterexample.md | examined |
| TC-669 | spec/test-cases/TC-669-race-freedom-item.md | examined |
| TC-672 | spec/test-cases/TC-672-replay-a-weak-memory-counterexample.md | examined |
| FR-224-AC-5 | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md | context_only |
| FR-224-AC-7 | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Several race replay branches ship with no test: fence, RMW, nested `fork`/`join` and `send`/`receive`. Only the plain store/load message-passing path is exercised, and SR-1483003 FND-001, FND-003 and FND-009 all sit in the untested branches. | qsl-replay/src/model_check/race_replay.rs:107-259 |
| FND-002 | medium | Queued-send carriers ship with no test. `carry_after`/`carried_after`, which clears a queued send's race map on later accesses, and the receive-side join of race maps are covered by no test in this PR. Their AC, FR-224-AC-5's channel variant, is deferred. | qsl-eval/src/simulation/protocol/steps.rs:1323-1335; qsl-eval/src/simulation/protocol/ra.rs:1589-1597 |
| FND-003 | low | No test reaches the new refusal and rejection branches: `RaceOverModel` in EN-1 and in replay, the SMT `item-form` refusal for the race item, and the closure certificate's `BadState` rejection of a racy edge. | qsl-analyze/src/model_check/check.rs:1042, 1353; qsl-replay/src/model_check/replay.rs:1047; qsl-replay/src/certificate/closure.rs:712-720 |

## Verdict

Every criterion in scope is covered, and the trace bindings are correct. The untested branches are where SR-1483003's high and medium replay defects live. When those defects are fixed, each fix should land with a test of its branch (a fence-based message passing forged race, and a lossy-channel forged race).
