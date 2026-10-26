---
id: SR-1639002
title: "gap analysis of PR #1639 (QSL-583a, FR-240 time-lock freedom on zones)"
type: SpecReview
analysis: gap-analysis
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1639 git diff origin/qsl-582-cert-checker...390f7abb0; FR-240-AC-3, FR-236-AC-2, FR-239-AC-5, FR-237-AC-4, FR-235-AC-3; TC-695, TC-691, TC-710, TC-692, TC-690"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-240
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-237
    type: reviews
---
# SR-1639002: gap analysis of PR #1639 (QSL-583a, FR-240 time-lock freedom on zones)

## Summary

Ticket: QSL-583. Head: 390f7abb0. Method: a manual check of each AC against its tests, with probes. Plan completion: not assessed.

| AC | PR claim | Measured |
| --- | --- | --- |
| FR-240-AC-3 | covered | Covered. The strict `Rpc` gives `send` at 0 and final delay 3. The unmodified `Rpc` holds at T = 3 and T = 4. `Stall` is violated and confirmed by fresh exploration. Its tag `FR-240-AC-3` is correct. |
| FR-236-AC-2 | covered | Covered: one step `send` with delay 0, `final_delay` 3, `TimeLock`, no lasso, and a `Formula` counterexample has `final_delay: None`. |
| FR-239-AC-5 | covered (now whole) | Covered. The time-locked `Ticker`'s claim is `NoAdmittedBehaviour`, and its time-lock item is refuted (initial state, final delay 0). The live `Ticker` holds. |
| FR-237-AC-4 | partial | Partial, as claimed. The test is tagged `TC-692` only. Both halves of STD-493 gap 3 are real spec gaps (see below). |
| FR-235-AC-3 | covered | Covered. The strict-guard time-lock now settles `refuted`, `closed-scope`, from EN-6's own counterexample, which equals the hand-built one. |

**The FR-237-AC-4 spec gap is real in both halves.**

- **(a) The `final_delay`.** FR-241's least-delay rule picks the least delay into the trapped set. At `Stall`'s initial node the whole zone `x ∈ [0, 1]` is trapped, so EN-6 reports 0, not the AC's 1/2. Both payloads reproduce, and the test replays both.
- **(b) The one-state limit.** From `x = 1/2`, replay's fresh exploration is a single zone node, `x ∈ [1/2, 1]`. `ping` loops back onto the same zone, and `z >= 1` is empty because `z - x = -1/2` and `x <= 1`. So `max_states` 1 is never exceeded and the item reproduces; the test shows this. A V-7 needs `max_transitions` 0, which the test uses. The AC's expected V-7 under a one-state limit cannot occur, so the AC needs rewording.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The per-valuation trap, which is the point of the departure, is tested only on `Hold` (one clock, one partial-zone shape). Several cases have no test: a two-clock diagonal trap, an open-boundary trap (`go < 1`, giving final delay 1), a Zeno reset loop under `x <= 0`, a trap reached through a step, and the tick-source branch of `traps()` / `divergent_nodes`. All of these pass in my probes (all reported locks replay, all Holds are right), so the fix is cheap: add them as regression tests, each asserting the outcome and that the reported counterexample replays. | qsl-analyze/src/zone_check/time_lock.rs:125-150,284-298; qsl-analyze/tests/it/zone_search.rs:475-516 |
| FND-002 | low | No test shows FR-240's "local before non-local at one node" order with both kinds present at one node. Nor does any test show that the first lock in canonical order is the one reported when a later node also has a lock. One small model with a local lock and a trap at the same node would pin it. | qsl-analyze/src/zone_check/time_lock.rs:96-122 |

## Verdict

**Mergeable**, together with SR-1639001. Every claimed AC is covered as stated. FR-237-AC-4 is partial, and both halves of its spec gap (STD-493 f4765723 item 3) are real. FND-001 is the missing regression coverage for the PR's central change. The probes for it already pass, so adding them is cheap.

## Coverage

| Test | Tag | Binding |
| --- | --- | --- |
| `time_locks_are_found_local_and_non_local` | FR-240-AC-3 | correct |
| `a_time_lock_counterexample_carries_its_final_delay` | FR-236-AC-2 | correct |
| `an_undefined_atom_refutes_only_where_time_diverges` | FR-239-AC-5 | correct |
| `a_non_local_time_lock_reproduces_by_fresh_exploration` | TC-692 | correct (partial AC-4, untagged) |
| `a_timed_refutation_settles_through_its_replay` | FR-235-AC-3 | correct |

## Dispositions

Round 1, head af99b38b4. `zone_` tests: 63/63 passed.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `trapped_valuations_of_every_shape_are_found` covers a diagonal trap reached by a step, an open boundary (least delay 1), a Zeno reset loop and the tick-source branch. Each lock goes through `reproduced_time_lock`, which asserts that FR-237's replay gives `Reproduced`. |
| FND-002 | fixed | `a_local_lock_comes_first_at_a_node_and_the_first_node_first` covers both ordering rules. At one node, the local lock at `x = 2` is reported, after delay 2, ahead of the trap from `x = 1`. On the time-locked `Ticker`, the initial node's trap comes before the later node's local lock. |
