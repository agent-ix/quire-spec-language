---
id: SR-12238
title: "Gap analysis of quire-spec-language PR #1119: binder environments, send records, Displaced and fence"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@9b307d66d18a2a01eae4a3b71886e30883240271; PR #1119 diff: qsl-eval/src/simulation/protocol/**, qsl-replay/src/model_check/{replay,counterexample}.rs, qsl-analyze/tests/it/{protocol_constructs,protocol_footprints,protocol_system}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-211
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-217
    type: reviews
---
# Gap analysis of quire-spec-language PR #1119

## Summary

Ticket: QSL-559. This analysis is planless (Plan completion: not assessed).
It is scoped to the ACs that the PR's diff touches. Each AC was read on
origin/main and compared with the test that claims it and with the code.

| AC | Test | Tag | Coverage |
| --- | --- | --- | --- |
| FR-205-AC-2 | `a_binder_is_held_by_each_environment_that_reaches_its_reader` | `TC-650`, `FR-205-AC-2` | Full. It covers all three variants (single attempt, `A` then `A2`, nested `R` all-join), and checks the holder and the lifetime in every state of every behaviour. |
| FR-205-AC-7 | `each_thread_reads_its_own_copy_of_a_rebound_binder` | `TC-650`, `FR-205-AC-7` | Full. It checks that `join(P)` copies `a` to the root and `fork(Q)` copies it to `u`. It checks that the re-fork of `P` takes ordinal 0 and the second `Q` takes ordinal 1. It finds a state where the two `u` hold different copies, where each `U1` settles over its own copy, and where no state refuses. Every holder is live, so a completed `u` drops its entry. A Boolean `a` stands in for `a.ready`, which does not change the substance. |
| FR-206-AC-7 | `a_send_binds_its_send_record` | `TC-651`, `FR-206-AC-7` | Full. It covers the four overflow policies, the record shape with `channel` and `displaced`, the binder holding the payload, and the resulting queues. |
| FR-206-AC-8 | `a_fence_moves_its_thread_and_changes_nothing_under_sc` | none ("Part of") | Partial, and correctly untagged. The `tso` clauses are not covered because the system refuses weak memory. |
| FR-211-AC-5 | `each_channel_wait_cause_names_its_condition` | none ("Part of") | Partial, and correctly untagged. It covers the channel causes, the three `Displaced` routes, `EmptyQueue` after a `receive`, and `FinishWaiting`. `CommitClosed`, `NoRoleInstance` and the `tso` fence `Gate` are not covered because those constructs are refused. |
| FR-216-AC-2 | `a_branch_end_writes_its_own_disposition` (TC-661) | `TC-661` | It adds the case where the last branch end removes the instance and writes the whole record. |
| FR-216-AC-5 | `records_guards_and_channel_anchors_are_read` and `an_enabling_footprint_…` | `TC-661` | Partial. It covers `send` visibility by channel. Template and role visibility are not covered because those constructs are refused. |
| FR-217-AC-5 | `a_protocol_replay_checks_each_send_record` | `TC-662`, `FR-217-AC-5` | Full. Only sends carry the record, the key and identity hold no record, it replays, and a changed `outcome` refuses `stale_dependency`/`content-mismatch` with both records. |
| FR-217-AC-6 | same | `TC-662`, `FR-217-AC-6` | Mostly. Both shape refusals are `invalid_runtime_input`/`invalid-value`. "Runs no step" holds in the code, but no test asserts it (FND-001). |

Soundness obligations (FT-4) were also checked:
`every_independent_pair_of_steps_commutes` covers the two new environment
fixtures, and each one has independent pairs. No code in the diff lacks an
owning requirement. No stub and no tautological assertion was found. One
test fixes a spec-violating identity (the fence's missing `ordering`); that
defect is recorded as FND-001 of SR-12237 and is not repeated here.

## Verdict

No blocking gap. Every tag claims an AC that is fully covered, and every
partial AC is left untagged with its uncovered clauses named. Two low gaps
are recorded.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-217-AC-6 says the shape refusals run "before replay … and run no step". The test asserts only the refusal value. A misplaced record on step 0 or step 1 would refuse the same way if it were checked during replay, so no test pins the order. Asserting it needs a prefix whose earlier step would refuse first, for example a corrupted digest on step 0 with a missing record on step 1, expecting `SendRecordShape`. | qsl-analyze/tests/it/protocol_system.rs:1553-1570; qsl-replay/src/model_check/replay.rs:552 |
| FND-002 | low | FT-5's new channel visibility is tested for `send` only. The `receive` arm of `visible` (`StepKind::Receive` with a `Queue` write) and the new FT-1 read of the joined branches' `ctl` at an `all` join have no direct assertion. The commutation test covers the join only indirectly. | qsl-eval/src/simulation/protocol/footprint.rs:219-226; qsl-eval/src/simulation/protocol/footprint.rs:446-450 |

## Dispositions

Round 1, reviewed at dccdc13b9735e5ad6df04b5ce5b7a457c538aa2e.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | dccdc13b9: a new case gives step 0 a stale post-state digest and removes step 3's record. It expects `SendRecordShape { step: 3 }`, which pins the check before replay. |
| FND-002 | fixed | dccdc13b9: `a_cancelling_join_writes_its_branches_and_a_queue_orders_its_steps` asserts that `receive` is visible to `Chatter::ch` and invisible to `Chatter::other`. It also asserts that `join(Both)` under `all` reads `ctl` of `s` and `r`. |
