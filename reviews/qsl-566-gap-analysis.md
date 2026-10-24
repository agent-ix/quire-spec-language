---
id: SR-12256
title: "Gap analysis of QSL-566 E16b ProtocolSystem<Ra> (PR #1141)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@233c0191d0a3df6a21589ebadeb17c7aa0a6698e; git diff origin/main...HEAD (production: qsl-eval/src/simulation/protocol/*, qsl-analyze/src/model_check/check.rs, qsl-replay/src/*; tests: qsl-analyze/tests/it/release_acquire.rs); FR-222, FR-223, FR-227, FR-229 and the declared remainder FR-224, FR-225, FR-228; ticket QSL-566"
review_set: subset
date: '2026-10-21'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-222
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-227
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-229
    type: reviews
---

## Summary

Ticket: QSL-566 (Implements FR-222, FR-223, FR-227, FR-229). This review is planless, so plan completion was not assessed. The criterion-to-test map was read from the `#[trace]` tags in the tree at head. `quoin matrix` did not finish within the review's time budget, so the map was not computed.

| AC | Test (release_acquire.rs) | Result |
| --- | --- | --- |
| FR-222-AC-1 | store_buffering_under_ra_reaches_18_states_and_is_refuted, a_proved_ra_run_states_its_message_bound | tagged; counterexample clause diverges (SR-12255 FND-004) |
| FR-222-AC-2 | the_ra_column_of_the_litmus_table_decides_each_weak_outcome | covered, all 8 rows |
| FR-222-AC-3 | read_modify_writes_chain_and_unreachable_messages_are_removed | covered |
| FR-222-AC-4 | fences_and_channels_pass_a_message_under_ra | covered |
| FR-223-AC-1 | seq_cst_store_buffering_prunes_the_old_message_choice | covered |
| FR-223-AC-2 | seq_cst_iriw_is_forbidden_and_release_acquire_iriw_is_not | covered |
| FR-223-AC-3 | the_sc_event_graph_stays_within_the_live_messages | covered |
| FR-223-AC-4 | the_sc_event_graph_is_deterministic_and_coalesces_removed_events | covered |
| FR-227-AC-2 | an_ra_counterexample_replays_by_its_message_depths | covered (SR-12255 FND-007) |
| FR-229-AC-1 (ra half) | store_buffering_under_ra..., the_memory_member_is_empty_outside_a_weak_parallel | covered |
| FR-229-AC-3 | a_read_modify_write_that_changes_nothing_applies_as_a_load | covered |
| FR-229-AC-4 (ra half) | ra_steps_carry_message_and_view_footprints | covered |

No stubs, no tautological asserts, and no production code without an owning requirement. Every new production path in `ra.rs` and in the seam traces to RA-1..RA-8, PSC-1..PSC-6 or MR-1.

## Verdict

Coverage of the in-scope ACs is complete apart from SR-12255 FND-004. The gaps are in the tracking of the remainder and in two spec rules that the engine invented and did not disclose.

The remainder is FR-224's race summary under `ra`, FR-227-AC-3, FR-228's `ra` visibility member and FR-225's `max_messages` request member. It is correctly out of QSL-566's FRs, except FR-227-AC-3, and FR-227 is one of QSL-566's own FRs. But the two tickets that own the rest are both Done: QSL-565 (FR-225, FR-227) and QSL-567 (FR-224, FR-227, FR-228). QSL-567 is blocked by QSL-566 and was closed with its `ra` parts unbuilt. Nothing open owns the remainder.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The declared remainder has no open owner, because QSL-565 and QSL-567 are both Done. The remainder is: the FR-224 race summary under `ra` (DR-3) with FR-224-AC-1/2/4 `ra` halves and FR-227-AC-3; FR-228-AC-1's `ra` visibility member (`Ra::fairness` returns none); and FR-225-AC-1/AC-3 `ra` halves with the `max_messages` request and certificate member. Reopen the owning tickets, or record the remainder on QSL-566 and keep it open | qsl-eval/src/simulation/protocol/ra.rs:1415-1419, qsl-eval/src/simulation/protocol/selection.rs:309-325 |
| FND-002 | medium | Two spec gaps the engine depends on are not filed. First, ADR-025 PSC-4 has no rule for a removed SC event's predecessors toward its thread's later events on other locations (SR-12255 FND-001). Second, PSC-4 and PSC-6 give SC fences no lifetime: fences are on no message, so only the engine's own supersession bounds them (SR-12255 FND-002). File both as STD tickets beside STD-239..241 | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:590-592 |
| FND-003 | low | The module doc says that for fences only `po` and fence-to-fence `hb` are built. STD-239 names only the `eco` part and accesses ordered before a fence through a non-SC event. It does not name the unbuilt `[Fsc]; hb; po; [Esc]` edge from a fence to a later SC access of a thread that synchronised with it, which is decidable from the frontier | qsl-eval/src/simulation/protocol/ra.rs:32-36, ra.rs:601-605 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | Tracker, not code (no sha). The remainder now has open owners related to QSL-566, verified Backlog in Linear: QSL-932 (FR-224 race summary under `ra`, FR-227-AC-3), QSL-933 (FR-228-AC-1 `ra` visibility), QSL-934 (FR-225 `max_messages` request member) |
| FND-002 | fixed | Tracker, not code (no sha). Filed STD-246 (PSC-4 predecessors of a removed event, for its thread, with the residual `hb\|loc` case) and STD-247 (SC fence lifetime) |
| FND-003 | fixed | Tracker, not code (no sha). STD-247 names the unbuilt `[Fsc]; hb; po; [Esc]` edge and the fence-lifetime rule that building it needs. The `ra.rs` module doc cites STD-246 and STD-247 |
