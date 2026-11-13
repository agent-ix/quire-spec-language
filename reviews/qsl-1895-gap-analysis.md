---
id: SR-2026111306075602
title: "gap analysis of PR2697 protocol projection onto one role"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-570-AC-1..7, FR-573-AC-2/3/5, FR-574-AC-1..5/7, FR-575-AC-1/3 and the tests PR2697 tags to them; PR2697 c7587607454cf151473007cd14cd87fdd7b08286"
review_set: subset
---

## Summary

Ticket: QSL-1895 (ADR-036 slice 3). Plan completion: not assessed. Planless audit scoped to the criteria the PR tags and to the ADR-036 slice 3 row. FR-570-AC-2, AC-3, AC-6 and FR-574-AC-1 to AC-5 and AC-7 are backed. Every tagged test calls new API (`project`, `GlobalStructure`, `unsupported_construct`, `replay_projection_counterexample`), so none can pass without the code. No `/// Trace:` lines. Three tags claim more than their tests show, and the two soundness holes in SR-2026111306075601 (FND-001, FND-002) have no test, so FR-570-AC-1, AC-4 and AC-5 are backed only for flat choices.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-575-AC-1 requires every construct row other than `evaluator-unsupported` to yield its slug and node. The tagged test covers `unsupported-join` (`any`, `quorum`), `repeat`, `unsupported-control` (`check` only) and `symbolic-queue-capacity`. It does not cover `dynamic-role-population`, `compensation` (template or `compensate` control), `timed-await`, `unassigned-local-choice` (a protocol-owned choice), a `predicate` join, or `outstanding` under `join all`. Each is one branch in `unsupported_construct` and one fixture. These are real gaps for this slice; `evaluator-unsupported` is realizability (slice 4) and is not. The node check is also skipped for the symbolic-capacity case. | qsl-analyze/tests/it/protocol_projection.rs:509-581; qsl-analyze/src/protocol_analysis/profile.rs:43-96 |
| FND-002 | medium | The FR-573-AC-2 tag on `the_replay_arm_reproduces_a_counterexample_and_refuses_an_edited_one` asserts only the replay arm's `Category::Success`. AC-2 requires `refuted`, `decisive-counterexample`, execution `completed`, truth `violated`, settled only after the replay, and no test reads any of those labels (the FR-360 map is not wired in this slice). Narrow the tag to what is shown (AC-3's disagreement half), or assert the labels. The same holds for FR-573-AC-5 on `a_run_limit_fails_the_run_naming_the_limit`: it asserts `Failed(limit)` and the limit name and value, but not `resource-incomplete` or `unavailable`. | qsl-analyze/tests/it/protocol_projection.rs:481-505,677-730 |
| FND-003 | medium | FR-570-AC-1, AC-4 and AC-5 are backed only by flat protocols: no test has a choice nested in a case, or a choice whose `visible` reads facts from more than one sender. Both are where the engine is unsound (SR-2026111306075601 FND-001 and FND-002). The AC-1 graph-equality test uses one fixture with no choice at all, so the local graph of a merged or owned choice is never compared with the global structure. | qsl-analyze/tests/it/protocol_projection.rs:385-463 |

## Scope and evidence

Tests are linked by `#[trace]` tags; the focused run of `protocol_projection` is recorded in the reviewer report. The coder's list of known untested paths was weighed one by one. `dynamic-role-population`, `compensation`, `timed-await` and `unassigned-local-choice` are FND-001. `evaluator-unsupported` belongs to realizability (slice 4) and is not a gap here. An end-to-end `ReplayRequestWire` through `ReplaySource::ProtocolAnalysis` is not a gap for this unserved slice: no wire codec decodes that source and nothing produces `ReplayResult::ProtocolAnalysis` yet (see SR-2026111306075601 FND-006).

- FR-570-AC-1 (spec/functional/FR-570-project-a-global-protocol-onto-one-role.md): For a projectable protocol the local graph's events and causal order equal the global event structure restricted to the role's events through the correspondence table. (examined)
- FR-570-AC-2 (spec/functional/FR-570-project-a-global-protocol-onto-one-role.md): Every global communication projects to exactly one local send and one local receive. (examined)
- FR-570-AC-3 (spec/functional/FR-570-project-a-global-protocol-onto-one-role.md): A parallel region with join all projects to a local partial order; two receives with equal evaluated FIFO key keep their order. (examined)
- FR-570-AC-4 (spec/functional/FR-570-project-a-global-protocol-onto-one-role.md): A choice branch merges for the role only by the PA-10 rule. (examined)
- FR-570-AC-5 (spec/functional/FR-570-project-a-global-protocol-onto-one-role.md): A choice the role cannot distinguish returns a counterexample of smallest length with the PA-11 tie-break. (examined)
- FR-570-AC-6 (spec/functional/FR-570-project-a-global-protocol-onto-one-role.md): the ADR-036 worked example, four variants. (examined)
- FR-570-AC-7 (spec/functional/FR-570-project-a-global-protocol-onto-one-role.md): A request the engine cannot decide or whose search reaches max_depth returns a non-conclusive result with no graph. (examined)
- FR-573-AC-2 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): A projection counterexample settles refuted, decisive-counterexample, completed, violated, only after the counterexample replays. (examined)
- FR-573-AC-3 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): A counterexample whose replay disagrees settles inconclusive. (examined)
- FR-573-AC-5 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): A run stopped by max_states or max_transitions settles failed, resource-incomplete, unavailable. (examined)
- FR-574-AC-1..5, AC-7 (spec/functional/FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md): the projection record, its components and the assessed-subject root. (examined)
- FR-575-AC-1 (spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md): For each construct row other than evaluator-unsupported, a protocol using it yields that slug and node. (examined)
- FR-575-AC-3 (spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md): A profile word other than bounded-static is refused naming profile. (examined)

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 66bed12d8/a0a95b4aa/ea3fb99bc: the FR-575-AC-1 tests now cover `dynamic-role-population` (`Pool each ... from cells`), a `predicate` join, `compensation` (`Refund` template), `timed-await` (`Wait`) and `outstanding` under `join all`. `unassigned-local-choice` was removed as unreachable (lead decision 3). |
| FND-002 | fixed | 66bed12d8: the FR-573-AC-2 and FR-573-AC-5 tags were removed. The tests keep only FR-573-AC-3 and FR-570-AC-7, which they show. This is acceptable while the FR-360 label map is unwired (slice 5). |
| FND-003 | still-open | The nested-choice and two-sender tests landed, but nothing covers a nested choice whose alternatives must each be compared against one equal action, where the engine gives a false `Refuted` (SR-2026111306075601 FND-010). The AC-1 graph-equality test still uses only the choice-free `parallel_flow` fixture. Add both. |
| FND-003 | fixed | bcb6cc197: `nested_alternatives_each_merge_with_the_other_branch` (FR-570-AC-8) covers nested alternatives against one equal action, and `local_graphs_with_a_choice_equal_the_global_structure` runs the AC-1 graph-equality check on receive-distinguished and merged choice fixtures for every role. |
