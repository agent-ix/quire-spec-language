---
id: SR-1727002
title: "Gap analysis of quire-spec-language PR #1727: lower parallel controls to V2 (QSL-1186a)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@c73d934be; PR #1727 against FR-466 parallel layout and FR-466-AC-4, FR-464-AC-1/2/4, FR-375-AC-23/26, FR-432 (QSpec 48373530)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-specification/FR-466
    type: reviews
---
# Gap analysis of quire-spec-language PR #1727

## Summary

Ticket: QSL-1186 (slice a). Plan completion: not assessed.

**Covered:**

- **FR-466 layout and FR-464-AC-1/2.** `a_parallel_of_sequences_emits_its_layout_and_reads_back` covers:
  - `sc` with `quorum(1)`, `outstanding cancel`, sorted branches, named bodies and `joined` ascending, read back by IR;
  - the plain form: empty optionals and `all` with its policy only.
- **FR-464-AC-4, one level.** The branch body is declared `[Flow, run, Main, Par, Ay]`.
- **The predicate refusal.** Covered by `a_predicate_join_is_not_written_yet`.
- **FR-375-AC-23/26.** `a_parallel_clause_writes_fork_join_and_drain_classes` covers `fork`/`join Local::par` on the `parallel` node with empty binders, and the `sc` drain entry.

**FR-466-AC-4's join checks** (quorum bounds, `all` omitting a branch, unknown labels, empty lists) are S3 refusals that predate this slice. My probes confirm each one still refuses before lowering.

**Not tested in this PR:**

- **The `tso` case.** The PR body says IR reads back "tso+all", but the test has no `tso` case. Its second package is the plain form. My probe shows `memory tso` with `all` reads back.
- **The holder rule beyond one level.** No test covers a `parallel` nested in a branch body, or a `parallel` that is itself a branch body. Those are the cases where "holder's declaration followed by its own name" and "a branch is no segment" compose (FR-464-AC-4). Probes pass and read back.
- **`outstanding continue`** (probe passes).

**Not recorded.** The predicate join is deferred to the slice that writes function nodes. The PR body says it is "still unwritten", but QSL-1186's remaining list (attempt, event, send, receive, fence, await, choice, repeat, check) does not name it.

## Verdict

**The slice's ACs are met.** There are two low items: tests for the cases the probes covered, and recording the predicate join.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Add read-back tests for three cases. (a) `memory tso` with `join all`, which the PR body claims but no test runs. (b) A `parallel` nested in a branch-body `sequence`, and a `parallel` as a branch body, asserting the declarations `[…, Par, Ay, In]` and `[…, Par, In, X]` (FR-464-AC-4's holder rule across levels). (c) `outstanding continue`. Probes for all three pass and read back. | qsl-package/src/emit/tests/protocol_graph.rs; probe_1727.diff; FR-464-AC-4; FR-466 `parallel` |
| FND-002 | low | Record on QSL-1186 that the `predicate` join (with its `function` reference, plus FR-466's arity and type checks on the function) is deferred to the slice that writes function nodes. It is not in the PR's remaining-work list. | QSL-1186; FR-466 per-operator check 4 |

## Dispositions

Round 1, head 7cd3f46f6. The gates are those in SR-1727001's dispositions.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `tso_continue_and_nested_parallels_read_back` reads back each case through IR and asserts it: `memory tso` with `all`; `outstanding continue` with its policy; a `parallel` nested in a branch body's sequence, with its declarations and its branch body's target; and a `parallel` that is itself a branch body. |
| FND-002 | fixed | QSL-1186 comment 5949da6c and the PR body ("Predicate join … deferred to the function-node slice") record the deferral, with the function reference and FR-466's arity and type checks. One correction to the record: FR-466 check 4 asks for one Boolean parameter per `joined` label and a Boolean result, not "two Booleans per joined label". |
