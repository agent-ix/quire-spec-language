---
id: SR-777
title: "QSL-301 gap analysis of PR 514 (FR-116 against TC-515)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@611da08ee299621e328b0fd56f9f70d9c896dd79; spec/functional/FR-116-replay-a-frame-counterexample.md; spec/test-cases/TC-515-replay-a-frame-counterexample.md; spec/functional/FR-115-run-an-operation-frame-over-an-invocation.md (read); spec/functional/FR-106-admit-snapshots-and-invocations.md check 11 (read); spec/decisions/ADR-012-semantic-family-extension-contracts.md §12.2 (read); spec/tests.md; qsl-replay/src/spine/clause/tests/frame_replay.rs; qsl-replay/src/witness/frame.rs; qsl-replay/src/execute/frame.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-515
    type: reviews
---
## Summary

Ticket: QSL-301. PR: quire-spec-language#514 at 611da08e. This review maps
each FR-116 acceptance criterion and Behavior bullet to TC-515's tests
(`qsl-replay/src/spine/clause/tests/frame_replay.rs`) and to the code.

| AC | Test(s) | Does the test tell success from failure? |
| --- | --- | --- |
| FR-116-AC-1 | `a_frame_counterexample_reproduces_and_keeps_its_identities`, `an_input_arm_envelope_settles_the_input_arm`, `a_check_time_frame_violation_replays_to_the_same_witness` | Yes. The anchor and frame are checked against the emitted package's own `operation_anchor`/`frame` nodes, found independently of the code under test. It checks the settlement, category, source digest and identity, `package_id`, all three `DocumentRef`s, and both changes. The end-to-end test did not check what `of_witness` decoded to (FND-002). |
| FR-116-AC-2 | `a_frame_respecting_invocation_is_inconclusive_by_verdicts`, `a_mismatched_claim_reproduces_keeping_both_changes`, `a_disagreeing_declared_delta_is_inconclusive_with_no_value` | Yes. It checks exact `DisagreementCause::Verdicts{violation, success}` and `NoValue{violation, refusal}`, no record or value, and the claimed and found changes kept apart. |
| FR-116-AC-3 | `a_stale_frame_identity_refuses_before_admission`, `a_stale_anchor_or_occurrence_refuses_revision_mismatch`, `a_source_edit_refuses_by_the_stale_package_rule`, `a_missing_operation_refuses_missing_name` | Partly. The stale-frame test built an envelope no producer can emit (FND-001 of SR-776). The source-edit test is refused inside `recompile` by the request's `package_id`, so `replay_frame`'s own envelope `package_id` check was never run (FND-001). The pre snapshot is removed in the stale-frame test, which proves the identity refusal comes before admission. |
| FR-116-AC-4 | `an_absent_pre_snapshot_refuses_with_the_admission_record`, `edited_invocation_bytes_refuse_byte_digest_mismatch` | Yes. Exact `Admission(Incomplete)` with `unavailable_observation`. The byte edit is refused by FR-098's request decode, which comes first, with `byte-digest-mismatch`. |
| FR-116-AC-5 | `replaying_one_envelope_twice_gives_equal_results` | Yes. It compares whole `FrameReplayResult`s (derived `Eq`). A generic helper bound on `P: FamilyPayload` proves the trait bound at compile time. |

Other items checked:

- `ClaimedChange::of_witness` has a unit test for all four `FrameChange`
  variants, retyping included (`witness/frame.rs`).
- IR/CG boundary. FR-116 Behavior's only waiting item is "the decode from
  IR's witness", which ADR-012 §12.2 gives to CG ("the frame witness bindings
  CG builds over the IR-owned `WitnessBinding`"). No other QSL work in FR-116
  is left undone. `WitnessPacket<FrameCounterexample>` has public fields, and
  `WitnessEnvelope::reconstruct` is public, so an outside crate can build a
  real envelope. There is no byte-level envelope wire format yet, and CG does
  not link `qsl-replay` today. That is also true of FR-098's `NoPayload` path,
  and FR-116 scopes its input to "the in-process envelope", so it is not a
  gap in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No test reaches `replay_frame`'s envelope `package_id` check (the PR's own FR-116 addition, "the request's or the envelope's"). `a_source_edit_refuses_by_the_stale_package_rule` sets the request's and the envelope's `package_id` to the same stale value, so `recompile` refuses first. Add a case where only the envelope's `package_id` is stale. | qsl-replay/src/execute/frame.rs:174-179; qsl-replay/src/spine/clause/tests/frame_replay.rs:611-642 |
| FND-002 | low | `a_check_time_frame_violation_replays_to_the_same_witness` passes `of_witness`'s output through the replay and compares it only with itself (`result.claimed() == claimed`). A decode that named the wrong population or key would still pass. Pin it to the expected claim (`child`, `parent`). | qsl-replay/src/spine/clause/tests/frame_replay.rs:789-797 |

## Verdict

PASS after fixes. Every FR-116 AC has a test. The two gaps were test
weaknesses, not missing behaviour. Both are fixed in-PR.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 975993fd: `a_stale_envelope_package_id_refuses_by_the_stale_package_rule`: the request's `package_id` is current and the envelope's is the parent-modifying package's; asserts `PackageIdMismatch{requested: envelope's, recompiled: current}` |
| FND-002 | fixed | 975993fd: the end-to-end test asserts `of_witness(witness) == child_change("parent")` before replaying |
