---
id: SR-1596004
title: "Gap analysis of PR #1604: tso race summary"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@012638084; git diff origin/qsl-932-fix-round-2...012638084 (PR #1604): FR-224 tso criteria, FR-227-AC-3, TC-669, TC-672; ticket QSL-995"
review_set: subset
date: '2026-10-26'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-224
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-669
    type: reviews
---

## Summary

Ticket: QSL-995 ("FR-224-AC-2 tso half"). Plan completion: not assessed. The AC-to-test check was done by hand over the diff.

FR-224's `tso` criteria are AC-2's second sentence, AC-3, AC-6 and AC-7. AC-1 and AC-4 are `ra`-only.

AC-2's `tso` half is covered by `a_relaxed_flag_refutes_race_freedom_under_tso_and_the_race_replays`, which fails with the summary disabled. AC-6 and AC-7 under `tso` have no test.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-224-AC-2 | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md | examined |
| FR-224-AC-6 | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md | examined |
| FR-224-AC-7 | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md | examined |
| TC-669 | spec/test-cases/TC-669-race-freedom-item.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-224-AC-6 (the `tso` typed values) has no test, although the PR ships the encoding. The criterion covers the `race-summary`, `race-summaries`, element `race`, `race-memory` and queue-entry `race` values, and `proved` with `send`/`receive`. SR-1596003 FND-002's wrong member name would be caught by AC-6's queue-entry sentence. | qsl-eval/src/simulation/protocol/tso.rs:240-560 |
| FND-002 | medium | No test exercises the `tso` carrier rules for fences, RMW (FR-224-AC-7's `tso` half), `flush` of a release store, or send/receive. Replay's `tso` fence and locked paths are untested too, and SR-1596003 FND-001 sits in the fence path. | qsl-eval/src/simulation/protocol/tso.rs:80-140, 622-655; qsl-replay/src/model_check/race_replay.rs:311-336, 412-470 |

## Verdict

The `tso` half of AC-2 is covered and gated. AC-6 and AC-7 under `tso` are uncovered. The fence and RMW carrier paths are untested.
