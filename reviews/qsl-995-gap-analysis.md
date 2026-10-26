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

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | b056e2ce6 adds `the_tso_race_summary_holds_the_typed_values_of_the_message_passing_shape` (FR-224-AC-6). It asserts `race-summary`, writer `race-summaries` `cur`/`acq`/`rel(flag)`, element `race` for `data` and `flag`, `race-memory` after `flush` with a `release` and a `relaxed` store, `proved` with `release`/`acquire` and with `send`/`receive`, the queue entry's `race` member, and `refuted` with `relaxed`. It fails with `"view"` restored. FR-438-AC-7's reader-`cur` value is not asserted. |
| FND-002 | deferred | QSL-1119 (FR-224-AC-7, `tso` RMW). The fence, locked-RMW and channel carrier paths now have tests: `a_forged_race_through_tso_fences_and_a_locked_access_is_refused`, `a_forged_race_through_a_tso_channel_is_refused_and_a_real_one_replays`, and `seq_cst_fences_do_not_order_threads_under_tso`. AC-7 needs EN-1 to classify an RMW whose precondition is false initially as an application. That matches the `WeakAccessShape` "access local" refusal I hit with a precondition-gated reader in the QSL-932 round-3 probe. |
