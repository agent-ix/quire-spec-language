---
id: SR-13323
title: "integrity review of STD-214 FR-169-AC-10/AC-11 and TC-644 (PR #1048)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md, spec/test-cases/TC-644-state-graph-certificates-that-do-not-hold-are-rejected.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-644
    type: reviews
---
# SR-13323: integrity review of STD-214 (PR #1048)

## Summary

Ticket: STD-214. This integrity pass covers ids, links and coverage in the
diff `origin/main...origin/std-214-spec`.

`quire validate --scope .` passes on FR-169 and TC-644 (exit 0, quire
0.36.2, engine 0.50.2). The new FR-169-AC-11 names `Test (TC-644)`, and
TC-644 already has a `verifies` edge to FR-169. TC-644's `Scope`, procedure
step 5, expected result and trace-tag instruction all add AC-11. No existing
AC is renumbered: AC-1 to AC-10 keep their ids, and AC-11 is appended.
FR-169-AC-11 is not on origin/main, and none of the 27 other open PRs adds
it. PR #1046 only tags `FR-169-AC-10`.

`quire matrix` on the branch shows FR-169-AC-9, AC-10 and AC-11 all
`untagged`. That is expected here, because the checker lands with
QSL-538 / PR #1046.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-169-AC-11 has no test on origin/main or in PR #1046, the PR that implements the state-graph checker. The ticket says the coder already rejects `BadState` (`qsl-replay/src/certificate/state_graph.rs:442` on #1046), but no test hits that branch or carries `#[trace("TC-644", "FR-169-AC-11")]`. Unless #1046 adds a test for TC-644 step 5 before it merges, AC-11 stays untagged in the matrix. | spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:227; spec/test-cases/TC-644-state-graph-certificates-that-do-not-hold-are-rejected.md:32-38,50-51 |

## Verdict

One low finding. It belongs to the QSL-538 implementation, not to this
spec PR. The link structure is otherwise intact.

Examined and found clean:

- AC-11's verification cell and TC-644's `verifies` edge;
- the TC-644 scope line and trace-tag instruction;
- AC id continuity, with no renumbering;
- the absence of any id collision on main or in open PRs.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Belongs to QSL-538 / PR #1046, not this spec PR. At #1046 head 07fd24df7, its only state-graph `BadState` test (`an_undetermined_contract_at_a_reached_state_is_bad`, qsl-analyze/tests/it/state_graph_certificate.rs:558-584) uses a `Fork` fixture and is tagged `#[trace("TC-644")]` alone. AC-11's guarded §7.1 vector and the `FR-169-AC-11` tag are still to be added there before #1046 merges. |
